# Socle 40 — Restauration

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée **avant** les lots suivants, **après** un code Rust existant (migrations 017, 034, 036, 074, 079 ; `domain-eleve/src/{repas,menu_cantine}.rs`) et un module Next.js en production (cantine et passages de l'add-on, D-035). Aucun code écrit ni modifié. Aucun accès à une base, à un secret ni à un environnement de production. Le code Next.js a été **lu seulement**, sur une copie de `origin/main` extraite par `git archive` dans le dossier temporaire de la session (rien exécuté, rien construit, aucun `.env` lu).
**Branche :** `ent-socle-40-41-conception`, partie de `origin/ent-mc0hoe` (`7467f2d4`).
**Passe personas :** `docs/socles/PERSONAS_40.md` (même branche). Ses ajouts sont faits **ici**, marqués **« ajout personas »**, et regroupés au **§30**. **En cas de contradiction, le §30 l'emporte** ; rien n'a été supprimé.
**Socle jumeau :** `docs/socles/41_INTERNAT.md` (dîner et petit-déjeuner des internes, effectif du soir).
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` D-017, D-021, D-023, D-026, D-029, D-031, **D-034** (doublon 2027-2028 : finances dans Charlemagne, santé désactivée sans HDS), **D-035**, **D-036** ; `docs/PRA_SAUVEGARDES.md` §18.4 ; `docs/TRANSITION_T0_INVENTAIRE.md` (M14) ; `docs/SOCLES.md` §40 ; `docs/SOCLE2_DOMAINES.md` D11 (RES-01 à RES-09, repris ici) ; `docs/MATRICE_CHARLEMAGNE.md` §3.12 (SAN-01 à SAN-04, SAN-12), FIN-12, FIN-17, FIN-18, T-23, T-27 ; conceptions `02_ELEVE.md` (régime), `03_INSCRIPTIONS.md` (régime demandé), `04_ADMINISTRATIF.md` (payeurs par catégorie « restauration »), `05_STRUCTURE.md` (fermetures R-S1, services, lieux), `06_PERSONNELS.md` (commensaux, postes `restauration`), `08_NOTIFICATIONS.md` (`restauration.solde_bas`), `09_INTEROP.md` (FX-74 Op@le / CRDC), `24_FINANCES.md` §11 (consommations, S-160), `25_PAIEMENTS.md` (porte-monnaie PAY-206), `34_SANTE.md` (consignes, audience « restauration », CS-34, CS-35, CS-56), `42_SORTIES_VOYAGES.md` §14 (commande cuisine, paniers, dispense) et `IA_COEUR.md`.

**Conventions.** **[F]** fait vérifié dans le code (Next.js : chemins relatifs à `app/` de `origin/main` ; Rust : depuis `ent-rust/`). **[V]** lu sur la source officielle (§5, avec URL et date). **[V-copie]** lu sur une copie ou un extrait. **[NV]** non vérifié. **[H]** hypothèse de terrain. **[R]** recommandation, non décidée.

| Élément | Numéros | Remarque |
|---|---|---|
| Actions existantes de Next.js | **NC-01**… | §2.2 |
| Défauts de Next.js | **DC-01**… | §2.3 |
| Défauts du code Rust existant | **RR-01**… | §2.5 |
| Actions cibles | **RES-100**… | RES-01 à RES-09 pris par `SOCLE2_DOMAINES.md` D11, repris et renvoyés |
| Règles | **R-RS1**… | |
| Cas limites | **CRS-01**… | |
| Questions | **Q-RS1**… | |
| Réglages signalés | **S-4000 à S-4049** | |
| Manques (personas) | **M-4001**… | |

**Plage de migrations imposée : 2800–2819** (inscrite dans `docs/socles/MIGRATIONS.md`). Le lot 0 (R0) part en **2800**.

Sommaire : §0 En bref · §1 Thèse · §2 Existant · §3 Acteurs · §4 L'année · §5 Réglementaire · §6 Frontières · §7 Concepts · §8 Droit au repas · §9 Réservations · §10 Passages · §11 Menus, allergènes, qualité · §12 Santé et régimes · §13 Prévision et production · §14 Commensaux · §15 Paniers et repas hors self · §16 Argent · §17 Gaspillage et indicateurs · §18 Cas limites · §19 IA · §20 Actions · §21 Règles · §22 Droits et minimisation · §23 Événements · §24 Écrans et mobile · §25 Modèle de données · §26 Lots · §27 Reprise et bascule · §28 Tests · §29 Réglages, questions, décisions · §30 Ajouts personas · §31 Faits, hypothèses, contradictions.

---

## 0. En bref

1. **La restauration répond chaque jour à trois questions** : *combien de repas préparer demain* (cuisine), *ce jeune a-t-il droit à ce repas et peut-il le manger sans danger* (au passage), *combien de repas facturer à qui* (comptabilité). Aujourd'hui, les trois réponses viennent de sources différentes et se contredisent (la prévision exclut les élèves en voyage, la facture non).
2. **Le module Next.js est petit et partiel** [F] : saisie **manuelle** des passages au self (pas de badge réel), prévision du jour (attendu / pris / imprévu / en sortie), grille hebdomadaire de 5 jours en JSON, facturation forfait ou réel en factures **brouillon** par foyer, alertes allergies issues d'extraits santé **en texte libre**, commande cuisine et paniers des voyages. **21 actions** (NC-01 à NC-21), **toutes reprises** sauf ce qui relève du 42 ou de l'infirmerie (§2.2). **Rien pour les familles** : ni menus, ni réservation, ni annulation.
3. **Ses défauts sont surtout financiers et de santé** [F] : un surveillant peut changer le mode de facturation et créer des factures (DC-01) ; deux clics = deux factures par foyer (DC-08) ; absences, sorties, fermetures et jours fériés **facturés** en forfait (DC-07) ; bornes de jour en UTC (DC-14) ; aucun passage corrigeable (DC-16) ; le libellé santé libre (parfois un diagnostic) est montré à l'accueil (DC-03). **22 défauts** (DC-01 à DC-22).
4. **Le code Rust est plus riche** (grilles, dispenses, badge, volumes, menus, bourses, export CSV cuisine) **mais a des failles à fermer d'abord (lot R0)** [F] : un passage peut être créé **à n'importe quelle date** et il est facturé (RR-01) ; une famille peut annuler un repas **passé** (RR-02) ; un badge peut pointer le **mauvais élève** (RR-03) ; **aucune alerte de consigne alimentaire** au passage alors que le circuit existe au 34 (RR-04) ; menus sans allergènes codés (RR-05) ; grilles qui se chevauchent et ne suivent pas le régime (RR-06) ; motif de dispense libre (santé possible) dans les événements (RR-09). **12 défauts** (RR-01 à RR-12).
5. **La valeur ajoutée est le branchement** (§6) : le **droit au repas** se déduit du **régime daté** (2) et de l'inscription au service ; les **non-repas** se déduisent **automatiquement** des faits des autres socles (absence annoncée 12, internat 41, voyage 42, stage 39, exclusion 13, fermeture 5) — **une seule règle** sert à la prévision **et** à la facturation ; les consignes alimentaires viennent du 34 **sans diagnostic** ; les payeurs du 4 ; la facture du 24 (en **mode suivi** pendant le doublon).
6. **Le réglementaire** (§5) : compétence du département (collèges) et de la région (lycées) [V] ; **tarifs plafonnés au coût par usager** dans le public (R. 531-52, R. 531-53) [V] ; dans le privé, service facultatif de l'organisme de gestion [NV] ; **allergènes** : information **écrite** des 14 allergènes pour les repas servis (C. consom. R. 412-12 à R. 412-16) [V] ; **menu végétarien hebdomadaire** obligatoire (C. rural L. 230-5-6) [V] ; **information annuelle** des usagers sur la part de produits durables (L. 230-5-3) [V] ; registre de composition des repas sur **trois mois** (D. 230-26) [V] ; **PAI** : le panier-repas n'est pas la première solution, seules les conséquences utiles sont connues, pas le diagnostic (circulaire du 10 février 2021) [V] ; **pas de droit à un repas de substitution** confessionnel (CE 11 décembre 2020) [V] ; mention « sans porc » admise, pas « halal » ni « casher » (ancienne NS-058, indicative) [V] ; **biométrie déconseillée**, badge par défaut, alternative sans pénalité (CNIL 2019 et 2021) [V] ; lutte contre le gaspillage avec **diagnostic** (C. env. L. 541-15-3) [V].
7. **Ce que ScolIA apporte de plus [H]** :
   - **une seule règle du droit au repas** (forfait, réservation, non-repas automatiques), lue par la cuisine, le surveillant du self **et** la facture ;
   - **réserver ou annuler en deux gestes** dans l'appli, avec l'heure limite lisible (« jusqu'à 9 h 30 demain »), et la **liste des repas annulés automatiquement** (« mardi : sortie au musée ») ;
   - au self, en **une seconde** : photo, droit du jour, **consigne alimentaire codée** (« éviction arachide — trousse : infirmerie »), sans jamais montrer une maladie ni un impayé ;
   - **menus** lisibles par les familles avec les 14 allergènes et un **filtre personnel côté appareil** (« masquer les plats avec lait ») qui ne stocke rien ;
   - **prévision à J-7, J-1 et le jour même à 10 h**, par service et par choix de plat, avec les adultes et les paniers ;
   - **mode suivi** pendant le doublon : ScolIA calcule ce qu'il aurait facturé, sans facturer, et compare avec Charlemagne ;
   - **gaspillage mesuré** (pesées par service, écart prévu / servi) et information **EGalim** prête à publier ;
   - **IA qui prépare** (prévision expliquée, brouillon du menu de la semaine vérifié contre les fréquences, synthèse du gaspillage), **ne décide rien**, **jamais sur la santé**.
8. **Le plan** :

   | Élément | Volume | Numéros |
   |---|---|---|
   | Actions | 124 | RES-100 à RES-249, numérotées par blocs avec des numéros libres en réserve (dont 15 ajout personas, RES-235 à RES-249) |
   | Cas limites | 56 | CRS-01 à CRS-56 (dont 12 ajout personas) |
   | Règles | 38 | R-RS1 à R-RS38 (dont 6 ajout personas) |
   | Réglages | 20 | S-4000 à S-4019 |
   | Lots | 14 | R0 à R13 |

   - **Lot R0** (correctifs urgents Rust) : dates de passage bornées, délai de prévenance, badge unique, consignes au passage, motif codé, FK composites, annulation de passage, grilles sans chevauchement (§26.1).
   - **Parcours minimal de la rentrée 2027** : R0, R1, R2, R3, R4, R5 (menus et allergènes), R6 (mode suivi), R8 (paniers), R10 (mobile), R11 (reprise).
9. **Décisions appliquées par défaut (D-031)** : 22 (§29.2). **Soumis à Florian** (§29.3) : qui tient aujourd'hui les passages et les badges (Charlemagne, prestataire, Next.js) et donc le régime de la rentrée 2027 (usage réel ou parallèle) ; le **prestataire** de restauration et ce qu'il reçoit ; les durées de conservation des passages (DPO) ; l'achat éventuel de **lecteurs** de badge ; l'activation du porte-monnaie prépayé (contrat de paiement).

---

## 1. Thèse : un repas est un droit daté, un passage est un fait, un non-repas a une cause

1. **Le droit au repas est calculé, jamais recopié.** Il vient du **régime daté** de l'élève (2), de son **inscription au service** (jours choisis, forfait ou réservation) et des **réservations** du jour ; il se lit « Léa a droit au déjeuner du mardi 12 ».
2. **Un non-repas a une cause.** Quand l'élève ne mangera pas, ScolIA le sait par un **fait d'un autre socle** (absence annoncée, voyage, stage, internat absent, exclusion, fermeture, panier fourni par la famille) ou par une **annulation** de la famille dans le délai. La cause est **codée** (jamais un motif libre, jamais une maladie) ; elle sert la cuisine (ne pas préparer), la facture (remise ou non selon le règlement) et la famille (« pourquoi ce repas n'est pas compté »).
3. **Un passage est un fait.** Il est horodaté, attribué à un service, rattaché à un élève (ou un commensal), corrigeable par un fait motivé, jamais effacé.
4. **La facture lit la même règle que la cuisine.** Prévision et facture partent du même calcul (droit − non-repas + passages hors droit) ; il n'y a plus de « prévision qui exclut les voyages et facture qui les compte » (DC-07).
5. **La santé passe par des consignes, pas par des fiches.** La restauration voit « éviction : arachide ; trousse : infirmerie » publié par l'infirmière (34), pour l'audience « restauration » ; elle ne voit **jamais** « allergie », un PAI ou un traitement.

---

## 2. Évaluation franche de l'existant

### 2.1 Où et comment Next.js le fait [F]
- **Environ 3 900 lignes** (plus que les 1 900 de T0) : pages `app/(admin)/passages/` (hub, portail, self, prévision, facturation), route multiplexée `app/api/passages/route.ts` (GET : recherche, alertes, prévision, facturation ; POST : mode de facturation, brouillons, passage), bibliothèques `lib/passages-*.ts`, grille dans `eleve_scolarite.grille_repas` (JSONB) avec `demi_pension` et `repas_par_semaine`, réglage `tenant_setting_attr` section `cantine` / `facturation_mode`.
- Liens : `lib/internat-repas-soir-db.ts` (dîner des internes), `api/stages/repas-absences` (jours de stage), voyages (`api/travels/send-cuisine`, `send-panier-repas-list`), extraits santé `sante_extrait` de portée « cantine ».
- Module ouvert aux rôles : directions, administratif, accueil, CPE, surveillant ; **aucun parent, élève ni professeur**.
- La colonne `passage.source` admet `badge | app | manuel`, mais **seul `manuel` est écrit** : il n'y a ni badge, ni borne, ni lecteur.

### 2.2 Ce que fait Next.js : actions existantes, toutes reprises [F]
| NC | Action | Où | Reprise |
|---|---|---|---|
| NC-01 | Saisir un passage au portail (entrée / sortie) d'un élève ou d'un invité nommé | `components/passages/PassagesClient.tsx`, `lib/passages-db.ts` | **socle 14** (portail) ; le 40 ne garde que le self |
| NC-02 | Saisir un « repas pris » au self | idem | RES-160, RES-163 |
| NC-03 | Rechercher un élève actif (2 caractères, 20 résultats) | `searchElevesForPassage` | RES-163 |
| NC-04 | Afficher les alertes « cantine » (extraits santé) à la saisie | `getAlertesCantinePourEleve` | RES-165 (consignes codées du 34) |
| NC-05 | Lister les passages du jour par lieu | | RES-168 |
| NC-06 | **Prévision** du jour, midi et soir : attendu (grille sinon régime) contre pris ; statuts attendu-pris / attendu-manquant / imprévu / en sortie (avec tests) | `lib/passages-prevision-shared.ts` | RES-180 à RES-186 |
| NC-07 | Retirer de la prévision les élèves en voyage | `listEleveIdsEnSortieOnDate` | R-RS6 (cause « sortie scolaire ») |
| NC-08 | Choisir le mode de facturation (forfait / réel) | réglage `cantine.facturation_mode` | RES-215 (droit comptable seulement) |
| NC-09 | Calculer les lignes d'une période (forfait ou réel) | `lib/passages-facturation-db.ts` | RES-210 |
| NC-10 | Créer des factures **brouillon** par foyer, élèves sans foyer comptés | idem | RES-211 → 24 |
| NC-11 | Créer automatiquement le tarif « repas » à 5 € s'il manque | `ensureRepasTarif` | **non repris** (tarif au 24, jamais créé par une lecture, DC-09) |
| NC-12 | Éditer la grille lundi–vendredi (midi, soir, étude, garderie, « sort seul ») dans le dossier | `api/eleves/[id]/dossier` `update_grille_repas` | RES-120 (midi, soir) ; étude, garderie → 43 ; « sort seul » → 14 |
| NC-13 | Même édition par l'assistant IA (préréglages 5 jours, n jours, internat, vide) | `brain-ai/tools/handlers/eleve-grille-repas.ts` | RES-247 (IA, N2) |
| NC-14 | Déduire la grille du régime pour l'affichage | `lib/eleve-dossier-synthese.ts` | R-RS2 |
| NC-15 | Repas du soir des internes (droit, pris après 16 h, sorties d'internat) | `lib/internat-repas-soir-db.ts` | RES-186 (avec 41) |
| NC-16 | Gérer les extraits santé de portée « cantine » | `api/sante/extraits` | **34** (consignes codées) |
| NC-17 | Voyage : commande cuisine (pique-niques, sans porc, végétarien, self adultes / élèves, boissons, livraison) avec PDF | `lib/travels-cuisine-*.ts` | **42** (VOY-180) ; le 40 **reçoit** (RES-195) |
| NC-18 | Voyage : « qui mange » nominatif, contrôle commandé = attribué, CSV et PDF | `lib/travels-panier-repas-send.ts` | **42** (VOY-181, VOY-182) ; le 40 reçoit (RES-196) |
| NC-19 | Stages : jours d'absence de stage pour la restauration | `api/stages/repas-absences` | R-RS6 (cause « stage », lu au 39) |
| NC-20 | « Dernier passage au portail = sortie » (où est X) | `lib/occupancy/passage-read.ts` | **14** |
| NC-21 | Passages de l'infirmerie (`source: infirmerie`) | | **34** (hors périmètre) |

**Absent de Next.js** [F] : menus, allergènes, réservation et annulation par la famille, badge et lecteur, correction de passage, export des passages, porte-monnaie, quotient, repas des personnels, prévision au-delà du jour, gaspillage.

### 2.3 Défauts trouvés dans Next.js (signalés à Florian ; corrigés dans ScolIA)
| DC | Gravité | Défaut [F] | Où | Correctif |
|---|---|---|---|---|
| DC-01 | **critique** | Une seule garde `requireModule("passages")` : un surveillant ou l'accueil change le mode de facturation et crée des factures | `api/passages/route.ts:15-16, 70-71, 88, 97` | droits distincts (§22.1) |
| DC-02 | haute | `passage.eleve_id` sans FK ; cloisonnement seulement applicatif | `lib/passages-db.ts:106-153` | E2, RLS |
| DC-03 | **critique** | Libellé santé **libre** (allergies, parfois diagnostics) montré à l'accueil et aux surveillants, sans journal de lecture | `GET /api/passages?alertes=cantine` | consignes codées du 34, audience « restauration » (R-RS15) |
| DC-04 | moyenne | Recherche sur tout l'établissement pour tout rôle du module | `api/passages` | périmètre du rôle (§22) |
| DC-05 | moyenne | Droits faits à la main sur la route des stages | `api/stages/repas-absences/route.ts:24-40` | catalogue D-017 |
| DC-06 | haute | Données de repas des voyages (INE, régimes « sans porc / végétarien ») envoyées en clair à une liste libre | voyages | 42 VD-06, VD-16 ; R-RS18 |
| DC-07 | **critique** | **La prévision exclut les élèves en voyage, la facture au forfait les compte** ; absences, fermetures, jours fériés facturés | `lib/passages-facturation-db.ts` | une seule règle (R-RS5, R-RS6) |
| DC-08 | **critique** | Numéro de facture `Date.now()`, aucune idempotence : deux clics = deux factures par foyer | `lib/passages-facturation-db.ts:320` | clé de source du 24 (R-RS30) |
| DC-09 | haute | Une **lecture** crée un tarif à 5 € codé en dur | `ensureRepasTarif` | tarifs au 24, jamais par une lecture |
| DC-10 | haute | `ALTER TABLE` exécuté depuis une route (DDL en production) | `lib/eleve-scolarite-schema.ts` | migrations seulement |
| DC-11 | haute | Plusieurs scolarités « en cours » dupliquent prévision et facture | jointures `eleve_scolarite` | `eleve::scolarite_a` (2) |
| DC-12 | haute | Mode réel : tout passage compte, re-passages compris ; pas de midi / soir | `passages-facturation-db.ts` | un passage facturable par élève, service, jour (R-RS10) |
| DC-13 | haute | Éditer la grille **écrase le régime** (`demi_pension = midi > 0`) | `dossier/route.ts:1277-1279` | régime au 2, inscription au 40 (R-RS2) |
| DC-14 | haute | Bornes de jour en **UTC** ; seuil midi / soir codé à 16 h | `passages-db.ts:61-62` et suivants | Europe/Paris, horaires du service (R-RS4) |
| DC-15 | moyenne | Horodatage toujours « maintenant » : un oubli ne se saisit pas | `createPassage` | saisie a posteriori motivée (RES-167) |
| DC-16 | haute | Aucune modification ni annulation de passage ; aucun auteur | `passages-db.ts` | faits correctifs (R-RS11) |
| DC-17 | haute | Grille sans historique : facturation rétroactive impossible | `grille_repas` | inscriptions datées (R-RS3) |
| DC-18 | moyenne | Grille en JSONB non typé ; régime en texte classé par expressions régulières | | D-036 ; régime codé |
| DC-19 | faible | Étude, garderie, « sort seul » stockés, jamais utilisés | | 43, 14 |
| DC-20 | moyenne | Prévision sans absences déclarées, infirmerie, exclusions, calendrier | | R-RS6 |
| DC-21 | moyenne | Pas de repas des personnels ; `annee_scolaire_id` jamais renseigné | | §14 |
| DC-22 | faible | Pas de TODO : les limites sont des absences de fonction | | — |

### 2.4 Ce qu'il ne faut pas perdre (D-035)
1. Le principe **« droit = grille ou régime ; pris = passage au self »** et les **quatre statuts** de la prévision, avec leurs tests.
2. La priorité **inscription explicite > régime déduit**.
3. La grille **jour par jour** (étendue aux 7 jours et datée, sans casser l'usage).
4. Les **alertes à la saisie** au self, limitées à une portée « cantine » (le principe est bon ; on corrige le contenu).
5. Le croisement avec les **voyages** et l'**internat**.
6. Les **brouillons par foyer** avec le décompte des élèves sans foyer ; **jamais de suppression**.
7. Le mode **forfait / réel** au choix de l'établissement.
8. Le flux voyage complet (commande cuisine, PDF, « qui mange », contrôle commandé = attribué) — porté par le 42, reçu par le 40.
9. L'**audit** et l'événement sur la modification de la grille.

### 2.5 Code Rust existant [F]
| Élément | Où | État |
|---|---|---|
| Services de repas | `017` `ent_repas_activite` (code, libellé, `kind` midi / soir / étude / garderie / autre, actif) | `created_at` seul ; pas de `UNIQUE (etablissement_id, id)` |
| Grilles | `017` `ent_eleve_grille_repas` (7 booléens lundi–dimanche, dates) | FK élève composite (240) ; pas d'exclusion de chevauchement |
| Bourses | `017` `ent_bourse_type`, `ent_eleve_bourse` | relèvent du 26 |
| Dispenses | `034` `ent_eleve_dispense_repas` (annulation douce, **motif libre**) | marquée « santé » au registre (`registre_donnees.rs:314`) |
| Passages | `036` `ent_repas_passage` (badge lu **en clair**, `annule` sans route, `actor_user_id TEXT`) | **sans FK élève** |
| Menus | `074` `ent_menu_cantine` (un par jour, allergènes en **notes libres**) | écrasé en place |
| Tarifs de consommation | `079` `ent_tarif_conso` (centimes) | au 24 |
| Domaine | `domain-eleve/src/repas.rs` (masque des jours, `grille_couvre_jour`, `compter_jours_facturables`, volumes cuisine, CSV ; 5 tests), `menu_cantine.rs` (1 test) | réutilisable |
| Routes | `GET/POST /eleves/{id}/repas/dispenses`, `…/annuler`, `…/grilles`, `GET …/volume`, `GET …/passages`, `GET/POST /repas/activites`, `GET /repas/volumes` (+ export CSV), `GET /repas/passages`, `POST /repas/passages/scan`, `GET/POST /facturation/cantine/badge`, `GET/POST /menus-cantine` | droits `repas.gerer` (Admin, Direction, Administratif, Compta), `repas.scanner` (+ CPE) |
| Événements | `repas.activite_created`, `eleve.grille_repas_posed` / `_close`, `eleve.dispense_repas_posed` / `_annulee`, `eleve.repas_passage`, `repas.menu` ; réaction du 24 à `eleve.regime_changed` ; la sortie de l'élève ferme les grilles | |
| Santé | `211` `ent_contrainte_publiee` (catalogue fermé de 17 codes, audiences dont `restauration`) ; `sante_acc.rs:957-1000` lisible avec `repas.gerer` / `repas.scanner` | **non branché** sur le passage |
| Interface | JavaScript `ui.rs` (`/app/cantine`) — F-001 ; aucun écran Leptos | |

| RR | Gravité | Défaut [F] | Où | Lot |
|---|---|---|---|---|
| RR-01 | **critique** | Jour de passage **libre** (passé ou futur) accepté et **facturé** ; pas de contrôle du calendrier | `routes.rs:11116-11140`, `service.rs:10370-10420`, `domain-eleve/src/facturation.rs:144` | R0 |
| RR-02 | **critique** | Dispense posée ou annulée par la famille **pour n'importe quel jour, passé compris**, sans heure limite ; motif sans borne | `repas.rs:381-401`, `service.rs:10141-10190` | R0 |
| RR-03 | haute | Badge dupliqué : `get_by_badge` en `LIMIT 1` sans ordre ; badges « contestés » partagés ; aucun contrôle du statut ni de la présence au passage | `eleve_repo.rs:57-79`, migration 281 | R0 |
| RR-04 | haute | **Aucune consigne alimentaire** au passage ; volumes sans régimes | `service.rs:10370`, `repas.rs:526` | R0 |
| RR-05 | haute | Menus sans allergènes codés ; écrasés en place ; identifiant d'événement différent de l'identifiant en base | `074`, `menu_cantine_repo.rs:22-30`, `service.rs:14268` | R0 (identifiant), R5 |
| RR-06 | haute | Grilles qui se chevauchent ; aucune clôture au changement de régime ; un élève passé externe reste facturable | `repas_repo.rs:83-100`, `repas.rs:227-246`, `service.rs:4219` | R0 |
| RR-07 | haute | Passage ni annulable ni corrigeable ; **numéro de badge brut** dans la table et la charge de l'événement | `036`, `service.rs:10428-10431` | R0 |
| RR-08 | haute | `ent_repas_passage` sans FK élève ; `activite_id` en FK simple ; `actor_user_id TEXT` | `036` | R0 |
| RR-09 | haute | **Motif libre de dispense** (santé possible) mis dans l'événement et lu par tout rôle ayant la section « repas » (professeur, infirmerie, psychologue) | `service.rs:10170-10186, 4690-4730` | R0 |
| RR-10 | faible | Liste des services lisible de tous (donnée non personnelle) | `service.rs:13304-13318` | acceptable |
| RR-11 | moyenne | Bourses sans montant ni historique de décision | `017` | socle 26 |
| RR-12 | moyenne | Pas de réservation, de choix de plat, de panier ; pas de lien PAI → restauration | | R2, R8 |

### 2.6 Verdict
On garde le **domaine Rust** (masques, comptage, volumes, CSV) et ses tables, **corrigées** (R0) puis **étendues** par migrations additives ; on reprend de Next.js la **prévision à quatre statuts**, le **croisement voyages / internat**, le **mode forfait / réel** et les **brouillons par foyer** (désormais émis par le 24) ; on ne reprend **ni** les extraits santé libres, **ni** la grille JSON, **ni** la création de tarif à la lecture, **ni** les passages du portail (14).

---

## 3. Acteurs, besoins réels, irritants

Abréviations : RST (responsable de restauration, chef de cuisine, gérant du prestataire), CUI (équipe de cuisine), SLF (surveillant du self, AED), CPT (comptabilité, intendance, régisseur), ADF (secrétariat), DIR (direction), CPE, INF (infirmière), PRF (professeur, organisateur de sortie), PAR (titulaire), PAY (payeur), ELV (élève), MAJ (majeur), COM (commensal : personnel, stagiaire, invité), PRE (prestataire sans compte ou avec compte restreint), SYS.

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait | Fréquence |
|---|---|---|---|---|---|
| **Responsable de restauration / chef** | Commande, produit, sert ; fait les menus ; gère les régimes et les PAI ; rend des comptes | Effectif découvert le matin ; sorties annoncées la veille ; listes d'allergies papier ; ressaisir les menus dans trois outils | **Un enfant allergique servi d'un plat interdit** ; gaspiller 40 repas ; un contrôle sanitaire | **Effectif fiable à J-1 et à 10 h**, par choix de plat ; liste du jour des **consignes** avec photos ; menus saisis **une fois** (affichage, appli, prestataire, registre) ; pesées faciles | quotidienne |
| **Équipe de cuisine** | Prépare, sert | Variantes non comptées | Erreur de plat adapté | Étiquettes « plat adapté » par élève **sans motif** | quotidienne |
| **Surveillant du self** (AED) | Contrôle l'entrée, compte, gère la file | Élèves sans badge, badges prêtés, files | Laisser passer un élève sans droit, ou refuser un élève qui y a droit ; rater une consigne | **Un signal clair en une seconde** (vert / orange / rouge) ; photo ; consigne visible ; « sans badge » réglé sans ralentir | quotidienne |
| **Comptabilité, intendance, régisseur** | Facture forfait ou réel, remises d'ordre, encaissements, bourses, fonds social | Exports Excel recopiés ; réclamations « mon fils n'a pas mangé ce jour-là » ; double facture | **Facturer un repas non dû** ; rater une recette ; un contrôle | Volume facturable **expliqué ligne par ligne** ; non-repas déjà déduits ; **mode suivi** pour comparer avec Charlemagne | mensuelle |
| **Direction** | Fixe le règlement, arbitre impayés et exclusions, répond aux plaintes | Plaintes allergies, menus, prix | **Un accident alimentaire** ; une exclusion de cantine contestée | Indicateurs (fréquentation, gaspillage, EGalim) ; aucune exclusion automatique | mensuelle |
| **Secrétariat** | Inscription au service à la rentrée, changements de régime, cartes | Formulaires papier « 4 jours / 5 jours » ; cartes perdues | Un élève inscrit nulle part | **Choix des jours dans l'inscription** (3) ; cartes rééditées en un geste | rentrée, puis hebdomadaire |
| **CPE / vie scolaire** | Absences, exclusions, élèves qui sautent le repas | Ne pas savoir qui n'a pas mangé | Un élève qui ne mange plus (signal de mal-être) | Signal « n'a pas déjeuné depuis 5 jours **alors qu'il est demi-pensionnaire et présent** » vers le CPE (sans le diffuser) | hebdomadaire |
| **Infirmière** | PAI, consignes, paniers, trousses | Recopier les listes pour la cuisine | Une consigne non vue | **Publier une fois** (34), audience « restauration » ; voir que la consigne est **affichée** au self | rentrée, puis au fil de l'eau |
| **Professeur organisateur de sortie** | Commande les paniers (42) | Annuler les repas de chaque élève | Que les élèves soient facturés | Les repas du jour **retirés seuls** (R-RS6) | par sortie |
| **Titulaires, payeurs** | Inscrivent, réservent, annulent, paient | Ne pas savoir s'il est trop tard pour annuler ; ne pas comprendre la facture ; pas de menu | **Un repas dangereux pour son enfant** ; payer un repas non pris ; l'humiliation d'un enfant à cause d'une dette | **Menus** avec allergènes ; **annuler avant l'heure limite** ; voir les repas **pris** et **comptés** ; comprendre chaque ligne | hebdomadaire |
| **Parents séparés** | Chacun paie parfois une part | Recevoir la facture de l'autre | Que l'autre voie ses coordonnées | Payeur par catégorie (4) ; vue propre à chacun | mensuelle |
| **Élève** | Mange, passe au self, choisit un plat | Oubli de badge ; file ; ne pas savoir le menu | **Être refusé devant les autres** ; être stigmatisé (panier, régime) | **QR dans l'appli** ; menu du jour ; choix du plat la veille ; jamais d'humiliation publique | quotidienne |
| **Élève majeur** | Idem, gère lui-même | | | Réserve et annule lui-même | quotidienne |
| **Commensal** (personnel, stagiaire, intervenant) | Mange au self | Tickets papier | | Ticket dans l'appli, facture ou retenue claire | quotidienne |
| **Prestataire de restauration** (concession) | Produit pour l'établissement, facture | Effectifs tardifs, menus à valider | Pénalités, contestations de facture | **Effectifs transmis** à heure fixe ; comptage servi partagé ; menus échangés | quotidienne |
| **Collectivité** (public) | Fixe les tarifs, règlement du service | Remontées hétérogènes | | État des droits constatés lisible (Op@le, FX-74) | trimestrielle |
| **Assistant IA** | Prévision expliquée, brouillon de menu, synthèse | | | Aucune donnée de santé ; propositions seulement | — |

---

## 4. L'année de la restauration vue du terrain [H, à confirmer avec Florian]
| Période | Ce qui se passe | Ce que ScolIA fait |
|---|---|---|
| **Avril – juillet** | Inscriptions et réinscriptions : régime demandé, jours | Le 3 recueille le régime et les jours ; le 40 prépare les **inscriptions au service** à la rentrée |
| **Fin août** | Cartes et badges, PAI de rentrée, règlement de la restauration, tarifs votés ou arrêtés | Badges générés (ou QR dans l'appli) ; consignes publiées par l'infirmière ; tarifs au 24 |
| **Septembre** | Ajustements (4 → 5 jours), premières absences, premières sorties | Changements datés ; non-repas automatiques |
| **Chaque jour** | Effectif à 10 h, service, comptage, gaspillage | Prévision, passages, consignes, pesées |
| **Chaque semaine** | Menus de la semaine suivante publiés (souvent le jeudi), commande | Menu vérifié (végétarien, allergènes), publié aux familles et au prestataire |
| **Chaque mois** | Facture au réel ou relevé, remises | Volume facturable au 24 (mode suivi pendant le doublon) |
| **Trimestre** | Bourses, fonds social, remises d'ordre | Quantités au 24 / 26 |
| **Juin – juillet** | Bilan, information EGalim annuelle, diagnostic gaspillage | Indicateurs et texte d'information prêts |

---

## 5. Ce qui est réglementaire (recherche du 2026-11-11)

**Limite d'accès** : education.gouv.fr (HTML), DGCCRF, éduscol (HTML) ont renvoyé 403 ou 503 ; Légifrance et le PDF du BO ont été lus pour l'essentiel.

### 5.1 Compétence, tarifs, service
- **Collèges** : le département assure « l'accueil, la restauration, l'hébergement » (C. éduc. **L. 213-2**, version du 4 mars 2022) [V, https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000045293747, 2026-11-11]. **Lycées** : la région (C. éduc. **L. 214-6**) [V, https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000038902139].
- Service **facultatif** (CE 24 juin 2019, n° 409659, Fondettes) [V, https://www.legifrance.gouv.fr/ceta/id/CETATEXT000038679149/] ; le droit à la cantine (L. 131-13) ne vise que le primaire [V-copie].
- **Tarifs** fixés par la collectivité ; **pas supérieurs au coût par usager**, modulation comprise (C. éduc. **R. 531-52, R. 531-53**, décret 2009-553) [V, https://www.legifrance.gouv.fr/codes/id/LEGISCTA000020743087] ; modulation selon les revenus admise (CE 23 octobre 2009, Oullins) [V-copie].
- **EPLE** : restauration et hébergement = « services spéciaux » du budget (R. 421-58) ; remise gracieuse de créances (R. 421-69) [V] ; **remise d'ordre** : **aucun texte national** — règlements du service de restauration et d'hébergement des collectivités [V pour l'absence dans R. 421-57 à R. 421-78 ; NV pour les règlements] → tout paramétrable par collectivité.
- **Privé sous contrat** : restauration hors contrat, financée par les contributions à l'organisme de gestion ; le plafond R. 531-53 vise le public [NV sur un texte exprès] ; redevances de demi-pension prévues au contrat (R. 442-48, vérifié par le 24).
- **Fonds social pour les cantines** : circulaire du 22 août 2017 (NOR MENE1718891C) [V pour les métadonnées, texte non lu] → aide affectée du 26.

### 5.2 Qualité nutritionnelle
- **Décret 2011-1227** et **arrêté du 30 septembre 2011** (C. rural **D. 230-25, D. 230-26**) [V, https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000024614716, https://www.legifrance.gouv.fr/loda/id/JORFTEXT000024614763] : fréquences sur **20 repas successifs** (au moins 10 avec crudités ou fruits crus, au plus 4 entrées grasses, au moins 4 avec poisson, etc.) ; grammages pour les produits achetés prêts ; **registre des documents de composition des repas sur trois mois** ; produits de saison identifiés sur les menus ; eau et pain en libre accès.
- **EGalim** (C. rural **L. 230-5-1**) : 50 % de produits durables et de qualité dont 20 % bio, en valeur d'achats ; version du **20 août 2026** (loi 2026-796, art. 8) [V résumé ; **texte intégral à relire**, §5.8].
- **Information des usagers** (L. 230-5-3) : à l'entrée du restaurant et **au moins une fois par an** par voie électronique, part des produits durables et de qualité [V, https://www.legifrance.gouv.fr/codes/id/LEGISCTA000022657397].
- **Menu végétarien** au moins **une fois par semaine** pour tous les gestionnaires scolaires publics ou privés (L. 230-5-6, I) [V, https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000037549829/] ; option végétarienne quotidienne pour la restauration de l'État seulement (III) [V] ; EPLE non concernés par le III [NV].
- **Plan de diversification des protéines** au-delà de 200 couverts par jour (L. 230-5-4) [V].
- **Plastique** : fin des contenants de cuisson, réchauffe et service en plastique au 1er janvier 2025 (C. env. L. 541-15-10, III) [V] ; décret 2025-80 et décision du Conseil d'État du 8 avril 2026 [NV].

### 5.3 Allergènes
- Règlement (UE) **1169/2011** (INCO), art. 9 §1 c), art. 44 §1 a) (denrées non préemballées, collectivités comprises), **annexe II : 14 allergènes** (céréales contenant du gluten, crustacés, œufs, poissons, arachides, soja, lait, fruits à coque, céleri, moutarde, sésame, anhydride sulfureux et sulfites > 10 mg/kg ou /l, lupin, mollusques) [V, https://eur-lex.europa.eu/legal-content/FR/TXT/?uri=CELEX:32011R1169].
- C. consom. **R. 412-12 à R. 412-16** (décret 2016-884, ex-décret 2015-447) [V, https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006069565/LEGISCTA000032807848/] : information **écrite**, lisible, ou indication de la manière d'y accéder librement (R. 412-14) ; en collectivité, dispense si un dispositif permet de **refuser** un ingrédient avant consommation, avec **trace conservée trois ans** (R. 412-15) ; document d'allergènes avec chaque livraison (R. 412-16).
- **Conséquence [R]** : chaque plat publié porte ses **14 allergènes codés** (présent / traces possibles / absent / non renseigné) ; **un menu n'est pas publiable** si un plat a un allergène « non renseigné » (R-RS20) ; ScolIA **n'utilise pas** le dispositif de dispense de R. 412-15.

### 5.4 PAI, paniers, régimes
- **Circulaire du 10 février 2021** (BO n° 9 du 4 mars 2021) [V, PDF : https://www.education.gouv.fr/sites/default/files/imported_files/documents/BO_MENJS_9_1383666.pdf] : l'élève mange **avec ses pairs, sans stigmatisation** ; **« le panier repas n'est donc pas la première solution à envisager »** ; les mesures de restauration du PAI ne visent que les **allergies ou intolérances médicalement avérées** — **« le PAI n'est pas destiné à … un régime alimentaire lié à des choix familiaux »** ; l'élève s'appuie sur l'affichage des allergènes ; **« seules les conséquences de la maladie utiles … (et non le diagnostic) doivent être connues »** ; partage avec consentement ; renouvellement annuel ; la famille assume fourniture, conditionnement et transport du panier [V-copie pour ce dernier point].
- **Repas de substitution confessionnels** : pas d'obligation, pas de droit ; la collectivité tient compte de l'intérêt général (CE 11 décembre 2020, n° 426483, Chalon-sur-Saône) [V, https://www.conseil-etat.fr/fr/arianeweb/CE/decision/2020-12-11/426483].
- **Mentions** : l'ancienne norme simplifiée NS-058 (abrogée en 2018, **indicative**) admettait « sans porc » ou « sans viande », pas « halal » ni « casher » [V, https://www.cnil.fr/sites/cnil/files/atoms/files/ns-058.pdf].
- **Conséquence [R]** : (a) **évictions médicales** = consignes du 34 seulement ; (b) **préférences** non médicales = catalogue fermé neutre (`sans_viande`, `sans_porc`, `vegetarien`, `sans_poisson`), déclarées par la famille, visibles de la seule restauration (R-RS16) ; ScolIA **propose de préférer le choix du plat du jour** (réservation de l'option) à une étiquette permanente (S-4012).

### 5.5 Gaspillage et biodéchets
- **Diagnostic préalable** et démarche de lutte contre le gaspillage (C. env. **L. 541-15-3**) [V] ; interdiction de rendre impropres les invendus ; **convention de don** au-delà de 3 000 repas par jour (L. 541-15-5, L. 541-15-6) [V] ; **tri à la source des biodéchets** généralisé (L. 541-21-1, au plus tard le 31 décembre 2023) [V].
- **Conséquence [R]** : pesées par service et par nature (assiettes, préparé non servi), fréquentation contre prévision, historique des diagnostics, registre des dons (facultatif).

### 5.6 Contrôle d'accès et données
- **Biométrie** : page CNIL « dispositifs biométriques pour l'accès aux cantines scolaires » (mise à jour du 18 août 2021) [V, https://www.cnil.fr/fr/les-dispositifs-biometriques-pour-lacces-aux-cantines-scolaires] : information adaptée à l'âge, **consentement explicite** et retirable, **alternative sans pénalité** ; l'autorisation unique AU-009 n'a plus de valeur depuis le 25 mai 2018 [V] ; reconnaissance faciale dans deux lycées jugée **non proportionnée**, « un contrôle par badge » suffit (CNIL, 29 octobre 2019) [V, https://www.cnil.fr/fr/experimentation-de-la-reconnaissance-faciale-dans-deux-lycees-la-cnil-precise-sa-position].
- **Conséquence [R]** : **aucune biométrie** dans ScolIA (R-RS12) ; badge, carte QR ou QR dans l'appli ; liste nominative en secours.
- **Durée des passages** : aucun texte ; NS-058 (indicative) : pas au-delà de la scolarité ou du recouvrement ; NS-042 (badges au travail) : 3 mois [NV] → proposition §22.3, **soumise au DPO**.
- **Quotient et revenus** : ne retranscrire que la tranche, jamais une copie intégrale de l'avis d'imposition (NS-058, indicative) [V] ; NIR interdit [V] → le quotient est au 24 / 26 ; le 40 ne le voit pas.
- **Conservation comptable** : 10 ans (C. com. L. 123-22) [V-copie] → pièces au 24.

### 5.7 Hygiène, commensaux
- **Plats témoins** conservés **au moins cinq jours** entre 0 et +3 °C (arrêté du 21 décembre 2009, annexe IV, point 5) [V, https://www.legifrance.gouv.fr/loda/article_lc/LEGIARTI000041864953] ; plan de maîtrise sanitaire et HACCP (règlement 852/2004) [NV] → **outillage facultatif** (lot R12), ScolIA ne remplace pas le plan de maîtrise sanitaire.
- **Avantage en nature nourriture** des salariés de droit privé (organisme de gestion) : forfait Urssaf 2026 [NV, pages en 503] → le 40 transmet les **repas pris** par les personnels au 49 / 24 ; il ne calcule rien.

### 5.8 Ce qui reste à vérifier [NV]
1. Texte intégral de **L. 230-5-1** après la loi 2026-796 du 18 août 2026.
2. Décret 2025-80 (plastique) et décision du Conseil d'État du 8 avril 2026.
3. Règlement du service de restauration et d'hébergement de la collectivité du premier client public (remise d'ordre, forfaits).
4. Circulaire de 2017 sur les fonds sociaux (texte intégral).
5. Durée de conservation des passages (avec le DPO).
6. Interface CRDC d'Op@le (FX-74).
7. Forfait Urssaf 2026 de l'avantage en nature.

---

## 6. Frontières et branchements sur le cœur

### 6.1 Qui possède quoi
| Donnée | Propriétaire | Le 40 |
|---|---|---|
| Élève, scolarité, **régime** daté | **2** | lit ; réagit à `eleve.regime_changed` |
| Régime et jours **demandés** à l'inscription | **3** | crée l'inscription au service à la validation |
| Payeurs par catégorie « restauration », titulaires | **4** | lit (avis, facture via le 24) |
| Sites, **lieux** (salle de restauration), calendrier, fermetures (R-S1), jours d'ouverture du service | **5** | lit |
| Personnels (commensaux), postes `restauration` | **6** | lit |
| Avis | **8** | émet des faits |
| Absences, demi-journées, **absence annoncée** | **12** | lit (cause de non-repas) |
| Exclusion temporaire, exclusion de la demi-pension | **13** | lit |
| Portail (entrée / sortie) | **14** | aucun |
| Informations annuelles (EGalim), campagnes | **17** | fournit le texte |
| Tarifs, facture, remises, avoirs | **24** | transmet des **quantités** |
| Porte-monnaie, encaissements | **25** | transmet des débits (si activé) |
| Bourses, fonds social | **26** | aucun (le 24 impute) |
| Consignes alimentaires (évictions, panier), trousses | **34** | lit l'audience « restauration » |
| Stages | **39** | lit (cause « stage ») |
| Internat (attendus du soir et du matin, absences annoncées) | **41** | lit |
| Séjours scolaires, commande cuisine, paniers | **42** | lit la commande, la produit |
| Études, garderie, goûter périscolaire | **43** | aucun (les booléens « étude », « garderie » de la grille partent au 43) |
| Achats alimentaires (valeurs EGalim) | **48** | lit les agrégats (ou saisie annuelle) |
| Repas des salariés (avantage en nature) | **49** | transmet des quantités |
| **Services de restauration, points de service, inscriptions au service, réservations, choix de plat, non-repas et causes, passages, badges de restauration, menus, plats, allergènes, labels, préférences non médicales, prévisions, commandes au prestataire, commensaux et tickets, paniers, pesées, indicateurs** | **40** | possède |

### 6.2 Ports consommés
`eleve::scolarite_a`, `eleve::regime_a`, `eleve::identite_affichee` (2) ; `inscriptions::validee` (3) ; `personnes::titulaires`, `personnes::payeurs(eleve, 'restauration', date)` (4) ; `structure::calendrier`, `structure::service_ouvert(etab, 'restauration', date)`, `structure::lieu` (5) ; `personnels::actifs` (6) ; `assiduite::absence_prevue(eleve, jour, plage)` (12) ; `discipline::exclusion(eleve, jour)` (13) ; `sante::consignes_du_jour(etab, 'restauration', date)`, `sante::consignes(eleve, 'restauration', date)`, `sante::trousse(eleve)` (34) ; `stages::en_stage` (39) ; `internat::attendu(etab, nuit)`, `internat::absences_annoncees` (41) ; `sorties::en_sortie(eleve, date, plage)`, `sorties::commande_restauration(sortie)` (42) ; `achats::egalim_annee` (48) ; `facturation::facturer_source` (24) ; `paiements::debiter_porte_monnaie` (25).

### 6.3 Ports exposés
| Port | Consommateurs |
|---|---|
| `restauration::droit(eleve, jour, service)` → droit, cause de non-repas éventuelle | 2 (fiche), 41, 42, 24 |
| `restauration::consommations(eleve, mois)` → repas dus, servis, non-repas par cause, hors droit | 24 |
| `restauration::passages(eleve, jour, service)` | 41 (dîner pris), 12 (signal CPE facultatif) |
| `restauration::prevision(etab, jour, service)` → agrégats | 47, prestataire |
| `restauration::commande_sortie(sortie)` (réception) | 42 |
| `restauration::retirer_repas(eleve, jours, cause)` | 42, 39, 41 (commande, idempotente) |

### 6.4 Répercussions automatiques
| Événement | Effet dans le 40 |
|---|---|
| `inscription.validee` avec régime DP ou interne | inscription au service avec les jours demandés, datée de la rentrée |
| `eleve.regime_changed` | inscription close ou ouverte à la date d'effet (corrige RR-06, DC-13) ; consommations recalculées et transmises au 24 |
| `scolarite.depart_enregistre`, `eleve.decede` | inscription close ; réservations futures annulées ; badge désactivé ; aucune relance |
| `assiduite.absence_annoncee` (journée ou plage du repas) | non-repas cause « absence annoncée » si **avant** l'heure limite (S-4003), sinon le repas reste dû selon S-160 du 24 |
| `sorties.participant_pret` / `desiste` (42) | non-repas cause « sortie scolaire » (ou retrait) |
| `stage.periode_ouverte` (39) | non-repas cause « stage » |
| `internat.absence_annoncee`, `internat.attendu_modifie` (41) | non-repas du dîner et du petit-déjeuner |
| `discipline.exclusion` (13) | non-repas cause « exclusion » (libellé famille « non servi ») |
| `referentiel.fermeture_exceptionnelle` (5) restauration fermée | non-repas cause « fermeture » pour tous ; si restauration maintenue, inchangé |
| `sante.consigne_publiee` / `retiree` (34) | liste du jour mise à jour ; alerte au passage |
| `payeurs.modifies` (4) | aucun effet au 40 (le 24 répartit) |

---

## 7. Concepts et vocabulaire

| Terme | Définition | Table |
|---|---|---|
| **Service** | prestation de repas : `dejeuner`, `diner`, `petit_dejeuner`, `gouter_internat` [R : le goûter périscolaire est au 43] ; horaires, jours d'ouverture, point de service | `ent_repas_activite` (étendue) |
| **Point de service** | lieu du 5 où l'on sert (self principal, salle annexe, site B) | `ent_repas_point_service` |
| **Formule** | mode d'accès : `forfait` (jours fixes), `reservation` (repas réservé à l'avance), `ticket` (pris sans réservation, au réel) | `ent_repas_formule` |
| **Inscription au service** | pour un élève, une formule et des jours, sur une période (remplace la grille) | `ent_eleve_grille_repas` (étendue) |
| **Réservation** | engagement de prendre un repas un jour donné (formule `reservation`), annulable jusqu'à l'heure limite | `ent_repas_reservation` |
| **Choix du plat** | option choisie à l'avance (plat principal A, B, végétarien) | `ent_repas_reservation.choix_id` |
| **Droit au repas** | vue calculée : élève × jour × service → `du` / `non_du` + cause | vue `ent_repas_droit_v` |
| **Non-repas** | repas qui ne sera pas servi alors qu'il était dû, avec sa **cause codée** et sa source (autre socle ou annulation) | `ent_repas_non_repas` (remplace la dispense libre) |
| **Passage** | fait horodaté : un convive a été servi (ou refusé) à un service | `ent_repas_passage` (étendue) |
| **Commensal** | convive qui n'est pas élève de l'établissement (personnel, stagiaire, intervenant, invité, élève d'un autre établissement) | `ent_repas_commensal` |
| **Menu** | pour un jour et un service, la liste des plats publiés, en **versions** | `ent_repas_menu`, `ent_repas_menu_version` |
| **Plat** | recette réutilisable : catégorie (entrée, plat, accompagnement, laitage, dessert), allergènes, labels, végétarien | `ent_repas_plat` |
| **Préférence non médicale** | choix alimentaire déclaré par la famille, d'un catalogue fermé neutre | `ent_repas_preference` |
| **Consigne alimentaire** | conséquence fonctionnelle publiée par le 34 (éviction, panier, trousse) ; **lue**, jamais stockée par le 40 | (34) |
| **Panier** | repas fourni par la famille (PAI) ou préparé par la cuisine (sortie, examen) | `ent_repas_panier` |
| **Prévision** | effectif calculé par jour, service, choix, à une heure donnée (J-7, J-1, J0 10 h), figée en **instantanés** | `ent_repas_prevision` |
| **Pesée** | mesure du gaspillage par service et nature | `ent_repas_pesee` |

---

## 8. Le droit au repas (corrige DC-07, DC-11, DC-13, DC-17, RR-06)

### 8.1 Inscription au service
- À la **validation de l'inscription** (3), l'élève demi-pensionnaire ou interne reçoit une inscription au service **à la date de rentrée**, avec la formule et les **jours** demandés (4 ou 5 jours, mercredi ou non) ; l'interne est inscrit au dîner et au petit-déjeuner selon son **rythme** (41).
- Changer de jours (« 4 jours à partir du 1er octobre ») = nouvelle **version datée** ; pas de chevauchement (contrainte d'exclusion) ; ancienne version close la veille.
- Le **régime** du 2 et l'inscription du 40 sont **cohérents par construction** : on ne peut pas inscrire au forfait un élève externe (sauf formule `ticket` ouverte aux externes, S-4001) ; un changement de régime ferme ou ouvre l'inscription (R-RS2).
- Le secrétariat voit les **incohérences** : demi-pensionnaire sans inscription, inscrit sans payeur « restauration » (4), inscrit parti.

### 8.2 Calcul du droit
Pour un élève, un jour J et un service S, `restauration::droit` :
1. **Ouvert ?** Le service est-il ouvert ce jour (calendrier et services du 5, jours d'ouverture du service) ? Sinon : rien.
2. **Dû ?** Forfait : J est un jour de l'inscription. Réservation : une réservation confirmée existe. Ticket : non dû, payé au passage.
3. **Non-repas ?** Si dû, existe-t-il une **cause** : absence annoncée dans le délai (12), sortie scolaire (42), stage (39), internat absent (41), exclusion (13), fermeture (5), panier famille (34, si le règlement le traite à part), annulation de la famille dans le délai, **non-repas posé** par le secrétariat (motif codé). La cause la plus forte l'emporte ; toutes sont gardées pour l'explication.
4. Résultat : `du` (servir et compter), `non_du` (+ cause), `hors_droit` (servi sans droit : ticket, imprévu).

**Une seule fonction** sert la prévision, le passage, la famille et la facture (R-RS5).

### 8.3 Ce que la famille voit
« Déjeuner du mardi 12 : **compté** » ; « jeudi 14 : **non compté — sortie au musée** » ; « vendredi 15 : **annulé par vous le 13 à 18 h** » ; « lundi 18 : **pris** à 12 h 21 » ; « mercredi 20 : **non pris, compté** (absence non annoncée) ». Chaque ligne a un « pourquoi ? » (phrases fixes, D-023).

---

## 9. Réservations et annulations (RES-03 de D11 ; nouveau)
- **Formule réservation** : la famille ou l'élève (majeur, ou mineur si l'établissement l'autorise, S-4005) réserve un ou plusieurs repas sur un calendrier ; **heure limite** par service (S-4003 : par défaut la veille à 18 h ou le jour même à 9 h 30) ; au-delà, la réservation est refusée (ou acceptée comme « tardive » si le règlement le prévoit, au tarif majoré du 24).
- **Formule forfait** : la famille **annule** un repas du forfait avant l'heure limite (si le règlement l'autorise) ; l'annulation est une **cause de non-repas** ; sa conséquence financière dépend du règlement (S-160 du 24).
- **Choix du plat** : quand le service propose plusieurs plats, l'élève ou la famille choisit la veille ; à défaut, **plat par défaut** (S-4011) ; le choix nourrit la prévision par plat.
- **Réservations récurrentes** (« tous les mardis ») : génèrent des réservations datées, annulables une à une.
- **Ce qu'on ne peut pas faire** : réserver ou annuler un jour **passé** (corrige RR-02) ; réserver pour un autre élève que le sien ; réserver pour un élève sans payeur « restauration » (avertissement au secrétariat, pas de blocage du repas : l'enfant mange, R-RS27).
- **Indisponibilité** (panne, G34 de la matrice) : l'heure limite est **prolongée** automatiquement de la durée de l'incident déclaré par l'administrateur (R-RS28).

---

## 10. Passages (RES-05 de D11 ; corrige DC-12, DC-14, DC-15, DC-16, RR-01, RR-03, RR-07, RR-08)

### 10.1 Moyens d'identification
- **Badge** (carte RFID ou carte imprimée à code) : identifiant **aléatoire** propre à la restauration (pas l'INE, pas le numéro d'élève), stocké sous forme d'**empreinte** pour la recherche et affiché tronqué (R-RS13) ; un seul badge **actif** par élève ; perte → nouveau badge, l'ancien désactivé ; coût de la carte perdue au 24 (article « carte de cantine perdue »).
- **QR dans l'appli** de l'élève (jeton tournant, valable 60 s, lié à l'appareil) (S-4006).
- **Liste** : recherche par nom avec photo, pour l'élève sans badge.
- **Aucune biométrie** (R-RS12).
- **Lecteurs** : un lecteur USB « clavier » ou NFC branché sur la tablette du self, ou la caméra de la tablette pour les QR [H] ; achat éventuel = Q-RS4.

### 10.2 Ce que le surveillant voit (en une seconde)
- **Vert** « dû » : photo, prénom, nom, classe, plat choisi ;
- **Orange** « à vérifier » : déjà passé à ce service aujourd'hui (heure du premier passage) ; non-repas enregistré (« sortie au musée ») ; badge désactivé ; élève parti ; hors droit (ticket possible si la formule l'autorise) ;
- **Bandeau consigne** (si le 34 a publié pour l'audience « restauration ») : « **Éviction : arachide, fruits à coque** — trousse : infirmerie » ; « **Panier famille** » ; « **Prévenir l'infirmerie en cas de malaise** » ; **jamais** de diagnostic ni de mot « allergie » ;
- **Jamais** : impayé, bourse, aide, quotient, motif d'absence, sanction (R-RS14).
- Le surveillant **ne refuse pas** l'accès à un élève orange sans décision : il **sert** (et le passage est marqué « hors droit ») ou appelle le CPE ; la règle de l'établissement le dit (S-4007, par défaut : servir, ne jamais laisser un élève sans repas).

### 10.3 Règles du passage
- Un passage est **unique** par convive, service et jour (le second est enregistré « re-passage » et non facturé) (R-RS10).
- Il est **daté au moment du geste**, dans les horaires du service (Europe/Paris) ; **jamais dans le futur** ; une saisie a posteriori (oubli, panne) est un fait **motivé** par un responsable, dans un délai de 7 jours (S-4008), au-delà par la comptabilité (corrige RR-01, DC-15).
- **Correction** : annuler ou réattribuer un passage = fait correctif motivé (corrige RR-07, DC-16) ; jamais de suppression.
- **Hors ligne** : la tablette garde la liste du jour (droits, photos, consignes) chiffrée ; passages horodatés localement ; synchronisés ; effacés en fin de service + 12 h (R-RS19).
- La **charge des événements** ne contient pas le numéro de badge (corrige RR-07).

---

## 11. Menus, allergènes, qualité (RES-06 de D11 ; corrige RR-05)

### 11.1 Plats et menus
- **Plat** : libellé, catégorie, **14 allergènes** (présent, traces possibles, absent ; « non renseigné » bloque la publication, R-RS20), végétarien (oui / non), labels (bio, label rouge, AOP, IGP, pêche durable, « fait maison » [H]), origine de la viande (si renseignée), produit de saison ; plats **réutilisables** d'une semaine à l'autre.
- **Menu** : par jour et service, liste ordonnée de plats, avec les **options** (plat A, plat B, végétarien) ; **versions** : brouillon → publié → modifié (nouvelle version datée, motif : rupture, livraison) ; la famille voit « modifié le 12 à 9 h : poisson remplacé par omelette ».
- **Contrôles à la publication** [R] : au moins **un menu végétarien** dans la semaine (L. 230-5-6) — sinon publication avec avertissement nommé ; allergènes renseignés ; **fréquences sur 20 repas** (arrêté de 2011) affichées comme **indicateur** (pas un blocage : le décompte dépend des fiches techniques).
- **Registre** des menus publiés et de leurs versions, conservé **trois mois** au minimum (D. 230-26) ; ScolIA le garde l'année (S-4013).
- **Diffusion** : appli (familles, élèves), affichage à l'entrée (écran ou PDF), prestataire, **aucune** donnée personnelle.

### 11.2 Ce que la famille fait avec les allergènes
- Elle voit les allergènes de chaque plat ; elle peut **filtrer sur son appareil** (« signaler les plats contenant lait ») : le filtre est enregistré **localement** sur l'appareil, **jamais** sur le serveur (R-RS17) — l'établissement ne sait pas ce que la famille filtre.

### 11.3 EGalim et information des usagers
- Part des produits durables et de qualité et part bio de l'année civile : **lue au 48** (achats) ou **saisie** par le responsable avec la pièce de télédéclaration « ma-cantine » ; ScolIA prépare le **texte d'information annuelle** et l'**affichage** à l'entrée (L. 230-5-3) ; diffusion par le 17.

---

## 12. Santé et régimes (corrige DC-03, RR-04, RR-09)

### 12.1 Consignes du 34
- La restauration lit, pour la date, la **liste des consignes de l'audience « restauration »** (34 `sante::consignes_du_jour`) : photo, nom, classe, **code de consigne** (`repas_sans_arachide`, `repas_sans_fruits_a_coque`, `repas_sans_lait`, `repas_sans_oeuf`, `repas_sans_gluten`, `repas_sans_poisson`, `repas_panier`, `pai_alimentaire`, `trousse_urgence` avec lieu, `prevenir_infirmerie`) [F catalogue 211], **sans la source** (PAI, PAP, manuelle) ni le diagnostic.
- Au **passage**, le bandeau consigne s'affiche (§10.2) ; la lecture est **journalisée** (qui, quand, quel élève) (R-RS15).
- **Plats compatibles** : pour une consigne d'éviction, ScolIA signale au service les plats du jour qui contiennent l'allergène (« plat A : contient arachide ») et l'option compatible ; l'**étiquette** de l'assiette adaptée porte le prénom et la classe, **sans motif** (R-RS18).
- **Export au prestataire** (34 CS-56) : liste de l'audience « restauration » (nom, classe, consigne, photo), sans source, tracée, durée de validité imprimée.
- **Santé désactivée** (D-034, pas d'HDS, N-1 non activé sans avis du DPO) : la restauration voit « **consignes alimentaires non gérées dans ScolIA : la liste de l'infirmerie fait foi** » ; **aucune saisie de remplacement** dans le 40 (R-RS15, CRS-20).

### 12.2 Paniers PAI
- Consigne `repas_panier` (34) : le passage enregistre « panier famille » (le repas n'est pas produit ; tarif « panier » au 24, sans nature médicale visible, 24 §11) ; stockage du panier (réfrigérateur dédié, étiquette au nom) : procédure de la restauration, rappelée dans l'écran du jour (« 3 paniers attendus aujourd'hui ») ; la circulaire rappelle que le panier n'est **pas la première solution** : ScolIA n'en fait pas un défaut.

### 12.3 Préférences non médicales
- Catalogue fermé (S-4012) : `vegetarien`, `sans_viande`, `sans_porc`, `sans_poisson` ; déclarée par un titulaire (ou le majeur) ; **datée** ; visible de la **seule restauration** (et compte agrégé au prestataire) ; **jamais** au professeur, à la vie scolaire, à l'accueil ; **jamais** dans un export nominatif autre que la commande d'une sortie (42) ; **pas** de mention religieuse (R-RS16).
- Recommandation : préférer **le choix du plat du jour** (réservation de l'option végétarienne) — rien n'est alors su durablement.
- La préférence n'est **pas un droit** (CE 2020) : si aucune option n'est servie ce jour, l'écran le dit (« pas d'option sans viande aujourd'hui »).

---

## 13. Prévision et production (RES-07 de D11 ; reprend NC-06, NC-07, NC-15)

### 13.1 Effectif
- **Calcul** : pour un jour et un service, somme des droits `du` (§8.2), par **choix de plat**, + commensaux réservés + paniers de sortie (42) − non-repas ; séparément : paniers famille, assiettes adaptées par consigne (nombre par code, **sans nom** pour la cuisine sauf étiquette).
- **Instantanés** figés à **J-7**, **J-1 18 h**, **J0 10 h** (S-4009) : on sait ce qu'on prévoyait et quand ; écart avec le servi.
- **Coefficient de présence** observé (taux de passage des inscrits au forfait par jour de semaine, par niveau) proposé à la cuisine pour corriger l'effectif (« le vendredi, 92 % des inscrits mangent ») ; **affiché**, jamais appliqué sans accord (R-RS22).
- **Quatre statuts** du jour repris de Next.js (attendu-pris, attendu-manquant, imprévu, en sortie) : pendant et après le service.

### 13.2 Prestataire (concession)
- **Commande** au prestataire à l'heure contractuelle (effectifs par plat, adultes, paniers, consignes agrégées « 3 assiettes sans arachide ») ; envoyée à **l'adresse du prestataire déclarée par l'établissement** (pas une adresse libre) ou via un **compte restreint** du prestataire (rôle « prestataire de restauration » : prévisions agrégées, menus, consignes de l'audience « restauration » si l'établissement l'a décidé) ; **comptage servi** partagé en fin de service ; rapprochement avec la **facture du prestataire** au 48.
- **Menus du prestataire** : importés (fichier) ou saisis, puis relus et publiés par l'établissement.

### 13.3 Service du soir et du matin (41)
- Dîner et petit-déjeuner : effectif = attendu de l'internat (41) + internes externés + commensaux ; mis à jour à l'heure limite du soir.

---

## 14. Commensaux (nouveau ; corrige DC-21)
- **Personnels** (6) : ticket ou réservation ; repas pris transmis au 24 (facture ou retenue) et au 49 (avantage en nature, salariés de droit privé) ; tarif commensal au 24.
- **Stagiaires, intervenants, invités** : fiche de commensal minimale (nom, qualité, période), sans compte, ticket nominatif ou ticket « invité de X » ; facturation au service invitant ou à la personne.
- **Élève d'un autre établissement** (convention, internat mutualisé) : commensal rattaché à une convention, facturé à l'établissement d'origine (Q-RS7).
- **Réunion, jury, conseil** : réservation de groupe (nombre, centre de coût).

---

## 15. Paniers et repas hors self
- **Sorties scolaires** (42) : la commande (jours, paniers et variantes, consignes agrégées, livraison) est **possédée par le 42** ; le 40 la **reçoit** dans sa boîte de restauration, la confirme, la produit, et ses avenants « annule et remplace » ; les repas du self des participants sont **retirés** (non-repas cause « sortie scolaire ») ; sortie **obligatoire** sur la pause méridienne : panier à la charge de l'établissement (42 R-SV8).
- **Examens, journées banalisées** : paniers pour un groupe (nombre, consignes agrégées).
- **Stages** : non-repas cause « stage » (39).

---

## 16. Argent (socles 24, 25, 26)
- Le 40 publie au 24, par élève et par mois, des **quantités** (R-RS29) : repas **dus** (forfait), **réservés**, **servis**, **non-repas par cause**, **hors droit** servis, **paniers**, **re-passages** (non facturés) ; et pour les commensaux, les repas servis. **Jamais un montant.**
- **Le règlement financier** (24) décide : forfait (par mois ou trimestre) ou réel ; quelles causes donnent remise (absence justifiée longue, stage, fermeture, sortie, exclusion : S-157 du 24) ; repas réservé non pris facturé ou non (S-160) ; tarif majoré hors réservation.
- **Explication** de chaque ligne (24 §14 « d'où vient cette ligne ? ») : « 16 déjeuners dus en octobre, 2 non comptés (sortie du 14, fermeture du 22), 14 × 4,80 € ».
- **Idempotence** : chaque quantité porte une **clé de source** (élève, mois, service, version) ; un recalcul remplace une ligne non émise, produit un avoir ou un complément après émission (corrige DC-08).
- **Doublon** (D-034) : **mode suivi** — quantités publiées, le 24 calcule sans émettre ; **état mensuel** de comparaison avec Charlemagne (RES-214) (R-RS31).
- **Porte-monnaie prépayé** (T-23, 25 PAY-206) : **désactivé par défaut** ; si activé, chaque passage débite au 25 ; solde bas → avis au payeur (8 `restauration.solde_bas`) ; **jamais** de refus de repas pour solde nul (R-RS27).
- **Impayés** : **aucune exclusion de cantine automatique** ; une exclusion pour impayé est une **décision de la direction**, après contact et proposition d'aide (26, 36) (T-27), notifiée par écrit, jamais visible au passage ; elle se traduit en fin d'inscription au service datée (R-RS27, Q-RS6).
- **Public (EPLE)** : état des **droits constatés** par période pour la saisie dans Op@le (FX-74 ; interface automatique si le ministère l'ouvre) ; régie au 25.

---

## 17. Gaspillage et indicateurs
- **Pesées** par service : restes d'assiettes, préparé non servi, pain ; saisie en trois chiffres sur la tablette (ou balance connectée plus tard [H]) ; ratio par convive.
- **Écart prévu / servi** par jour et service ; **taux de fréquentation** des inscrits ; **taux de réservation non honorée**.
- **Diagnostic** annuel (L. 541-15-3) : rapport prérempli (quantités, coûts estimés si le coût matière est saisi, actions) ; historique.
- **Dons** (facultatif) : registre (date, association, quantités) ; **biodéchets** : non suivis par défaut (affaire du prestataire ou de la collectivité).
- **Pilotage** (47) : indicateurs agrégés sans nom ; EGalim ; nombre de consignes par code (agrégé, sans nom, jamais par classe si l'effectif est < 5, R-RS24).
- **Signal CPE** (facultatif, S-4014, désactivé par défaut) : demi-pensionnaire **présent en cours** qui ne passe plus au self depuis N jours → information au **seul CPE**, sans diffusion (CRS-30).

---

## 18. Cas limites (catalogue)
| # | Situation | Traitement |
|---|---|---|
| CRS-01 | Élève passé de DP à externe au 1er octobre | Inscription close au 30/09 ; réservations futures annulées ; consommations recalculées ; avoir si forfait déjà facturé (24 CF-23) |
| CRS-02 | Changement de régime rétroactif saisi le 15 pour le 1er | Inscription close au 1er ; passages entre le 1er et le 15 « hors droit » (servis) ; le 24 propose la régularisation |
| CRS-03 | Absence annoncée à 8 h pour la journée, heure limite 9 h 30 | Non-repas cause « absence annoncée » ; non facturé si le règlement le prévoit |
| CRS-04 | Absence annoncée à 11 h | Repas dû selon S-160 ; la famille voit « annoncé après l'heure limite » |
| CRS-05 | Absence non annoncée | Repas dû, non pris, « compté » (si le règlement le prévoit) |
| CRS-06 | Absence justifiée de 3 semaines (maladie) | Non-repas cause « absence justifiée » à partir du seuil (24 R-F12) ; aucun motif |
| CRS-07 | Sortie au musée de 9 h à 16 h | Non-repas « sortie scolaire » ; panier commandé par le 42 si pause méridienne |
| CRS-08 | Sortie de 9 h à 11 h (retour avant midi) | Plages réelles (42) : le repas reste dû |
| CRS-09 | Élève désisté d'une sortie la veille | Non-repas retiré, repas de nouveau dû ; cuisine informée à J-1 |
| CRS-10 | Stage de 4 semaines | Non-repas « stage » ; remise selon S-157 |
| CRS-11 | Exclusion temporaire de 2 jours | Non-repas « exclusion » ; libellé famille « non servi » |
| CRS-12 | Exclusion de la demi-pension (sanction) | Inscription close sur la période par le 13 ; jamais au passage en rouge public |
| CRS-13 | Fermeture du self (panne de four) | Fermeture du service (5) ; non-repas « fermeture » pour tous ; paniers éventuels ; familles prévenues |
| CRS-14 | Neige, cours suspendus, internes présents | Restauration maintenue (R-S1 du 5) ; seuls les internes et présents comptent |
| CRS-15 | Grève du personnel de cuisine | Fermeture du service, ou menu froid (version de menu) ; non-repas « fermeture » si non servi |
| CRS-16 | Élève sans badge | Recherche par nom et photo ; passage normal |
| CRS-17 | Badge prêté à un camarade | Photo à l'écran : le surveillant voit l'écart ; passage annulé ou réattribué (fait correctif) ; fait de liaison au CPE |
| CRS-18 | Badge perdu | Désactivé ; nouveau badge ; carte perdue facturée selon le règlement |
| CRS-19 | Élève qui passe deux fois | « Re-passage » orange ; non facturé ; signal si répété |
| CRS-20 | Santé désactivée (pas d'HDS) | Bandeau « liste de l'infirmerie fait foi » ; aucune saisie de remplacement |
| CRS-21 | Nouvel élève allergique arrivé le matin | Consigne manuelle publiée par l'infirmière (34 CS-34, 3 mois au plus) avant le PAI ; alerte au passage dès la publication |
| CRS-22 | Consigne retirée hier | Disparaît de la liste du jour (34 CS-57) |
| CRS-23 | Plat du jour contenant un allergène d'une consigne | Signal au service ; option compatible proposée ; étiquette d'assiette sans motif |
| CRS-24 | Menu modifié le matin (rupture) | Nouvelle version ; allergènes revérifiés ; famille avertie si un plat change d'allergènes |
| CRS-25 | Menu sans plat végétarien dans la semaine | Avertissement nommé à la publication ; publication possible, tracée |
| CRS-26 | Panier famille oublié | Procédure de l'établissement (appel famille, repas adapté si possible) ; aucune décision par ScolIA |
| CRS-27 | Élève majeur qui réserve pour lui | Permis ; ses parents ne voient plus ses réservations s'il le demande et si le règlement le prévoit (S-4005) |
| CRS-28 | Parents séparés : la mère paie la cantine, le père réserve | Réservation par tout titulaire ; facture au payeur « restauration » (4) ; chacun voit ses propres gestes |
| CRS-29 | Désaccord connu des titulaires sur l'inscription au service | Changement bloqué, renvoyé au secrétariat |
| CRS-30 | Demi-pensionnaire présent en cours qui ne mange plus depuis 5 jours | Signal facultatif au seul CPE (S-4014) ; jamais une diffusion |
| CRS-31 | Impayé de 3 mois | Aucune conséquence au passage ; procédure du 25 (relance, aide 26, médiation 36) ; exclusion = décision écrite de la direction |
| CRS-32 | Porte-monnaie à zéro | Repas servi ; solde négatif signalé au payeur ; aucun refus (R-RS27) |
| CRS-33 | Passage saisi par erreur pour un homonyme | Fait correctif « réattribué à … » ; facture recalculée avant émission |
| CRS-34 | Panne réseau au self | Mode hors ligne ; synchronisation ; conflits listés |
| CRS-35 | Panne totale de ScolIA | Liste papier imprimée du jour (droits, consignes) à J-1 18 h (S-4010) ; saisie a posteriori motivée |
| CRS-36 | Invité de la direction | Ticket « invité de la direction » ; centre de coût |
| CRS-37 | Stagiaire enseignant | Commensal daté ; tarif commensal |
| CRS-38 | Personnel de l'OGEC qui déjeune | Repas pris → 49 (avantage en nature) et 24 (participation) |
| CRS-39 | Élève d'un autre établissement interne chez nous | Commensal sous convention ; facture à l'établissement d'origine (Q-RS7) |
| CRS-40 | Repas du soir d'un interne absent annoncé à 15 h | Non-repas « internat absent » ; dîner et petit-déjeuner |
| CRS-41 | Interne présent non attendu (devait partir) | Hors droit servi ; ajouté à l'effectif si encore possible |
| CRS-42 | Jour férié, pont | Calendrier du 5 : service fermé, rien de dû |
| CRS-43 | Élève décédé | Inscription close ; aucune relance ; badge désactivé ; aucune ligne future |
| CRS-44 | Demande d'accès RGPD aux passages | Export des passages et non-repas de l'élève (période), sans données d'autres élèves |

---

## 19. Intelligence artificielle (contrat IA)

### 19.1 Ce que l'IA fait [R]
- **Expliquer la prévision** (« effectif de jeudi plus bas : sortie des 2nde B et C, 54 élèves ») (N0).
- **Proposer le menu de la semaine** à partir des plats connus, vérifié contre les fréquences sur 20 repas et la présence d'un menu végétarien, avec ses raisons ; **brouillon** que le responsable modifie et publie (N1 brouillon).
- **Rédiger l'information EGalim** annuelle et le **diagnostic** de gaspillage (N0).
- **Repérer des anomalies** déterministes et les expliquer (passages hors horaires, re-passages répétés, écart prévu / servi inhabituel) (N0).
- **Répondre** à « combien de végétariens jeudi ? », « mon fils a-t-il mangé hier ? » (dans la portée de la personne) (N0).
- **Inscription au service** dictée (reprise de NC-13) : préparer le changement de jours, confirmé par la personne (N2).

### 19.2 Ce que l'IA ne fait pas [R]
- **Aucune lecture des consignes de santé** pour en déduire quoi que ce soit ; ne dit jamais « allergique » ; refus explicite (« je n'ai pas accès à la santé de l'élève ») (34 R-S30, `IA_COEUR.md` Q-IA5).
- Ne **publie** aucun menu, ne **facture** rien, ne **refuse** aucun repas, n'**exclut** personne, ne contacte aucune famille.
- Ne profile pas les élèves (habitudes alimentaires nominatives).

### 19.3 Capacités déclarées
| Capacité | Sensibilité | Portée | Niveau |
|---|---|---|---|
| `restauration.lire_prevision` | C0 (agrégats) | restauration, direction | lecture |
| `restauration.expliquer_droit` | C1 | famille (son enfant), comptabilité | lecture |
| `restauration.proposer_menu` | C0 | restauration | N1 (brouillon) |
| `restauration.rediger_information_egalim` | C0 | restauration, direction | N0 ; diffusion N3 par l'écran (17) |
| `restauration.preparer_inscription_service` | C1 | secrétariat | N2 |
| consignes, préférences nominatives | — | — | **fermé à l'IA** |

---
## 20. Actions métier (RES-100 à RES-234)

Colonnes : acteur · droit (§22.1) ; règles ; événement ; lot. ★ = réauthentification récente. Correspondance avec D11 : RES-01 → RES-100 ; RES-02 → RES-120 ; RES-03 → RES-140 ; RES-04 → RES-126 ; RES-05 → RES-160 ; RES-06 → RES-200 ; RES-07 → RES-180 ; RES-08 → RES-165 ; RES-09 → RES-210.

### 20.1 Paramétrage du service
| RES | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RES-100 | Définir les services (déjeuner, dîner, petit-déjeuner, goûter d'internat), horaires, jours d'ouverture | RST, DIR · `restauration.parametrer` | calendrier du 5 | `restauration.service_modifie` [F `repas.activite_created`] | R0 (modifier), R1 |
| RES-101 | Déclarer les points de service (lieux du 5) | RST · `restauration.parametrer` | | `restauration.point_modifie` | R1 |
| RES-102 | Définir les formules offertes (forfait, réservation, ticket) et qui y accède (DP, internes, externes, commensaux) | DIR, CPT · `restauration.parametrer` | S-4001 | `restauration.formule_modifiee` | R1 |
| RES-103 | Fixer les heures limites de réservation et d'annulation par service | DIR · `restauration.parametrer` | S-4003 | idem | R0, R4 |
| RES-104 | Fixer le plat par défaut quand aucun choix n'est fait | RST | S-4011 | idem | R4 |
| RES-105 | Activer le catalogue des préférences non médicales | DIR | S-4012 | idem | R7 |
| RES-106 | Déclarer le prestataire, son adresse de commande, ses heures | DIR · `restauration.parametrer` | R-RS23 | `restauration.prestataire_modifie` | R3 |
| RES-107 | Ouvrir un compte restreint au prestataire | DIR · `restauration.parametrer` ★ | §13.2 | `restauration.prestataire_acces` | R3 |
| RES-108 | Enregistrer les lecteurs et tablettes du self | RST · `restauration.parametrer` | | `restauration.appareil_enregistre` | R2 |
| RES-109 | Fixer la conduite au passage pour un élève « orange » | DIR | S-4007 | idem | R2 |
| RES-110 | Déclarer une fermeture ou une modification ponctuelle du service (via le 5) | DIR, RST | R-S1 du 5 | `referentiel.fermeture_exceptionnelle` | R1 |
| RES-111 | Déclarer une indisponibilité de la réservation (prolongation automatique des délais) | administrateur | R-RS28 | `restauration.incident_reservation` | R4 |
| RES-112 | Paramétrer les instantanés de prévision | RST | S-4009 | idem | R3 |
| RES-113 | Paramétrer le délai de saisie a posteriori | DIR | S-4008 | idem | R0 |
| RES-114 | Choisir l'identification (badge, QR, liste) | DIR | S-4006 | idem | R2 |

### 20.2 Inscription au service et non-repas
| RES | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RES-120 | Créer l'inscription au service à la validation de l'inscription (3) (NC-12) | SYS ; ADF ajuste · `restauration.inscrire` | R-RS2 | `restauration.inscription_ouverte` [F `eleve.grille_repas_posed`] | R1 |
| RES-121 | Changer les jours ou la formule (version datée) | ADF · `restauration.inscrire` ; PAR (demande) | R-RS3 | `restauration.inscription_modifiee` | R0 (chevauchement), R1 |
| RES-122 | Demander un changement de jours (famille) | PAR, MAJ | validé par le secrétariat | `restauration.demande_inscription` | R4 |
| RES-123 | Clore l'inscription (régime, départ, décès) | SYS ; ADF | §6.4 | `restauration.inscription_close` [F `eleve.grille_repas_close`] | R0 |
| RES-124 | Voir les incohérences (DP sans inscription, inscrit sans payeur, inscrit parti) | ADF, CPT · `restauration.lire` | | — | R1 |
| RES-125 | Inscrire un interne au dîner et au petit-déjeuner selon son rythme (41) | SYS | | idem 120 | R1 |
| RES-126 | Poser un non-repas avec cause codée (secrétariat) (NC-19) | ADF, CPE · `restauration.inscrire` | motif codé, jamais libre (R-RS7) | `restauration.non_repas` [F `eleve.dispense_repas_posed`] | R0 (codé), R1 |
| RES-127 | Annuler un non-repas | ADF, CPE | jamais pour un jour passé sans motif | `restauration.non_repas_annule` | R0 |
| RES-128 | Recevoir les non-repas automatiques (12, 13, 39, 41, 42, 5) | SYS | R-RS6 | `restauration.non_repas` | R1 |
| RES-129 | Voir pour un élève le calendrier des droits et non-repas avec leurs causes | ADF, CPT, PAR, ELV · `restauration.lire` | §8.3 | — | R1 |
| RES-130 | Expliquer un droit (« pourquoi ce repas est compté ? ») | PAR, CPT | phrases fixes | — | R1 |
| RES-131 | Retirer les repas d'un élève sur une période (port `retirer_repas`) | SYS (42, 39, 41) | idempotent | `restauration.non_repas` | R1 |
| RES-132 | Ouvrir la formule ticket à un externe (repas occasionnel) | ADF | S-4001 | idem 120 | R4 |
| RES-133 | Inscrire en lot (classe, niveau) au changement de rentrée | ADF | aperçu | idem 120 | R1 |
| RES-134 | Reconduire les inscriptions des réinscrits pour l'année suivante (aperçu) | ADF | confirmation | idem 120 | R1 |

### 20.3 Réservations et choix
| RES | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RES-140 | Réserver un repas (RES-03 de D11) | PAR, MAJ, ELV (si permis) | heure limite, pas de jour passé (R-RS8) | `restauration.reservation` | R4 |
| RES-141 | Annuler une réservation | PAR, MAJ, ELV | idem | `restauration.reservation_annulee` | R4 |
| RES-142 | Annuler un repas du forfait (si le règlement l'autorise) | PAR, MAJ | cause « annulation famille » | `restauration.non_repas` | R4 |
| RES-143 | Réserver en récurrence (tous les mardis) | PAR | | `restauration.reservation` | R4 |
| RES-144 | Choisir son plat la veille | ELV, PAR | | `restauration.choix` | R4 |
| RES-145 | Réserver tardivement (si le règlement le prévoit) | PAR | tarif majoré au 24 | `restauration.reservation` | R4 |
| RES-146 | Réserver ou annuler pour une famille au guichet | ADF, CPE · `restauration.inscrire` | trace de l'agent | idem | R4 |
| RES-147 | Voir ses réservations, l'heure limite, les repas comptés | PAR, ELV | | — | R4 |
| RES-148 | Recevoir un rappel « vous n'avez rien réservé pour la semaine prochaine » | PAR | 8 (U0) ; désactivable | `restauration.rappel_reservation` | R4 |
| RES-149 | Réserver pour un groupe (réunion, jury) | DIR, ADF | centre de coût | `restauration.reservation_groupe` | R7 |

### 20.4 Passages et badges
| RES | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RES-160 | Enregistrer un passage par badge ou QR (RES-05 de D11) (NC-02) | SLF · `restauration.passage` | R-RS9, R-RS10 | `restauration.passage` [F `eleve.repas_passage`] | R0, R2 |
| RES-161 | Émettre, désactiver, remplacer un badge | ADF · `restauration.badges` | R-RS13 | `restauration.badge_*` | R0 (unicité), R2 |
| RES-162 | Imprimer les cartes (QR, photo facultative) en lot | ADF | | — | R2 |
| RES-163 | Enregistrer un passage par recherche nominative (élève sans badge) (NC-03) | SLF | | idem 160 | R2 |
| RES-164 | Afficher le QR de restauration dans l'appli de l'élève | ELV | jeton tournant | — | R10 |
| RES-165 | Voir le bandeau consigne au passage (RES-08 de D11) (NC-04) | SLF, RST · `restauration.passage` | R-RS15, lecture journalisée | — | R0, R2 |
| RES-166 | Corriger un passage (annuler, réattribuer) avec motif | RST · `restauration.gerer` | R-RS11 | `restauration.passage_corrige` | R0 |
| RES-167 | Saisir un passage a posteriori (oubli, panne) avec motif | RST ; CPT au-delà du délai | S-4008 | `restauration.passage` | R0 |
| RES-168 | Lister les passages du jour, par service et point (NC-05) | RST, SLF · `restauration.lire` | | — | R2 |
| RES-169 | Servir un élève « orange » (hors droit) | SLF | S-4007 | `restauration.passage` (hors droit) | R2 |
| RES-170 | Enregistrer un passage d'un commensal (ticket) | SLF | | `restauration.passage_commensal` | R7 |
| RES-171 | Passer en mode hors ligne et synchroniser | appareil | R-RS19 | idem 160 | R10 |
| RES-172 | Voir la liste des consignes du jour (photos) avant le service | RST, SLF | journalisée | — | R2 |
| RES-173 | Imprimer la liste du jour de secours | SYS ; RST | S-4010 | — | R2 |
| RES-174 | Voir ses propres passages (famille, élève) (NC repris de `GET /eleves/{id}/repas/passages`) | PAR, ELV | | — | R4 |
| RES-175 | Contrôler les re-passages et les badges suspects | RST | | — | R2 |

### 20.5 Prévision, production, prestataire
| RES | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RES-180 | Voir la prévision du jour par service et plat (RES-07 de D11) (NC-06) | RST, CUI, DIR · `restauration.lire` | §13.1 | — | R3 |
| RES-181 | Figer les instantanés J-7, J-1, J0 | SYS | S-4009 | `restauration.prevision_figee` | R3 |
| RES-182 | Voir le coefficient de présence et l'appliquer (ou non) | RST | R-RS22 | — | R3 |
| RES-183 | Voir les quatre statuts pendant et après le service | RST, SLF | | — | R3 |
| RES-184 | Envoyer la commande au prestataire | RST · `restauration.gerer` | R-RS23 | `restauration.commande_prestataire` | R3 |
| RES-185 | Partager le comptage servi au prestataire | SYS ; RST | | `restauration.servi_jour` | R3 |
| RES-186 | Prévoir le dîner et le petit-déjeuner des internes (41) (NC-15) | SYS | | — | R3 |
| RES-187 | Exporter la prévision (CSV cuisine) [F export existant] | RST | sans donnée personnelle | — | R0 |
| RES-188 | Voir le nombre d'assiettes adaptées par consigne (sans nom) et les étiquettes | RST, CUI | R-RS18 | — | R3 |
| RES-189 | Rapprocher la facture du prestataire et le servi (48) | CPT | | — | R12 |
| RES-190 | Recevoir et publier les menus du prestataire (import) | RST | relecture | `restauration.menu_importe` | R5 |
| RES-191 | Voir l'historique prévu / servi | RST, DIR | | — | R3 |
| RES-192 | Alerter si l'effectif change fortement après J-1 (sortie annoncée tard) | SYS | seuil | `restauration.ecart_prevision` | R3 |

### 20.6 Paniers et sorties
| RES | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RES-195 | Recevoir une commande de sortie (42) et la confirmer (NC-17) | RST | boîte de la restauration | `restauration.commande_sortie_confirmee` | R8 |
| RES-196 | Recevoir le « qui mange » et ses avenants (NC-18) | RST | sans INE | — | R8 |
| RES-197 | Voir les paniers famille attendus du jour (consigne `repas_panier`) | RST, SLF | sans motif | — | R8 |
| RES-198 | Commander des paniers pour un examen ou une journée banalisée | DIR, ADF | | `restauration.commande_groupe` | R8 |
| RES-199 | Enregistrer un passage « panier famille » | SLF | tarif panier au 24 | `restauration.passage` | R8 |

### 20.7 Menus, allergènes, qualité
| RES | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RES-200 | Créer un plat (catégorie, 14 allergènes, végétarien, labels, origine, saison) (RES-06 de D11) | RST · `restauration.menus` | R-RS20 | `restauration.plat_modifie` | R5 |
| RES-201 | Composer le menu d'un jour (plats, options) | RST · `restauration.menus` | | `restauration.menu_brouillon` | R5 |
| RES-202 | Publier un menu (contrôles végétarien, allergènes, fréquences) | RST · `restauration.menus` | R-RS20, R-RS21 | `restauration.menu_publie` [F `repas.menu`] | R0 (identifiant), R5 |
| RES-203 | Modifier un menu publié (nouvelle version, motif, avis si allergènes changent) | RST | | `restauration.menu_modifie` | R5 |
| RES-204 | Dupliquer une semaine, un cycle de menus | RST | | — | R5 |
| RES-205 | Voir les menus (famille, élève, affichage, prestataire) | tous | aucune donnée personnelle | — | R5 |
| RES-206 | Filtrer les plats par allergène sur son appareil | PAR, ELV | R-RS17 (local) | — | R10 |
| RES-207 | Consulter le registre des menus publiés (3 mois au moins) | RST, DIR, contrôle | D. 230-26 | — | R5 |
| RES-208 | Saisir ou lire les parts EGalim de l'année (48) | RST, CPT | | `restauration.egalim_saisi` | R9 |
| RES-209 | Préparer et diffuser l'information annuelle EGalim et l'affichage | RST, DIR | L. 230-5-3 ; diffusion par le 17 | `information.*` | R9 |

### 20.8 Argent
| RES | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RES-210 | Publier au 24 les quantités du mois (RES-09 de D11) (NC-09) | SYS | R-RS29, R-RS30 | `restauration.consommations_mois` | R6 |
| RES-211 | Valider le volume du mois avant facturation (contrôle des variations) (NC-10) | CPT · `restauration.facturation` | 24 §11 contrôle du mois | `restauration.volume_valide` [F `repas.volume_valide` prévu] | R6 |
| RES-212 | Recalculer un mois après correction (avant émission ; après : avoir ou complément) | SYS ; CPT | | idem 210 | R6 |
| RES-213 | Voir les consommations d'un élève, expliquées | CPT, PAR (payeur) | | — | R6 |
| RES-214 | Exporter l'état mensuel « ce que ScolIA aurait facturé » pendant le doublon | CPT · `restauration.facturation` | R-RS31 | `restauration.export` | R6 |
| RES-215 | Choisir forfait ou réel (NC-08) | CPT, DIR · `restauration.facturation` ★ | règlement financier | `restauration.formule_modifiee` | R6 |
| RES-216 | Transmettre les repas des personnels (24, 49) | SYS | | `restauration.consommations_commensaux` | R7 |
| RES-217 | Débiter le porte-monnaie au passage (si activé) | SYS → 25 | R-RS27 | `paiements.*` | R13 |
| RES-218 | Produire l'état des droits constatés pour Op@le (public) | CPT · `restauration.facturation` | FX-74 | `restauration.export` | R12 |
| RES-219 | Décider d'une fin d'inscription pour impayé (direction, après médiation) | DIR · `restauration.gerer` ★ | R-RS27 | `restauration.inscription_close` | R6 |

### 20.9 Gaspillage, indicateurs, conformité
| RES | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RES-220 | Saisir les pesées d'un service | CUI, RST · `restauration.gerer` | | `restauration.pesee` | R9 |
| RES-221 | Voir les indicateurs (fréquentation, gaspillage par convive, prévu / servi, réservations non honorées) | RST, DIR · `restauration.lire` | agrégats | — | R9 |
| RES-222 | Produire le diagnostic gaspillage annuel | RST, DIR | L. 541-15-3 | `restauration.diagnostic` | R9 |
| RES-223 | Tenir le registre des dons (facultatif) | RST | | `restauration.don` | R9 |
| RES-224 | Publier les indicateurs au 47 (agrégés, seuil de 5) | SYS | R-RS24 | — | R9 |
| RES-225 | Recevoir le signal « ne mange plus » (facultatif, CPE seul) | SYS → CPE | S-4014 | `restauration.signal_cpe` | R9 |
| RES-226 | Répondre à une demande d'accès RGPD (passages, non-repas, préférences) | DIR (délégué) | CRS-44 | journal | R11 |
| RES-227 | Purger selon les durées | SYS | §22.3 | `restauration.purge` | R11 |
| RES-228 | Consulter le journal des lectures de consignes | DIR, délégué à la protection des données | | — | R2 |
| RES-229 | Tenir le registre des plats témoins et des températures (facultatif) | CUI | §5.7 | `restauration.hygiene` | R12 |

### 20.10 Système, reprise, IA
| RES | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RES-230 | Réagir aux événements du cœur (§6.4) | SYS | | — | R0, R1 |
| RES-231 | Reprendre les données de Next.js et de Charlemagne (grilles, passages non facturés, badges) | opérateur | §27 | rapport | R11 |
| RES-232 | Comparer chaque jour passages et prévision avec l'outil officiel pendant le parallèle | SYS | | rapport | R11 |
| RES-233 | IA : expliquer la prévision, proposer un menu, rédiger l'information EGalim | RST, DIR | §19 | — | R13 |
| RES-234 | IA : préparer un changement d'inscription au service (reprise de NC-13) | ADF | N2 | — | R13 |

---

## 21. Règles transverses (R-RS1 à R-RS32) [R]
| # | Règle | Lot |
|---|---|---|
| R-RS1 | **Pas de suppression** de passage, d'inscription, de non-repas, de menu publié : fait correctif ou version (E6). | R0 |
| R-RS2 | **Le régime est au 2, l'inscription au service au 40** ; l'un ne modifie jamais l'autre en douce (corrige DC-13) ; un changement de régime ferme ou ouvre l'inscription à sa date. | R0 |
| R-RS3 | **Inscriptions datées sans chevauchement** (contrainte d'exclusion) (corrige RR-06, DC-17). | R0 |
| R-RS4 | **Europe/Paris** pour tout jour ; horaires du service pour midi / soir ; aucun seuil codé (corrige DC-14). | R0 |
| R-RS5 | **Une seule fonction de droit** pour la prévision, le passage, la famille et la facture (corrige DC-07). | R1 |
| R-RS6 | **Non-repas automatiques** depuis les faits des socles 5, 12, 13, 39, 41, 42 ; cause codée ; idempotents. | R1 |
| R-RS7 | **Cause codée, jamais de motif libre** dans un non-repas ; aucun motif dans les événements (corrige RR-09). | R0 |
| R-RS8 | **Pas de réservation, d'annulation ni de non-repas pour un jour passé** par une famille ; heure limite lisible (corrige RR-02). | R0 |
| R-RS9 | **Passage daté au geste**, jamais dans le futur ; saisie a posteriori motivée dans le délai (corrige RR-01). | R0 |
| R-RS10 | **Un passage facturable** par convive, service et jour ; les suivants sont des re-passages (corrige DC-12). | R0 |
| R-RS11 | **Correction** d'un passage = fait motivé (annulation, réattribution) (corrige RR-07, DC-16). | R0 |
| R-RS12 | **Aucune biométrie.** | — |
| R-RS13 | **Badge** : identifiant aléatoire propre à la restauration, un seul actif par élève (unicité en base), stocké en empreinte, affiché tronqué, absent des événements (corrige RR-03, RR-07). | R0 |
| R-RS14 | **Le passage ne montre jamais** un impayé, une aide, une bourse, un quotient, un motif d'absence ou une sanction. | R2 |
| R-RS15 | **Santé** : seules les consignes du 34 (audience « restauration »), codées, sans source ; lecture journalisée ; santé désactivée = bandeau, **aucune saisie de remplacement** (corrige DC-03, RR-04). | R0 |
| R-RS16 | **Préférences non médicales** : catalogue fermé neutre, sans mention religieuse ; visibles de la seule restauration ; jamais un droit. | R7 |
| R-RS17 | **Filtre d'allergènes des familles** : local à l'appareil, jamais transmis. | R10 |
| R-RS18 | **Étiquette d'assiette adaptée** : prénom, classe, « assiette adaptée », jamais le motif ; listes de sortie sans INE. | R3 |
| R-RS19 | **Hors ligne** : liste du jour chiffrée, effacée en fin de service + 12 h ; consignes codées seulement. | R10 |
| R-RS20 | **Allergènes obligatoires** : un plat publié a ses 14 allergènes renseignés ; sinon le menu n'est pas publiable. | R5 |
| R-RS21 | **Menu végétarien hebdomadaire** contrôlé à la publication ; fréquences sur 20 repas en indicateur. | R5 |
| R-RS22 | **Coefficient de présence** affiché, appliqué seulement par le responsable. | R3 |
| R-RS23 | **Destinataires** de commandes et de listes : adresses déclarées par l'établissement, jamais fournies par le navigateur. | R3 |
| R-RS24 | **Indicateurs agrégés** sans nom ; aucun chiffre de consigne pour un groupe de moins de 5 élèves. | R9 |
| R-RS25 | **Périmètre au serveur** : un surveillant du self ne voit que le jour et le point de service où il est affecté. | R2 |
| R-RS26 | **Une lecture n'écrit jamais** (corrige DC-09) ; aucune DDL hors migrations (corrige DC-10). | R0 |
| R-RS27 | **Aucun enfant privé de repas par un automatisme** (solde, impayé, payeur manquant, badge) ; la fin d'inscription pour impayé est une décision écrite de la direction, après contact et proposition d'aide. | R2, R6 |
| R-RS28 | **Panne de la réservation** : délais prolongés automatiquement de la durée de l'incident. | R4 |
| R-RS29 | **Quantités, jamais montants** vers le 24 ; causes codées ; « exclusion » n'apparaît sur aucun document financier. | R6 |
| R-RS30 | **Idempotence** : chaque quantité porte une clé de source (élève, mois, service, version) (corrige DC-08). | R6 |
| R-RS31 | **Doublon** (D-034) : mode suivi, aucune émission ; état de comparaison mensuel. | R6 |
| R-RS32 | **Aucune décision par l'IA** ; l'IA ne lit ni consignes ni préférences nominatives. | R13 |

---

## 22. Droits et minimisation

### 22.1 Catalogue d'actions proposé (D-017) [R]
| Droit | Rôles modèles par défaut | Remarque |
|---|---|---|
| `restauration.lire` | responsable de restauration, direction, comptabilité, secrétariat, CPE | prévisions, inscriptions, causes codées |
| `restauration.passage` (remplace `repas.scanner`) | surveillant du self, responsable de restauration, CPE | jour et point affectés (R-RS25) |
| `restauration.gerer` (remplace `repas.gerer` pour le service) | responsable de restauration, direction | corrections, commandes, pesées |
| `restauration.inscrire` | secrétariat, CPE | inscriptions au service, non-repas codés, guichet |
| `restauration.badges` | secrétariat | |
| `restauration.menus` | responsable de restauration | |
| `restauration.facturation` ★ | comptabilité, direction | volumes, formules, exports ; **pas** au surveillant ni à l'accueil (corrige DC-01) |
| `restauration.parametrer` | direction, responsable de restauration | |
| Prestataire | rôle « prestataire de restauration » : prévisions agrégées, menus, consignes de l'audience si l'établissement l'a décidé | compte restreint |
| Familles, élèves | implicites (lien, scolarité) | soi ou son enfant |

`repas.gerer` et `repas.scanner` restent reconnus pendant R0–R1 (alias), puis sont remplacés. **Professeur, infirmerie, psychologue** n'ont **plus** la section « repas » du dossier au-delà de « demi-pensionnaire : oui / non » (corrige RR-09).

### 22.2 Ce que voit chacun
| Information | Surveillant du self | Responsable de restauration | Comptabilité | Secrétariat | CPE | Professeur | Prestataire | Titulaires / payeur | Élève |
|---|---|---|---|---|---|---|---|---|---|
| Droit du jour, plat choisi | ✔ (jour) | ✔ | ✔ | ✔ | ✔ | — | agrégé | ✔ | ✔ |
| Cause de non-repas (codée) | ✔ (jour) | ✔ | ✔ | ✔ | ✔ | — | — | ✔ | ✔ |
| Consignes (34) | ✔ (passage, liste du jour) | ✔ | — | — | — | — | selon décision | ✔ | ✔ |
| Préférence non médicale | — (sauf plat choisi) | ✔ | — | — | — | — | agrégée | ✔ | ✔ |
| Passages | ✔ (jour) | ✔ | ✔ | ✔ | ✔ | — | servi agrégé | ✔ | ✔ |
| Quantités du mois | — | ✔ | ✔ | — | — | — | — | payeur | — |
| Badge | tronqué | tronqué | — | ✔ | — | — | — | — | QR |

### 22.3 Conservation [R, soumis au DPO]
| Donnée | Durée | Ensuite |
|---|---|---|
| Passages détaillés | année scolaire + 1 an (contestation ; prescription de 2 ans des créances au 24) | agrégés (quantités mensuelles conservées avec la pièce au 24, 10 ans) |
| Réservations, non-repas | idem | |
| Inscriptions au service | scolarité + 1 an | |
| Préférences non médicales | jusqu'au retrait ou au départ | supprimées |
| Lectures de consignes (journal) | 1 an | |
| Menus publiés (registre) | 1 an (3 mois exigés) | supprimés |
| Pesées, indicateurs | 5 ans (agrégats) | |
| Badges désactivés | 1 an | supprimés |

---

## 23. Événements
**Émis** (outbox ; charges minimales, sans badge, sans motif) : `restauration.service_modifie`, `restauration.point_modifie`, `restauration.formule_modifiee`, `restauration.prestataire_modifie` / `_acces`, `restauration.appareil_enregistre`, `restauration.incident_reservation`, `restauration.inscription_ouverte` / `_modifiee` / `_close`, `restauration.demande_inscription`, `restauration.non_repas` / `_annule`, `restauration.reservation` / `_annulee` / `_groupe`, `restauration.choix`, `restauration.rappel_reservation`, `restauration.passage` / `_corrige` / `_commensal`, `restauration.badge_emis` / `_desactive`, `restauration.prevision_figee`, `restauration.commande_prestataire`, `restauration.servi_jour`, `restauration.ecart_prevision`, `restauration.commande_sortie_confirmee`, `restauration.commande_groupe`, `restauration.plat_modifie`, `restauration.menu_brouillon` / `_publie` / `_modifie` / `_importe`, `restauration.egalim_saisi`, `restauration.consommations_mois`, `restauration.volume_valide`, `restauration.consommations_commensaux`, `restauration.export`, `restauration.pesee`, `restauration.diagnostic`, `restauration.don`, `restauration.signal_cpe`, `restauration.purge`, `restauration.hygiene`, `restauration.solde_bas` (avec le 25).
Les événements existants (`repas.activite_created`, `eleve.grille_repas_posed` / `_close`, `eleve.dispense_repas_posed` / `_annulee`, `eleve.repas_passage`, `repas.menu`) restent émis pendant R0–R1, **sans** motif ni badge, puis sont remplacés.

**Consommés** : `inscription.validee`, `inscription.desistement` (3) ; `eleve.regime_changed`, `scolarite.depart_enregistre`, `eleve.decede` (2) ; `referentiel.fermeture_exceptionnelle` (5) ; `assiduite.absence_annoncee`, `assiduite.absence_modifiee` (12) ; `discipline.exclusion` (13) ; `sante.consigne_publiee` / `_retiree` (34) ; `stage.periode_ouverte` / `_close` (39) ; `internat.absence_annoncee`, `internat.attendu_modifie`, `internat.effectif_repas` (41) ; `sorties.participant_pret` / `_desiste`, `sorties.restauration_commandee` / `_avenant` (42).

---

## 24. Écrans et mobile
**Web (Leptos)**
1. **Aujourd'hui** (tuiles) : effectif par service et plat, instantanés, écart, consignes du jour (nombre par code), paniers attendus, commandes de sortie, pesée à saisir.
2. **Self** (tablette, plein écran) : lecture badge / QR / recherche ; carte verte / orange ; bandeau consigne ; compteur ; mode hors ligne visible.
3. **Menus** : calendrier de la semaine, plats glissés, allergènes en pastilles, contrôles (végétarien, allergènes, fréquences), publication, versions.
4. **Inscriptions** : liste des élèves avec formule et jours, incohérences, changements en lot.
5. **Comptabilité** : volume du mois, variations, explication par élève, état du doublon.
6. **Indicateurs** : fréquentation, gaspillage, EGalim, diagnostic.
7. **Paramètres** : services, points, formules, heures limites, prestataire, lecteurs.
8. **Famille** : menus (avec filtre local), calendrier des repas (comptés, annulés, pris), réserver / annuler / choisir, solde (si porte-monnaie), facture (24).

**Mobile (appli unique)** : élève — QR, menu, choix du plat, réservation ; famille — menus, **réserver / annuler en deux gestes**, repas pris, solde ; surveillant — self sur téléphone en secours ; responsable — effectif et consignes du jour.

---

## 25. Modèle de données (sans JSON, D-036 ; E1–E14)
Règles communes : comme au 41 §24 (UUID générés par l'application, `etablissement_id NOT NULL`, `UNIQUE (etablissement_id, id)`, FK composites, `cree_le` / `modifie_le`, faits en ajout seul, RLS, aucun `jsonb` / `bytea` / tableau d'identifiants, registre E12, aucun nom dans un objet ou un journal applicatif).

```
-- R0 (2800–2802) : correctifs de l'existant
ent_repas_activite + UNIQUE (etablissement_id, id), modifie_le, heure_debut TIME, heure_fin TIME,
  kind étendu CHECK (dejeuner|diner|petit_dejeuner|gouter_internat|midi|soir|etude|garderie|autre)   -- anciens codes en lecture
ent_eleve_grille_repas + formule_id NULL, modifie_le, FK composite activite, EXCLUDE (etablissement_id, eleve_id,
  activite_id, daterange(date_debut, date_fin, '[]') WITH &&)                                         -- 2800
ent_eleve_dispense_repas + cause CHECK (absence_annoncee|absence_justifiee|sortie_scolaire|stage|internat_absent|
  exclusion|fermeture|panier_famille|annulation_famille|decision_etablissement|autre), annulee_le, annulee_par ;
  motif libre gelé (lecture seule, purgé à la reprise)                                                -- 2801 ; devient ent_repas_non_repas (vue)
ent_repas_passage + FK composite élève (NOT VALID puis VALIDATE), FK composite activite, par_user_id UUID (remplace actor_user_id TEXT),
  badge_empreinte TEXT (remplace badge_lu en clair), statut CHECK (du|hors_droit|re_passage|panier_famille),
  correction_de UUID NULL, motif_correction_code NULL, saisi_a_posteriori BOOLEAN, a TIMESTAMPTZ   -- 2802
ent_eleve_identifiant : index unique partiel « un badge BADGE actif par établissement » ; badges contestés refusés au passage
ent_menu_cantine : identifiant d'événement corrigé ; gelée à partir de R5 (lecture seule)

-- R1 (2803–2804) : services, formules, non-repas
ent_repas_point_service (id, etablissement_id, lieu_id → ent_lieu / ent_salle (5), libelle, actif, cree_le, modifie_le)
ent_repas_formule (id, etablissement_id, code CHECK (forfait|reservation|ticket), activite_id, ouverte_dp BOOLEAN,
  ouverte_internes BOOLEAN, ouverte_externes BOOLEAN, ouverte_commensaux BOOLEAN, heure_limite_reservation TIME NULL,
  veille BOOLEAN, heure_limite_annulation TIME NULL, annulation_forfait_permise BOOLEAN, cree_le, modifie_le)
ent_repas_non_repas (id, etablissement_id, eleve_id, activite_id, jour DATE, cause (catalogue R0),
  source_socle CHECK (5|12|13|34|39|41|42|famille|etablissement), source_id UUID NULL, pose_par NULL, annule_le NULL,
  cree_le, UNIQUE (etablissement_id, eleve_id, activite_id, jour, cause, source_id))                 -- idempotent
-- vue ent_repas_droit_v (élève × jour × service → statut, cause principale)

-- R2 (2805) : badges, appareils, lectures
ent_repas_badge (id, etablissement_id, eleve_id NULL, commensal_id NULL, empreinte TEXT, fin_affichee CHAR(4),
  actif_du, actif_au NULL, cause_fin CHECK (perdu|remplace|depart|autre) NULL, cree_le,
  CHECK ((eleve_id IS NULL) <> (commensal_id IS NULL)), UNIQUE (etablissement_id, empreinte))
ent_repas_appareil (id, etablissement_id, point_id, libelle, enregistre_par, actif, cree_le, modifie_le)
ent_repas_lecture_consigne (id, etablissement_id, user_id, eleve_id NULL, contexte CHECK (passage|liste_jour|export), a) -- ajout seul

-- R3 (2806) : prévision, prestataire
ent_repas_prevision (id, etablissement_id, activite_id, jour, instant CHECK (j_moins_7|j_moins_1|j0|manuel), calcule_le, cree_le)
ent_repas_prevision_ligne (id, etablissement_id, prevision_id, plat_option_id NULL, categorie CHECK (eleves|internes|commensaux|paniers_sortie|assiettes_adaptees),
  consigne_code NULL, nombre INTEGER)
ent_repas_prestataire (id, etablissement_id, raison_sociale, courriel_commande, heure_commande TIME, cree_le, modifie_le)
ent_repas_commande_prestataire (id, etablissement_id, prestataire_id, prevision_id, envoyee_le, envoyee_par, cree_le)
ent_repas_servi (id, etablissement_id, activite_id, jour, servis INTEGER, comptes_le)

-- R4 (2807) : réservations
ent_repas_reservation (id, etablissement_id, eleve_id, activite_id, jour, choix_option_id NULL, par_user_id, qualite CHECK (titulaire|eleve|personnel),
  tardive BOOLEAN, statut CHECK (confirmee|annulee), annulee_le NULL, cree_le, modifie_le,
  UNIQUE (etablissement_id, eleve_id, activite_id, jour) WHERE statut = 'confirmee')
ent_repas_reservation_recurrente (id, etablissement_id, eleve_id, activite_id, jour_semaine SMALLINT, du, au NULL, cree_le)
ent_repas_incident (id, etablissement_id, debut, fin, declare_par, cree_le)

-- R5 (2808–2809) : plats, menus, allergènes
ent_repas_allergene (code PRIMARY KEY, libelle, rang)                     -- 14 codes INCO, table de référence globale (E12)
ent_repas_label (code PRIMARY KEY, libelle)                               -- bio, label_rouge, aop, igp, peche_durable, fait_maison, saison
ent_repas_plat (id, etablissement_id, libelle, categorie CHECK (entree|plat|accompagnement|laitage|dessert|autre), vegetarien BOOLEAN,
  origine_viande TEXT NULL, actif, cree_le, modifie_le)
ent_repas_plat_allergene (etablissement_id, plat_id, allergene_code, presence CHECK (present|traces|absent))  -- 14 lignes par plat publié
ent_repas_plat_label (etablissement_id, plat_id, label_code)
ent_repas_menu (id, etablissement_id, activite_id, jour, point_id NULL, UNIQUE (etablissement_id, activite_id, jour, point_id))
ent_repas_menu_version (id, etablissement_id, menu_id, version SMALLINT, etat CHECK (brouillon|publie|remplace), publie_le, publie_par,
  motif_modification CHECK (rupture|livraison|erreur|autre) NULL, cree_le)
ent_repas_menu_option (id, etablissement_id, menu_version_id, code CHECK (a|b|vegetarien|defaut), libelle)
ent_repas_menu_ligne (id, etablissement_id, menu_version_id, option_id NULL, plat_id, rang SMALLINT)
-- reprise : ent_menu_cantine (074) → plats « reprise » avec allergènes « non renseigné » (non publiables tels quels)

-- R6 (2810) : quantités vers le 24
ent_repas_consommation_mois (id, etablissement_id, eleve_id NULL, commensal_id NULL, mois DATE, activite_id, version SMALLINT,
  dus SMALLINT, reserves SMALLINT, servis SMALLINT, hors_droit SMALLINT, paniers SMALLINT, re_passages SMALLINT,
  calcule_le, valide_le NULL, valide_par NULL, cle_source TEXT UNIQUE (etablissement_id, cle_source))
ent_repas_consommation_non_repas (etablissement_id, consommation_id, cause, nombre SMALLINT)

-- R7 (2811) : commensaux, préférences
ent_repas_commensal (id, etablissement_id, personnel_id NULL (6), nom_affiche TEXT NULL, qualite CHECK (personnel|stagiaire|intervenant|invite|eleve_convention),
  du, au NULL, centre_cout TEXT NULL, convention_id NULL, cree_le, modifie_le)
ent_repas_preference (id, etablissement_id, eleve_id, code CHECK (vegetarien|sans_viande|sans_porc|sans_poisson), declare_par, du, au NULL, cree_le)

-- R8 (2812) : paniers et commandes de groupe
ent_repas_commande_groupe (id, etablissement_id, nature CHECK (sortie|examen|journee|reunion), sortie_id NULL (42), jour, nombre, livraison_lieu, livraison_heure, confirmee_le NULL, cree_le)
ent_repas_commande_groupe_ligne (etablissement_id, commande_id, type CHECK (panier|panier_vegetarien|panier_sans_porc|repas_self|boisson|gouter), consigne_code NULL, nombre)

-- R9 (2813) : gaspillage, EGalim
ent_repas_pesee (id, etablissement_id, activite_id, jour, nature CHECK (assiettes|prepare_non_servi|pain|autre), grammes INTEGER, saisi_par, cree_le)
ent_repas_egalim (id, etablissement_id, annee SMALLINT, part_durable_pct NUMERIC(5,2), part_bio_pct NUMERIC(5,2), source CHECK (achats|teledeclaration), saisi_par, cree_le)
ent_repas_diagnostic (id, etablissement_id, annee, objet_id → ent_objet (500), cree_le)
ent_repas_don (id, etablissement_id, jour, association, kilogrammes NUMERIC, cree_le)

-- R11 (2814) : reprise ; R12 (2815) : hygiène facultative ; 2816–2818 réserve ; 2819 : retrait des colonnes héritées (sur décision)
ent_repas_reprise_correspondance (id, etablissement_id, source CHECK (next|charlemagne), cle_source, cible_table, cible_id, lot_id, cree_le)
ent_repas_plat_temoin (id, etablissement_id, activite_id, jour, preleve_par, conserve_jusqu_au DATE, cree_le)
```

**Registre (E12)** : tables d'établissement en export `simple` ; `ent_repas_lecture_consigne`, `ent_repas_preference` en `double` ; `ent_repas_allergene`, `ent_repas_label` en `globale_reference` ; **E9** : la commande au prestataire part par la file d'envoi du cœur (à neutraliser à la restauration) ; **E11** : l'empreinte du badge est un HMAC avec une clé inscrite à l'inventaire des clés (identifiant de clé dans la colonne) [R].

---

## 26. Lots

### 26.1 Lot R0 — correctifs urgents de l'existant Rust
| # | Correctif | Défaut | Migration |
|---|---|---|---|
| 1 | Jour de passage = aujourd'hui (Europe/Paris), dans les horaires du service ; saisie a posteriori motivée (délai) ; refus si le service est fermé (calendrier) | RR-01 | 2802 |
| 2 | Dispenses familles : ni jour passé, ni après l'heure limite ; motif **codé** (cause), texte libre gelé ; motif retiré des événements et des lectures | RR-02, RR-09 | 2801 |
| 3 | Badge : unicité en base d'un badge actif ; refus d'un badge contesté ; contrôle du statut de l'élève (parti, désactivé) au passage ; empreinte au lieu du numéro brut ; badge retiré de la charge des événements | RR-03, RR-07 | 2802 |
| 4 | Consignes du 34 (audience « restauration ») dans la réponse du passage et dans la liste du jour ; lecture journalisée ; bandeau « santé désactivée » | RR-04 | 2802 (journal) |
| 5 | Annuler, réattribuer un passage (fait correctif) | RR-07 | 2802 |
| 6 | FK composites (passage → élève, activité ; grille, dispense → activité) ; `actor_user_id` en UUID | RR-08 | 2800, 2802 |
| 7 | Grilles : contrainte d'exclusion de chevauchement ; réaction à `eleve.regime_changed` qui ferme ou ouvre l'inscription | RR-06 | 2800 |
| 8 | Menus : identifiant d'événement = identifiant en base | RR-05 | — |
| 9 | Section « repas » du dossier réduite à « demi-pensionnaire : oui / non » pour professeur, infirmerie, psychologue | RR-09 | — |
| 10 | Tests : isolation, droits, jour futur et passé, badge dupliqué, chevauchement, fuseau | — | — |

### 26.2 Lots suivants
| Lot | Contenu | Dépend de | Critère de fin | Migrations |
|---|---|---|---|---|
| **R1** Droit au repas | services et formules, inscriptions datées depuis le 3, non-repas automatiques (5, 12, 13, 39, 41, 42), vue de droit, explication | R0 ; 2, 3, 5, 12 | R-RS5, R-RS6 testés ; DC-07 impossible | 2803–2804 |
| **R2** Self | badges et cartes, QR, recherche, tablette, carte verte / orange, bandeau consigne, re-passages, liste de secours, périmètre | R1 ; 34 (N-1 ou bandeau) | passage < 1 s ; R-RS14, R-RS15 testés | 2805 |
| **R3** Prévision et production | prévision par plat, instantanés, coefficient, quatre statuts, prestataire (commande, compte restreint, servi), étiquettes, dîner des internes | R1 ; 41 | parité NC-06, NC-07, NC-15 | 2806 |
| **R4** Familles | réservations, annulations, choix du plat, récurrences, guichet, rappels, calendrier famille, incident de réservation | R1 ; 8 | heure limite testée (R-RS8, R-RS28) | 2807 |
| **R5** Menus | plats, 14 allergènes, labels, menus versionnés, contrôles, registre, import prestataire, diffusion | R0 | R-RS20, R-RS21 testés | 2808–2809 |
| **R6** Argent | quantités au 24, validation du mois, recalcul, explication, **mode suivi** et état de comparaison, décision de fin d'inscription pour impayé | R1 ; 24 (F0, F4) | R-RS29–R-RS31 testés ; DC-08 impossible | 2810 |
| **R7** Commensaux et préférences | personnels, stagiaires, invités, conventions, réservations de groupe, préférences non médicales | R2 ; 6, 49 | | 2811 |
| **R8** Paniers et sorties | commandes de sortie (42), avenants, paniers famille (34), commandes de groupe | R3 ; 42 | parité NC-17, NC-18 (réception) | 2812 |
| **R9** Gaspillage et indicateurs | pesées, diagnostic, dons, EGalim et information annuelle (17), indicateurs (47), signal CPE facultatif | R3 ; 17, 47, 48 | | 2813 |
| **R10** Mobile | QR de l'élève, réservation en deux gestes, filtre d'allergènes local, self hors ligne | R2, R4, R5 ; 11 | service complet en mode avion | — |
| **R11** Reprise et parallèle | reprise Next.js (grilles, passages non facturés) et Charlemagne (badges, inscriptions), comparaison quotidienne, purges, accès RGPD | R1–R6 | rapport sans perte | 2814 |
| **R12** Public et hygiène | droits constatés Op@le, régie (25), rapprochement prestataire (48), plats témoins et températures (facultatif) | R6 ; 25, 48 | | 2815 |
| **R13** Porte-monnaie et IA | porte-monnaie prépayé avec le 25 (désactivé par défaut) ; capacités IA §19.3 | R6 ; 25 ; `ent-ia` | aucun refus de repas automatique (R-RS27) | — |

**Parcours minimal de la rentrée 2027** : R0, R1, R2, R3, R4, R5, R6 (mode suivi), R8, R10, R11. Ordre recommandé : R0 → R1 → R2 → R5 → R3 → R4 → R6 → R8 → R10 → R11 → R7 → R9 → R12 → R13.

### 26.3 Plage de migrations
| N° | Lot | Objet |
|---|---|---|
| 2800 | R0 | FK composites, uniques, `modifie_le`, exclusion de chevauchement des grilles |
| 2801 | R0 | causes codées des dispenses ; gel du motif libre |
| 2802 | R0 | passages : FK, statut, correction, empreinte de badge, saisie a posteriori ; unicité du badge actif ; journal des lectures de consignes |
| 2803 | R1 | points de service, formules, horaires |
| 2804 | R1 | non-repas ; vue de droit |
| 2805 | R2 | badges, appareils |
| 2806 | R3 | prévisions, prestataire, servi |
| 2807 | R4 | réservations, récurrences, incidents |
| 2808 | R5 | allergènes, labels, plats |
| 2809 | R5 | menus versionnés, options, lignes ; reprise de 074 |
| 2810 | R6 | consommations du mois |
| 2811 | R7 | commensaux, préférences |
| 2812 | R8 | commandes de groupe |
| 2813 | R9 | pesées, EGalim, diagnostic, dons |
| 2814 | R11 | correspondances de reprise |
| 2815 | R12 | plats témoins |
| 2816–2818 | réserve | |
| 2819 | — | retrait des colonnes et tables héritées (`badge_lu`, motif libre, `ent_menu_cantine`), sur décision et sauvegarde vérifiée (E8) |

---

## 27. Reprise et bascule
1. **Next.js** (M14) : passages **non encore facturés** à conserver, facturés perdables (la facture fait foi) [T0] ; grille JSON → inscription au service datée de la reprise (midi, soir ; étude et garderie au 43 ; « sort seul » au 14) ; **extraits santé de portée cantine non repris** (34, après HDS, sur décision du DPO) ; mode de facturation → réglage du 24.
2. **Charlemagne** (si c'est lui qui tient le badge et la demi-pension, Q-RS1) : inscriptions et jours, numéros de badge (convertis en empreintes, numéros physiques conservés pour ne pas rééditer les cartes), tarifs (au 24).
3. **Rust existant** : grilles 017 → inscriptions (formule `forfait`) ; dispenses 034 → non-repas (cause `autre` + texte gelé, à recoder par le secrétariat si nécessaire) ; menus 074 → plats « reprise » non publiables sans allergènes.
4. **Bascule** [R, Q-RS2] : **été 2027**, en même temps que l'internat (dîner des internes) ; parallèle de répétition en juin (prévision et passages comparés chaque jour à l'outil officiel, RES-232) ; **un seul outil officiel par service et par jour** (R-RS33, ajout personas).
5. **Pendant l'année de doublon** : facturation par Charlemagne (D-034) ; ScolIA en mode suivi (R-RS31).

---

## 28. Tests et recette
- Isolation (matrice entre établissements) ; RLS stricte ; périmètre du surveillant (jour, point).
- Droits : surveillant et accueil **refusés** sur la facturation (DC-01) ; professeur refusé sur les causes et consignes.
- Temps : passages autour de minuit, changements d'heure, saisies a posteriori ; heure limite à la minute.
- Règle de droit : une table de cas (une centaine) jouée identiquement par la prévision, le passage, la famille et la facture.
- Santé : aucune colonne, aucun événement, aucun export du 40 ne contient de diagnostic ni de motif libre (test de schéma et de charge utile).
- Allergènes : impossibilité de publier un plat non renseigné.
- Parité : chaque NC rejoué contre Next.js ; chaque DC et RR en test négatif.
- Hors ligne : service complet en mode avion.

---

## 29. Réglages, questions et décisions

### 29.1 Réglages signalés (D-023)
| S | Réglage | Défaut [R] |
|---|---|---|
| S-4000 | Services ouverts et horaires | déjeuner ; dîner et petit-déjeuner si internat |
| S-4001 | Formules et publics | forfait pour DP et internes ; ticket fermé aux externes |
| S-4002 | Jours proposés à l'inscription | 4 ou 5 jours |
| S-4003 | Heures limites de réservation et d'annulation | veille 18 h (déjeuner) ; jour même 16 h (dîner) |
| S-4004 | Annulation d'un repas du forfait par la famille | non (seules les causes automatiques) |
| S-4005 | Réservation par l'élève mineur | non (titulaires) ; oui au lycée si l'établissement le décide |
| S-4006 | Identification | badge et QR ; liste toujours possible |
| S-4007 | Conduite au passage « orange » | servir, marquer hors droit |
| S-4008 | Délai de saisie a posteriori | 7 jours |
| S-4009 | Instantanés de prévision | J-7, J-1 18 h, J0 10 h |
| S-4010 | Liste de secours imprimée | J-1 18 h |
| S-4011 | Plat par défaut | option A |
| S-4012 | Préférences non médicales | catalogue activé ; recommandation « choix du plat » |
| S-4013 | Durée du registre des menus | 1 an |
| S-4014 | Signal « ne mange plus » au CPE | désactivé |
| S-4015 | Photo au passage | oui |
| S-4016 | Rappel de réservation hebdomadaire | oui (désactivable par la famille) |
| S-4017 | Seuil d'écart de prévision qui alerte | 10 % |
| S-4018 | Compte prestataire : consignes visibles | non (liste exportée par l'infirmière) |
| S-4019 | Porte-monnaie prépayé | désactivé |

### 29.2 Appliquées par défaut (D-031)
| Q | Question | Recommandation appliquée |
|---|---|---|
| Q-RS3 | Biométrie ? | Jamais (R-RS12) |
| Q-RS5 | Les extraits santé libres de Next.js ? | Non repris ; consignes du 34 |
| Q-RS8 | Grille JSON ? | Inscriptions datées typées |
| Q-RS9 | Passages du portail ? | Socle 14 |
| Q-RS10 | Étude, garderie de la grille ? | Socle 43 |
| Q-RS11 | Qui choisit forfait ou réel ? | Comptabilité ou direction, ★ |
| Q-RS12 | Allergènes obligatoires à la publication ? | Oui (R-RS20) |
| Q-RS13 | Menu sans végétarien dans la semaine ? | Publiable avec avertissement tracé |
| Q-RS14 | Préférences « halal », « casher » ? | Non ; catalogue neutre |
| Q-RS15 | Filtre d'allergènes côté serveur ? | Non : local |
| Q-RS16 | Élève sans droit au passage ? | Servi, marqué hors droit |
| Q-RS17 | Exclusion pour impayé automatique ? | Jamais (R-RS27) |
| Q-RS18 | Porte-monnaie ? | Désactivé par défaut |
| Q-RS19 | Prestataire : adresse libre ? | Non : déclarée (R-RS23) |
| Q-RS20 | Coefficient de présence automatique ? | Affiché, appliqué par le responsable |
| Q-RS21 | Durées de conservation | §22.3, à confirmer avec le DPO |
| Q-RS22 | Facturation pendant le doublon | Mode suivi |
| Q-RS23 | Signal « ne mange plus » | Désactivé, CPE seul |
| Q-RS24 | Dispenses existantes à motif libre | Gelées, recodées par le secrétariat |
| Q-RS25 | Anciennes actions `repas.*` | Alias pendant R0–R1 |
| Q-RS26 | Plats témoins dans ScolIA | Facultatif (R12) |
| Q-RS27 | Information EGalim | Préparée, diffusée par le 17 |

### 29.3 Soumis à Florian
| Q | Question | Recommandation |
|---|---|---|
| **Q-RS1** | Qui tient aujourd'hui badges, passages et demi-pension (Charlemagne, prestataire, Next.js) ? | Décide la reprise et le régime de la rentrée 2027 (usage réel ou parallèle) |
| **Q-RS2** | Moment de la bascule | Été 2027, avec l'internat ; parallèle de répétition en juin |
| **Q-RS4** | Achat de lecteurs de badge et de tablettes pour le self | Lecteur USB ou NFC standard ; à chiffrer |
| **Q-RS6** | Règle de l'établissement en cas d'impayé de restauration | Aucune exclusion sans décision écrite de la direction après médiation |
| **Q-RS7** | Élèves d'autres établissements (internat mutualisé, conventions) | Commensaux sous convention, facturés à l'établissement d'origine |
| **Q-RS28** | Prestataire : nom, contrat, ce qu'il doit recevoir (effectifs, consignes) | à fournir ; consignes par export de l'infirmière par défaut |
| **Q-RS29** | Activer le porte-monnaie (contrat de paiement du 25) | non pour la rentrée 2027 |

---
## 30. Ajouts personas (`PERSONAS_40.md`, 2026-11-11) — **l'emporte en cas de contradiction**

### 30.1 Quatre bloquants tranchés (D-031)
- **A — Santé désactivée à la bascule** : R-RS33. Le self ne bascule dans ScolIA que si le niveau N-1 du 34 est actif (consignes codées, avis écrit du DPO), **ou** si l'infirmière publie chaque semaine une liste papier signée affichée en cuisine et que la direction l'atteste dans ScolIA (RES-235 ter devient la case d'attestation de RES-242). Jalon N-1 **avant juin 2027** (aligné sur le 34).
- **B — Deux outils de pointage pendant le doublon** : R-RS34, RES-235, RES-236 ; tranché par Q-RS1.
- **C — Personnel du prestataire à la ligne de service** : R-RS35, RES-249 ; **S-4018 modifié** : « liste exportée par l'infirmière **et** bandeau au passage pour les personnes du prestataire désignées par la direction, sous contrat de sous-traitance (Q-RS30) ».
- **D — Menu modifié le jour même** : R-RS36, RES-248.

### 30.2 Actions ajoutées (RES-235 à RES-249)
| RES | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| RES-235 | Exporter chaque jour les passages au format de l'outil qui facture (Charlemagne, prestataire) | SYS ; CPT | R-RS34 | `restauration.export` | R6 |
| RES-236 | Importer chaque jour les passages de l'outil officiel qui pointe (ScolIA ne pointe pas alors) | SYS ; CPT | R-RS34 | `restauration.passages_importes` | R11 |
| RES-237 | Définir les **créneaux** d'un service (premier, second) et leur affectation (niveau, classe, fin des cours du 10) ; prévision et passage par créneau ; élève au mauvais créneau servi et signalé (M-4001) | RST, DIR · `restauration.parametrer` | S-4022 | `restauration.creneau_modifie` | R3 |
| RES-238 | Valider contradictoirement le servi du mois avec le prestataire (M-4008) | CPT ; prestataire | | `restauration.servi_valide` | R12 |
| RES-239 | Comparer le doublon **élève par élève**, sur les droits et remises, avec l'explication des écarts (M-4015, M-4016) | CPT · `restauration.facturation` | R-RS31 | `restauration.export` | R6 |
| RES-240 | Proposer l'aide (fonds social, 26) au payeur à la deuxième relance du 25 ; informer la direction (M-4018) | SYS → 25, 26 | S-4020 | `restauration.aide_proposee` | R6 |
| RES-241 | Préparer la commission des menus (indicateurs, plaintes, propositions) (M-4019) | RST, DIR | agrégats | — | R9 |
| RES-242 | Accuser la prise en compte des consignes par la restauration (« vu le … »), visible de l'infirmière sans nom ; attester la liste papier quand N-1 n'est pas actif (M-4022, bloquant A) | RST, DIR | R-RS33 | `restauration.consignes_vues` | R2 |
| RES-243 | Rétablir les repas quand la source d'un non-repas disparaît (sortie annulée ou reportée, absence retirée, stage annulé) et alerter la cuisine avant l'heure de lancement (M-4035) | SYS | R-RS38 | `restauration.non_repas_annule`, `restauration.ecart_prevision` | R1 |
| RES-244 | Recevoir le rapport quotidien des saisies a posteriori et des corrections (M-4040) | RST | S-4024 | — | R2 |
| RES-245 | Publier la notice d'information de la restauration (identification, photo, consignes, préférences, durées, prestataire) (M-4042) | DIR | modèle fourni | `information.*` | R11 |
| RES-246 | Demander une remise sur justificatif (famille), pièce au 7, décision au 24 (M-4045) | PAR ; CPT | cause « sur demande » | `restauration.demande_remise` | R6 |
| RES-247 | Reprendre les numéros des cartes existantes (empreintes) et vérifier un échantillon au self (M-4048) | opérateur ; RST | | rapport | R11 |
| RES-248 | Recalculer les assiettes adaptées à toute publication ou modification de menu ; alerter cuisine et responsable ; confirmation explicite le jour même (bloquant D) | SYS ; RST | R-RS36 | `restauration.assiettes_adaptees_modifiees` | R5 |
| RES-249 | Désigner les personnes du prestataire autorisées au bandeau du self (contrat de sous-traitance renseigné) (bloquant C) | DIR · `restauration.parametrer` ★ | R-RS35 | `restauration.prestataire_acces` | R2 |

### 30.3 Règles ajoutées (R-RS33 à R-RS38)
| # | Règle | Lot |
|---|---|---|
| R-RS33 | **Pas de bascule du self sans consignes** : N-1 actif, ou liste papier signée de l'infirmière attestée chaque semaine par la direction. | R2, R11 |
| R-RS34 | **Un seul pointage physique** par service et par jour ; l'autre outil reçoit un export ou fournit un import. | R6, R11 |
| R-RS35 | **Personnes du prestataire désignées** nommément, sous contrat de sous-traitance et engagement de confidentialité : bandeau au passage et étiquettes seulement, lectures journalisées. | R2 |
| R-RS36 | **Menu publié ou modifié = assiettes adaptées recalculées** et alerte ; modification le jour même confirmée explicitement. | R5 |
| R-RS37 | **Écran double au self** : la face visible de l'élève ne montre ni consigne, ni droit, ni dette. | R2 |
| R-RS38 | **Un non-repas suit la vie de sa source** : si la sortie, l'absence ou le stage disparaît ou change, le repas est rétabli et la cuisine prévenue. | R1 |

Règles **précisées** : R-RS14 (rien de sensible non plus dans l'**appli de l'élève** : le solde n'apparaît qu'aux payeurs) ; R-RS16 (une préférence n'est **jamais présumée** : pas de case pré-cochée ni de reprise sans confirmation de la famille ; exportée seulement en nombre, sauf type de panier d'une sortie au 42) ; R-RS25 (le droit du jour vient aussi du **poste « self » du planning** du 6, pour un remplaçant).

### 30.4 Cas limites ajoutés (CRS-45 à CRS-56)
| # | Situation | Traitement |
|---|---|---|
| CRS-45 | Terminale au premier service parce que son cours est annulé | Servi ; signalé « autre créneau » ; aucune pénalité |
| CRS-46 | Même carte passée deux fois à 10 secondes | Second passage « re-passage » ; photo du titulaire à l'écran ; signal |
| CRS-47 | Sortie annulée à 8 h pour la pluie | Repas rétablis (R-RS38) ; cuisine alertée (+ effectif) ; paniers déjà préparés comptés en gaspillage « sortie annulée » |
| CRS-48 | Plat B remplacé à 9 h par un plat contenant de l'arachide | R-RS36 : assiettes adaptées recalculées ; alerte ; confirmation explicite |
| CRS-49 | Employé du prestataire non désigné à la ligne de service | Pas d'accès au bandeau ; l'AED désigné porte l'étiquette ; alerte à la direction si aucune personne désignée n'est en service |
| CRS-50 | N-1 désactivé et liste papier non attestée cette semaine | Bandeau rouge au responsable et à la direction « liste des consignes non attestée » ; le service continue |
| CRS-51 | Charlemagne pointe, ScolIA importe ; import du jour manquant | Prévision du lendemain avec « passages d'hier non reçus » ; aucun passage inventé |
| CRS-52 | Majeur payé par ses parents qui ne veut pas qu'ils voient ses repas | Le **payeur** voit le détail facturable ; un titulaire non payeur ne le voit plus si le règlement le prévoit (**corrige CRS-27**) |
| CRS-53 | Famille qui demande « halal » | Non stocké ; proposition « choix du plat » ; aucune mention religieuse |
| CRS-54 | Remplaçant AED affecté au self pour la journée | Droit du jour par le poste du planning (6) ; fin du droit à la fin du poste |
| CRS-55 | Collège public : forfait trimestriel, élève malade 3 semaines avec certificat | Demande de remise sur justificatif (RES-246) ; décision au 24 selon le règlement départemental |
| CRS-56 | Panier préparé pour un élève finalement absent de la sortie | Gaspillage « préparé non servi » ; aucune facturation du panier sans règle |

### 30.5 Modèle de données complété (D-036)
```
ent_repas_creneau (id, etablissement_id, activite_id, code, libelle, heure_debut, heure_fin, ordre, cree_le, modifie_le)   -- 2803
ent_repas_creneau_affectation (id, etablissement_id, creneau_id, niveau_id NULL, classe_id NULL, selon_fin_cours BOOLEAN, du, au NULL)
ent_repas_passage + creneau_id NULL
ent_repas_frequence (code PRIMARY KEY, libelle)                 -- codes de l'arrêté du 30/09/2011, référence globale (E12) -- 2808
ent_repas_plat_frequence (etablissement_id, plat_id, frequence_code)
ent_repas_commensal + qualite_employeur CHECK (salarie_organisme|agent_public|autre), categorie_tarif TEXT NULL         -- 2811
ent_repas_prestataire + contrat_sous_traitance_le DATE NULL
ent_repas_prestataire_personne (id, etablissement_id, prestataire_id, nom_affiche, designe_par, designe_le, fin_le NULL, user_id NULL, cree_le)  -- 2805
ent_repas_attestation_consignes (id, etablissement_id, semaine_du DATE, mode CHECK (n1_actif|liste_papier), atteste_par, atteste_le)            -- 2805
ent_repas_demande_remise (id, etablissement_id, eleve_id, du, au, piece_id NULL (7), statut CHECK (deposee|transmise|decidee), cree_le, modifie_le) -- 2810
ent_repas_passage_import (id, etablissement_id, source CHECK (charlemagne|prestataire), jour, lignes INTEGER, recu_le, cree_le)                  -- 2814
```

### 30.6 Lots révisés
- **R2** reçoit RES-242, RES-244, RES-249 et R-RS33, R-RS35, R-RS37 ; **R3** reçoit RES-237 ; **R5** reçoit RES-248 ; **R6** reçoit RES-235, RES-239, RES-240, RES-246 ; **R11** reçoit RES-236, RES-245, RES-247 ; **R1** reçoit RES-243.
- **Condition de bascule** du parcours minimal : R-RS33 satisfaite (N-1 actif ou attestation hebdomadaire), Q-RS1 tranchée (R-RS34), Q-RS30 signée si le prestataire sert.

### 30.7 Défauts Next.js ajoutés
**DC-23** (prévision et facturation lisibles de tous les rôles du module, quantités par foyer exposées à l'accueil) et **DC-24** (invité en texte libre, sans durée) : `PERSONAS_40.md` §3.3 ; corrigés par §22.1 et §14.

---

## 31. Faits, hypothèses, contradictions
**Faits [F]**
- Next.js : environ 3 900 lignes pour la cantine (T0 annonçait 1 900) ; passages **uniquement manuels** ; aucune fonction famille ; grille JSONB 5 jours × 5 booléens ; facturation brouillon par foyer non idempotente.
- Rust : tables 017, 034, 036, 074, 079 avec `etablissement_id NOT NULL` et RLS (permissive) ; `ent_repas_passage` sans FK élève ; circuit des consignes (211) présent mais non branché sur le passage ; export CSV cuisine existant.
**Hypothèses [H]**
- L'établissement pilote facture la restauration dans Charlemagne et pointe par badge dans Charlemagne ou chez le prestataire (Q-RS1) ; deux services de déjeuner ; cartes à identifiant fixe.
- Les lecteurs de badges du marché fonctionnent en « clavier » USB ou NFC, exploitables par une page web ou l'appli [H, à tester].
**Contradictions signalées**
1. `TRANSITION_T0_INVENTAIRE.md` (M14) : « 12 fichiers / 1 900 lignes / 1 route » ; mesuré : environ 3 900 lignes, et des liens dans 7 autres fichiers (internat, stages, voyages, santé, IA).
2. `SOCLES.md` §40 range « badges et passages » au 40 ; les passages du **portail** de Next.js (même table `passage`) relèvent du **14** : la table de Next.js mélange les deux.
3. `SOCLES.md` §40 cite l'« alerte minimale PAI au passage » sans dire qu'elle dépend du niveau N-1 du 34, désactivé par D-034 sans HDS ni avis du DPO (bloquant A).
4. `SOCLE2_DOMAINES.md` RES-04 fait poser des **dispenses** par le secrétariat ou le système ; la conception remplace la dispense à motif libre par le **non-repas à cause codée** (R-RS7).
5. `24_FINANCES.md` §11 : « un repas réservé non pris est facturé ou non selon le règlement (S-160, avec délai de prévenance du socle 40) » : cohérent ; le délai est S-4003 ici.
6. `MATRICE_CHARLEMAGNE.md` T-23 classe le porte-monnaie en « TROU » et la checklist le disait « DROP » : la conception le garde **désactivé par défaut** (R13), activable pour les seuls commensaux (S-4021).
