# Plan de bascule de février 2027 (Next.js → ScolIA Rust)

**Date :** 2026-11-11 — **Statut :** **proposé par le pilote (session de conception), à valider par Florian.** Docs seulement : aucun code écrit, aucun accès à la production. Ce plan applique D-057 : **une seule bascule complète et invisible** aux vacances d'hiver (**sam. 20 février → lun. 8 mars 2027**). Si le Rust n'est pas prêt, la bascule passe aux vacances de printemps (**17 avril → 3 mai 2027**). **Noël** sert de démo et de **répétition hors production**.

Sources lues : `DECISIONS.md` (D-034, D-035, D-051, D-052, D-054, D-057 à D-060), `TRANSITION_NEXT_RUST.md`, `TRANSITION_T0_INVENTAIRE.md`, `socles/28_MISE_EN_SERVICE.md`, `SOCLES.md`, `socles/MIGRATIONS.md`, `pilotage/SESSIONS_EN_COURS.md`, `POINT_QUOTIDIEN.md`, la section « Lots » des 50 conceptions de socles, et l'arbre `origin/ENT` (migrations et crates, lecture seule).

Légende : **[F]** fait vérifié · **[H]** hypothèse · **[R]** recommandation · **[Q]** question pour Florian.

---

## 0. Résumé

1. [F] **Les conceptions comptent environ 750 lots de code** pour les 51 socles. À 8–12 lots versés par jour au mieux, c'est **plus que ce que permet la période d'ici au 20 février**. On ne peut donc pas faire à la fois toute la V1 de D-034 et la bascule.
2. [R] **La bascule ne demande qu'environ 195 lots (≈ 223 unités en comptant les gros lots double)** : environ 25 en cours et environ 170 à faire. Ce sont le cœur nécessaire, les modules que le Next.js utilise vraiment, l'outil de reprise, l'exploitation et les corrections d'audits. À un rythme réaliste de **6 lots par jour en moyenne** (pas 10 : la file de CI et la limite des 5 heures freinent), ce travail est fini **vers le 30 janvier**. Il reste alors deux semaines de marge pour la recette, l'audit Fable et la répétition générale.
3. [R] **Date jugée réaliste : le 20 février 2027 tient, avec un risque moyen**, à quatre conditions : (a) le **socle N1 « Demandes »** est conçu avant fin novembre (il n'existe pas encore) ; (b) le choix de l'**e-mail sortant** du Rust est fait avant mi-décembre ; (c) l'**hébergement de production** de ScolIA existe avant le 14 décembre (F-08) ; (d) l'**accord écrit de l'établissement et l'avis du DPO** sont obtenus avant le 31 janvier, car la bascule transfère des données réelles. Si l'une manque au **go/no-go du jeudi 18 février**, on bascule aux vacances de printemps, sans dommage : le Next.js continue.
4. [R] La V1 des socles **non nécessaires à la bascule** (évaluations, compétences, bulletins hors fiche de dialogue, bourses, examens, CDI, instances, pilotage, compta et paie, mobile, IA…) **glisse de février à avril**. Cela modifie D-034 (« V1 complète fin janvier – février ») : **décision de Florian (Q-PB1)**.

---

## 1. Fonctions du Next.js réellement utilisées et lots Rust qui les remplacent

### 1.1 Principe
- [F] L'inventaire T0 classe 34 modules (`TRANSITION_T0_INVENTAIRE.md` §3.0 à §3.3). D-035 (précision 2) et D-054 trient : **fiables en usage** (voyages, salles), **partiels**, **ébauches** à refaire (RH hors absences, paie, comptabilité, pilotage) et **essais vides ou inutilisés** (infirmerie, bien-être).
- [F] D-051 : on ne reprend **que les actions métier**, aucun contournement d'authentification. Chaque lot ferme ses défauts NJ-xx avec un test de non-régression (règle 14).
- [F] Aujourd'hui, les **utilisateurs connectés du Next.js sont des personnels** (D-054). Élèves et parents n'ont pas de compte : ils passent par des liens publics ou à jeton (rendez-vous d'inscription, portes ouvertes, conventions de stage, autorisations de voyage, vérification des certificats).
- [R] « Invisible » veut donc dire : **mêmes gestes pour les personnels, dès le premier jour**. Pour les familles, leurs démarches habituelles se font **depuis un vrai compte** (invitation, puis passkey : D-009, 28 MS6). Les prospects (qui n'ont pas encore d'élève inscrit) passent par un lien public vérifié par un code envoyé par e-mail (3 I10, 19 AG6), ce qui n'est pas un contournement.

### 1.2 Tableau de correspondance

Colonne « Bascule » : **N** = nécessaire au 20 février · **N?** = nécessaire si Florian confirme l'usage (Q-T0-02, Q-T0-14) · **A** = archive en lecture seule, sans module · **D** = différable (perdable ou vide, refait plus tard).

| Code T0 | Fonction Next.js (en usage) | Lots Rust qui la remplacent | Reprise des données | Bascule |
|---|---|---|---|---|
| M01 | **Voyages et sorties** : dossier, budget, devis, participants, accompagnateurs, documents, rappels, messages, étranger | 42 V0 (V0a, V0b ✅ ; V0c en cours ; V0d), V1 (exigences, autorisations), V2 (argent : 24 F1–F4, F10 ; 25 P1), V3 (EDT et appel « en sortie »), V4 réduit (santé au niveau N-1 seulement, 34 S1–S3), V5, V7 (répercussions), V9 (étranger) ; 18 FO0–FO3 (autorisations) | V0d (ombre, bascule, différentielle) = R6 ; photos et devis via R7 | **N** |
| M02 | **Réservation de salles** | 19 AG0–AG1 (en cours), AG2, AG3 (parité NX-01 à 19) ; D-058 : salles de classe libérées, avec 10 T2–T3 et 15 A3–A4 | AG11 = R5 (salles, réservations **futures**) | **N** |
| M03 | Planning par domaines (sessions, inscriptions) | 19 AG6b | AG11 | **N?** (seulement si une campagne court en février) |
| M04 | **Demandes** (assistance, demandes internes, routage) | **Socle N1 « Demandes et flux de travail » : n'existe pas encore** (Q-T0-04, Q-PO3) ; N1-0 à N1-5 [R], plus 29 P4 (files des personnels) | N1-5 = R9 | **N** — **risque n° 1** |
| M05 | **Absences du personnel** (professeurs et OGEC, rattrapages, heures, justificatifs) | 15 A0–A1 (en cours), A2, A3, A4, A5, A7, A8, A9, A11 (écrans) | A10 = R8 | **N** |
| M06 | RH : dossier, congés, onboarding, « pouls » | Ébauche (Florian) : 6 P1–P4 ✅ couvrent l'annuaire et les engagements ; le reste vient avec 6 P7, P16 et 49 RH1–RH3 au printemps | Pièce RH réelle signalée par Florian seulement (R7) | **D** (sauf annuaire : déjà là) |
| M07 | Registre SST | 49 RH7 | Archive figée du registre + fiches ouvertes | **A** au 20 février ; RH7 au printemps [R] — **N?** si de nouvelles fiches s'écrivent chaque semaine |
| M08 | Demandes d'HSE ; photocopies couleur | HSE = nature de demande du socle N1 (validation, export) ; 49 RH4 plus tard. Photocopies : perdable, ou nature N1 | HSE de l'année en cours | **N** (HSE) / **D** (photocopies) |
| M09 | **Certificats et attestations**, vérification publique par QR | 7 D0–D6, D11 ; route publique de vérification qui **garde les jetons déjà distribués** | R7 + jetons ; redirection des anciennes URL (Q-T0-13) | **N** |
| M10 | **Fiches de dialogue** (orientation 3e et 2de) | 23 B0, B8 (reprise Next.js de l'orientation) ; signatures via 7 D6 | 23 B14 (archives des fiches) | **N** : la campagne des vœux provisoires tombe en **mars** (conseils du 2e trimestre), donc en pleine bascule |
| M11 | **Internat** | 41 I0–I1 (en cours), I2 (la nuit), I3 (absences et sorties), I4 (vie quotidienne) | I11 | **N** [H : internat en usage] |
| M12 | **Stages** (offres, conventions, signatures) | 39 ST0–ST1 (en cours), ST2, ST8 (observation de 3e, stage de 2de) | lot de reprise inclus dans ST1(d) [H], sinon ST-R | **N** : les conventions de 2de (stage de juin) se préparent dès mars |
| M13 | Santé, infirmerie | 34 (N-1 : S0–S4) ; N-2 bloqué tant qu'il n'y a pas d'hébergeur HDS | **Aucune reprise sans HDS.** Export chiffré hors ligne gardé par l'établissement [R] | **D** (D-054 : essai vide) — **[Q-PB4]** confirmer qu'il n'y a aucune donnée réelle |
| M14 | Cantine : passages | 40 R0 (correctifs), R1 (droit au repas), R2 (self) | passages non facturés | **N?** (si le self pointe tous les jours dans le Next.js) |
| M15 | Facturation des familles, trésorerie | Ébauche → 24–25 au printemps (doublon Charlemagne, D-034) | vide probable (Q-T0-06) | **D** |
| M16 | **RDV d'inscription, préinscriptions, fiches de rentrée** | 3 I3, I4, I5, I6, I10 ; 19 AG4, AG5, AG6 (créneaux, campagnes) ; 4 A8 (fiche de rentrée) ; plus de Google Agenda (D-059) | campagne en cours (R-module M16) | **N** : la saison des inscriptions va de janvier à juin |
| M17 | **Portes ouvertes** ; invitations ; partenariats | 3 I10 + 19 AG6 (sessions collectives) ; invitations perdables ; partenariats 43 PS9 | prospects de l'année | **N** (portes ouvertes) / **D** (invitations, partenariats) |
| — | **Accueil : saisie des absences et retards d'élèves** | 14 AS0, AS1 (NXA-01 à 10) ; 12 AS1–AS2 (en cours), AS3 | perdable (Q-T0-07) | **N** |
| M18 | Bien-être, signalements | 36 SO3 (+ SO12 reprise) | archive si données | **D** (D-054 : inutilisé) — **[Q-PB4]** |
| M19 | **Espace documents des personnels** (fichiers, partages) | 32 ET0, ET1 (parité NX32), ET9, ET14 (reprise M19) ; ET5 (ArtCraft) si l'édition en ligne est utilisée | ET14 + R7 (copie S3) | **N** : une perte de fichiers serait un incident |
| M20 | Feuille de semaine de la direction | 19 feuille de semaine (AG1-bis, D-059) | perdable | **N** (la direction s'en sert chaque semaine) [H] |
| M20 | Covoiturage, affiches, site public, répartition des classes | 43 PS10 ; 17 ; 5 S9 | perdable | **D** |
| P06 | **Messagerie interne des personnels** | 16 M0a–M0b ✅, M1–M2 (en cours), M3, M4, M5, M6, M9, M10, M12, M14, M16 | **A** (lecture seule, pas de reprise) | **N** |
| P07 / C03 | Messagerie et espace famille | 16 (mêmes lots) + 29 P3 (à-faire famille) | fils ouverts en lecture seule | **N?** (à vérifier : aujourd'hui les parents n'ont pas de compte) |
| P04 | **Registre RGPD** (violations, demandes) | 27 G0, G1, G2 | à conserver | **N** (obligation) |
| P01 | **Comptes et rôles** | socle 1 ✅ ; 28 MS6 (invitations par lots, passkeys) ; table de passage des 17 rôles, relue par Florian | R3 + R10 | **N** |
| P03 | Journaux de sécurité | — | **A** (export figé) | A |
| C01–C04 | Établissement, élèves, familles, classes, EDT (copies de SIECLE) | cœur : 2, 4, 5, 6 ✅ en partie ; 10 T0–T5 ; T10 ou reprise C04 pour l'EDT réel | **R3** (rapprochement, file de décisions) | **N** |
| C06, C07 | Notes, vie scolaire | Charlemagne reste officiel | vide ou perdable | **D** |
| P05, P08, P10 | Assistant IA, abonnement, paramétrages | — | perdable ; liste lisible des réglages remise à Florian | **D** |

**[Q-PB2] À trancher par Florian avant le 27 novembre** : les lignes **N?** (M03, M07, M14, P07) et la confirmation que M13 et M18 sont vides. Chaque **N?** confirmé ajoute 2 à 4 lots.

---

## 2. Lots de code nécessaires d'ici le 20 février, dans l'ordre

### 2.1 État constaté sur `origin/ENT` [F] (11 novembre, soir)
- **240 migrations.** La plus haute est `2240_assiduite_as0`. Les plages présentes couvrent : socle 1 ; 2 (E0–E6, 280–294) ; 3 (I0–I2, 300–309) ; 4 (A0, A0 bis, A1, A2, 260–272) ; 5 (S0–S5, S7, 330–341) ; 6 (P0–P4, 350–362) ; stockage objets S3 (500, 601) ; 12 AS0 (2240) ; 16 M0a–M0b (480–481) ; 21 V0 (460) ; 24 F0 (520) ; 42 V0a–V0b (640–643) ; RLS sur 150 tables (580–587) et RLS stricte du domaine élèves (600) ; D-036 lots 1–2 (560–563, 661–667).
- **En cours** (`SESSIONS_EN_COURS.md`) : 42 V0c ; 39 ST0–ST1 ; 41 I0–I1 ; 16 M1–M2 ; 19 AG0–AG1 ; 10 T0–T2 ; 15 A0–A1 ; 20 C0–C1 ; 29 P2–P3 ; 12 AS1–AS2 ; trois sessions de correctifs de l'audit Fable 1 ; retrait des colonnes D-036.
- **Débit observé** : 9 à 12 lots versés les 9 et 10 novembre, environ 5 le 11 novembre (file GitHub saturée, passages de 1 h 30 à 2 h). CI parallèle d'environ 30 minutes quand la file est libre ; 3 à 4 branches de code à la fois au plus.

### 2.2 Hypothèses de calcul [H]
- Une unité = un lot versé sur ENT. Un lot **L** compte pour 2 unités (l'expérience montre qu'on le découpe : V0 → V0a à V0d). Un lot **S** ou **M** compte pour 1.
- **Débit moyen réaliste : 6 unités par jour**, 6 jours sur 7 (8–12 les bons jours ; 3–5 les jours de file saturée ou de quota épuisé). Pendant les vacances de Noël, le débit tombe à 3 par jour : la répétition prend du temps et Florian est moins disponible.
- **Gel fonctionnel le dimanche 7 février.** Ensuite, seuls passent des corrections (audit, recette, répétition).
- Estimations des conceptions : S ≈ ½ journée de session, M ≈ 1 jour, L ≈ 2 jours ou plus. Le temps de CI s'ajoute.

### 2.3 Liste ordonnée (cœur, puis modules)

Statut : ✅ sur ENT · ⏳ en cours · ○ à faire. « Unités » = estimation pour le calcul de charge. Les dates cibles sont des dates de **versement sur ENT**.

**Vague 0 — en cours, à finir (cible : 20 novembre)**

| Lots | Statut | Unités | Cible |
|---|---|---|---|
| 10 T0, T1, T2 · 15 A0, A1 · 12 AS1, AS2 · 16 M1, M2 · 19 AG0, AG1 · 29 P2, P3 · 42 V0c · 39 ST0, ST1 · 41 I0, I1 · 20 C0–C1 (différable mais commencé) · D-036 retrait des colonnes · correctifs audit Fable 1 (×3) | ⏳ | ≈ 26 | 20 nov. |

**Vague 1 — le cœur nécessaire (cible : 11 décembre ; démo de début décembre au milieu)**

| Socle | Lots | Taille | Unités | Dépend de | Cible |
|---|---|---|---|---|---|
| 7 Documents | D0, D1, D2, D3, D4, D5, D6 | S, M, M, L, M, L, L | 10 | — (objets S3 ✅) | 4 déc. |
| 8 Notifications | N0, N1, N2, N3, N4, N6 (e-mail réel), N11 (bascule des émetteurs) | — | 7 | **choix de l'e-mail (Q-PB3)** | 11 déc. |
| 2 Élève | E7, E8, E9 (écrans) | M, M, L | 4 | E1–E6 ✅ | 27 nov. |
| 4 Dossier | A3, A4, A5, A11, A12 (écrans) | L, L, M, M, L | 8 | A2 ✅ ; 8 N2 pour A11 | 4 déc. |
| 5 Structure | S6 (groupes), S12 (écrans) | L, L | 4 | S0–S7 ✅ | 27 nov. |
| 6 Personnels | P5, P6 (délégations, signature), P11 (écrans) | M, M, L | 4 | P4 ✅ | 4 déc. |
| 10 EDT | T3, T4, T5, T12a–c | L, M, M, L | 7 | T2 ⏳ | 11 déc. |
| 12 Assiduité | AS3 (léger), AS14 (écrans de l'appel) | L, L | 4 | AS2 ⏳ | 11 déc. |
| 29 Portails | P4 (files des personnels), P5 (recherche), P14 (retrait de `ui.rs`) | — | 3 | P3 ⏳ | 11 déc. |
| Sécurité | RLS stricte des autres domaines (`RLS_STRICTE.md` §7), par blocs | M ×4 | 4 | 600 ✅ | 11 déc. |
| **Sous-total** | | | **55** | | |

**Vague 2 — mise en service, reprise et exploitation (cible : 18 décembre, avant la répétition)**

| Socle | Lots | Unités | Dépend de | Cible |
|---|---|---|---|---|
| 28 | MS0 (correctifs), MS1 (création de l'établissement, jeu de démo) | 2 | — | 27 nov. (démo) |
| 28 = R0 | `ent-reprise` : inspection d'un dump, comptages, jeu fictif au format Next.js | 2 | MS0 | 27 nov. |
| 28 = R1 | outil d'anonymisation (liste blanche des 184 tables et des collections), **livré à Florian** | 2 | R0 | **4 déc.** |
| 28 = R2 | zone de reprise, décodage strict des JSON (base et S3), correspondance, simulation | 2 | R0 | 11 déc. |
| 28 = R3 | rapprochement des référentiels et des personnes, file « à décider » | 2 | R2 ; 2, 4, 5, 6 | 18 déc. |
| 28 | MS2 (assistant de paramétrage), MS3 (modules et bandeaux) | 2 | MS1 ; 27 G1 | 18 déc. |
| 27 | G0, G1 | 2 | — | 11 déc. |
| 28 = MS4 e | rapports signés, annulation, tableau de bascule (`rep_bascule`, pré-approbation R-MSE38) | 1 | R3 | 18 déc. |
| Infra | INF1 production technique Scaleway (conteneur et base séparés : **F-08**) ; INF2 sauvegarde double destination et **restauration testée** (PRA) ; INF3 supervision et alertes | 3 | F-08 (Florian) | 18 déc. |
| **Sous-total** | | **18** | | |

**Vague 3 — les modules de l'add-on (cible : 29 janvier)**

| Module | Lots | Unités | Dépend de | Cible |
|---|---|---|---|---|
| Salles (M02) + D-058 | 19 AG2, AG3 ; libération des salles de classe (avec 15 A4) | 3 | AG1 ⏳ ; 10 T3 | 8 janv. |
| Absences du personnel (M05) | 15 A2, A3, A4, A5, A7, A8, A9, A11 | 13 | A1 ⏳ ; 10 T3 | 22 janv. |
| Messagerie (P06) | 16 M3, M4, M5, M6, M9, M10, M12, M14, M16 | 10 | M2 ⏳ ; 7 D5 ; 8 N1 | 15 janv. |
| Formulaires | 18 FO0, FO1, FO2, FO3 | 4 | 7 D6 ; 8 | 8 janv. |
| Voyages (M01) | 42 V0d, V1, V2, V3, V4 réduit, V5, V7, V9 | 11 | V0c ⏳ ; 18 FO3 ; 24–25 ; 10 T2 ; 12 | 29 janv. |
| Argent des voyages | 24 F1, F2, F3, F4, F10 ; 25 P1 | 7 | F0 ✅ | 15 janv. |
| Demandes (M04, M08 HSE) | **conception N1 (Opus, docs)** puis N1-0 (correctifs et modèle), N1-1 (natures, routage), N1-2 (prise en charge, délégation, pièces), N1-3 (HSE, reprographie), N1-4 (écrans), N1-5 (reprise R9) | 7 | conception **avant le 27 novembre** ; 6, 7, 29 P4 | 29 janv. |
| Inscriptions (M16, M17) | 3 I3, I4, I5, I6, I10 ; 4 A8 ; 19 AG4, AG5, AG6 ; feuille de semaine (D-059) | 14 | I2 ✅ ; 19 AG1 | 22 janv. |
| Certificats (M09) | 7 D11 + vérification publique des anciens jetons | 2 | D6 | 8 janv. |
| Fiches de dialogue (M10) | 23 B0, B8, B14 | 4 | 7 D6 ; 4 A4 | 22 janv. |
| Stages (M12) | 39 ST2, ST8 (+ ST-R si la reprise n'est pas dans ST1) | 3 | ST1 ⏳ | 15 janv. |
| Internat (M11) | 41 I2, I3, I4, I11 | 6 | I1 ⏳ | 22 janv. |
| Accueil | 14 AS0, AS1 | 2 | 12 AS2 | 8 janv. |
| Espace documents (M19) | 32 ET0, ET1, ET9, ET14 | 4 | objets S3 ✅ | 22 janv. |
| RGPD (P04) | 27 G2 | 1 | MS2 | 15 janv. |
| Comptes | 28 MS6 (invitations, enrôlement, guichet) = R10 ; MS8 (support) ; MS9 (page d'état, incidents) | 4 | socle 1 ✅ | 29 janv. |
| Reprises de modules | R5 salles (AG11), R6 voyages (V0d), R7 fichiers (D15), R8 absences (A10), reprises internat, stages, fiches, M19, M16 | 5 (part non comprise dans les lots ci-dessus) | R3 | 29 janv. |
| Infra | INF4 e-mail sortant (domaine, SPF, DKIM, DMARC) ; INF5 redirections des anciennes URL (certificats, liens de voyage) | 2 | Q-PB3 ; DNS | 29 janv. |
| **N?** (si confirmés) | 19 AG6b ; 49 RH7 ; 40 R0–R2 ; 16 + 29 P3 pour les familles | 0 à 10 | — | 29 janv. |
| **Sous-total** | | **≈ 102** (jusqu'à 112) | | |

**Vague 4 — audits et corrections (en continu, puis du 8 au 18 février)**

| Contenu | Unités | Cible |
|---|---|---|
| Audits Fable ciblés après chaque gros bloc (D-060 complément : documents et notifications, messagerie, EDT et absences, voyages et argent) → corrections | ≈ 8 | au fil de l'eau |
| **Audit Fable complet mi-février** (D-060) : lancé le **lun. 8 février** sur la tête gelée, rapport le 10 février ; corrections des critiques et élevés du 10 au 17 février | ≈ 8 | 17 fév. |
| Corrections de la recette par personas et de la répétition générale | ≈ 6 | 17 fév. |
| **Sous-total** | **≈ 22** | |

**Total nécessaire : ≈ 223 unités** (26 en cours + 55 + 18 + 102 + 22, hors N?), soit environ 195 lots. **Capacité** du 12 novembre au 7 février : environ 62 jours ouvrés à 6 unités + 12 jours de Noël à 3 = **≈ 400 unités**. La marge (≈ 175 unités) sert : (1) à absorber les retours en arrière et les CI rouges (on compte 30 %, soit ≈ 67) ; (2) à avancer la V1 des autres socles (≈ 110 unités sur environ 500 restantes). La charge seule serait finie mi-janvier ; ce sont les **chaînes de dépendances** (§2.4) et les répétitions qui fixent la fin réaliste vers le 30 janvier.

### 2.4 Chemins critiques [R]
1. **Documents → formulaires → voyages** : 7 D2 → D3 → D6 → 18 FO3 → 42 V1 → V2 → V7. C'est le plus long (≈ 9 lots en chaîne, dont 4 L).
2. **EDT → absences → salles libérées** : 10 T2 → T3 → 15 A3 → A4 → 19 AG3 + D-058 → A7 → A8.
3. **Reprise** : R0 → R1 (outil à Florian le 4 décembre) → export anonymisé de Florian → R2 → R3 → répétition 1 → modules.
4. **Socle N1 (Demandes)** : il n'est pas conçu. Sa conception est à lancer **tout de suite** (session Opus, docs seulement), sinon les demandes bloquent la bascule.

---

## 3. Calendrier par semaine

| Semaine | Dates | Code (vagues) | Reprise et exploitation | Florian | Débit visé |
|---|---|---|---|---|---|
| S46 | 12–15 nov. | vague 0 ; lancement de la **conception N1** | — | Q-PB1, Q-PB2 | 6/j |
| S47 | 16–22 nov. | fin de la vague 0 ; 2 E7, 5 S6, 7 D0–D2 | R0 | Q-PB3 (e-mail) | 6/j |
| S48 | 23–29 nov. | 7 D3, 2 E8–E9, 4 A3, 6 P5, MS0, MS1 ; **audit Fable fin novembre** (D-060) | R0 versé | réponse à Q-PB2 ; ouvrir la demande d'**accord écrit de l'établissement** et la saisine du **DPO** | 6/j |
| S49 | 30 nov. – 6 déc. | **démo de début décembre** (D-034) ; 7 D4–D6 ; 4 A4–A5 ; 10 T3 ; RLS 1–2 | **R1 livré à Florian (4 déc.)** | démo ; test de l'anonymiseur | 6/j |
| S50 | 7–13 déc. | 8 N0–N6 ; 10 T4–T5, T12 ; 12 AS3 ; 29 P4–P5 ; 27 G0–G1 ; N1-0 | R2 ; INF1 commencé | **F-08 : hébergement de production prêt le 14 déc.** | 6/j |
| S51 | 14–18 déc. | fin de la vague 1 ; 8 N11 ; MS2–MS3 ; N1-1 | R3 ; MS4 e ; INF2–INF3 ; **restauration testée** | **export anonymisé n° 1** (base + liste S3), le 18 déc. au plus tard | 6/j |
| S52 | 19–27 déc. | vague 3 au ralenti (19 AG2–AG3, 18 FO0–FO1) | **répétition n° 1** (§5.1) | disponible deux demi-journées | 3/j |
| S53 | 28 déc. – 3 janv. | idem | bilan de la répétition, correctifs | relit le rapport | 3/j |
| S1 | 4–10 janv. | 19 AG3 + D-058 ; 18 FO2–FO3 ; 7 D11 ; 14 AS0–AS1 ; 24 F1–F2 ; 15 A2–A3 | R5 | — | 6/j |
| S2 | 11–17 janv. | 16 M3–M9 ; 24 F3–F4, F10, 25 P1 ; 39 ST2, ST8 ; 27 G2 ; N1-2 | R7 fichiers | **export anonymisé n° 2** (le 15 janv.) | 6/j |
| S3 | 18–24 janv. | 15 A4–A9 ; 3 I3–I10 ; 19 AG4–AG6 ; 23 B0, B8, B14 ; 41 I2–I4 ; 32 ET | **répétition n° 2** (tous les modules prêts, export n° 2) | relit le rapport | 6/j |
| S4 | 25–31 janv. | 42 V1–V9 ; N1-3 à N1-5 ; MS6, MS8, MS9 ; INF4–INF5 ; 16 M10–M16 | R6, R8, R9, R10 | **accord écrit de l'établissement et avis du DPO, au plus tard le 31 janv.** ; table de passage des rôles relue | 6/j |
| S5 | 1–7 févr. | fin de la vague 3 ; **recette par personas** sur tous les parcours ; **gel le dim. 7 févr.** | répétition n° 3 sur export n° 3 (chronométrée) | **export n° 3** (anonymisé, ou réel en pré-production fermée si l'accord est obtenu) | 6/j |
| S6 | 8–14 févr. | corrections seulement ; **audit Fable complet** (8–10 févr.) ; corrections des critiques et élevés | **répétition générale** (13–14 févr.) : déroulé du jour J complet, retour arrière compris | assiste à la répétition générale ; essaie lui-même les parcours (relecture F) | corrections |
| S7 | 15–19 févr. | dernières corrections ; **go/no-go jeudi 18 février, 18 h** | préparation du jour J | **décision go/no-go** ; autorisation écrite pour la base (D-057) | — |
| J | **sam. 20 févr. → lun. 8 mars** | aucun lot fonctionnel ; correctifs d'urgence seulement | bascule (§5.2) | — | — |

**Contraintes de CI prises en compte** [R] : au plus 3–4 branches de code à la fois ; lots du même socle enchaînés sur une seule branche quand ils se suivent (moins de refusions) ; fusion par le pilote sans relance quand il n'y a pas de chevauchement (règle 17) ; pas de relance inutile (règle 16) ; les plages de migrations sont réservées d'avance (`MIGRATIONS.md`) pour N1 (proposition : **3000–3019**) et pour les reprises (2564–2577, déjà prévues).

---

## 4. Critères « prêt à basculer » (go/no-go du jeudi 18 février)

Tous sont **bloquants**. Un seul critère rouge = report aux vacances de printemps.

| N° | Critère | Preuve |
|---|---|---|
| PB-1 | **Fonctions couvertes** : chaque ligne **N** (et **N?** confirmée) du §1.2 est versée sur ENT, CI verte | tableau §1.2 coché, commit de tête |
| PB-2 | **« À ne pas perdre »** : la liste de chaque module de l'add-on (conceptions §2.4) est rejouée dans le Rust ; chaque écart est accepté par écrit par Florian | grille de parité signée |
| PB-3 | **Défauts NJ-xx** : chaque défaut du registre `ALERTES_NEXTJS_PROD.md` qui concerne un module basculé a un test de non-régression vert (D-051) | liste des tests |
| PB-4 | **Reprise vérifiée** : la répétition générale (export n° 3) passe sans refus non expliqué ; comptages exacts sur les personnes, écart ≤ 1 % sur le reste (S-574) ; totaux des voyages au centime ; deux exécutions = même résultat (idempotence) ; file « à décider » vide ou tranchée | rapport de reprise signé (R-MSE38) |
| PB-5 | **Durée** : la reprise complète tient en **moins de 8 heures**, fichiers S3 compris | chronométrage de la répétition générale |
| PB-6 | **Recette** : parcours personas joués sur la tête gelée, contre le vrai backend, en navigateur (direction, secrétariat, accueil, CPE, professeur, organisateur de voyage, responsable des salles, gestionnaire de l'internat, famille invitée) ; aucun défaut bloquant ouvert | rapport de recette |
| PB-7 | **Audit Fable complet** (D-060) : aucun critique, aucun élevé ouvert ; les moyens sont acceptés ou planifiés | `docs/audits/AUDIT_FABLE_3.md` |
| PB-8 | **Exploitation** : production Scaleway en service ; sauvegarde double destination ; **restauration testée en moins de 2 heures** ; supervision et page d'état hors Scaleway (MS9) ; e-mail sortant délivré (SPF, DKIM, DMARC) | procès-verbal de l'exercice PRA |
| PB-9 | **Retour arrière** répété pendant la répétition générale : Next.js remis en écriture en moins de 30 minutes, sans perte | journal de la répétition |
| PB-10 | **Comptes** : table de passage des rôles relue par Florian ; invitations prêtes par lots (personnels d'abord) ; guichet d'aide prêt | MS6 |
| PB-11 | **Juridique** : accord écrit de l'établissement et avis du DPO pour le transfert des données réelles (`MISE_EN_SERVICE_DONNEES.md` A8, A10) ; registre des traitements à jour (27 G1) | documents signés |
| PB-12 | **Autorisation de Florian** pour lire et transférer la base de production (D-057), donnée **par écrit** et datée | message de Florian |

---

## 5. Répétition de Noël et jour J

### 5.1 Répétition n° 1 (vacances de Noël, hors production)
- **Où** : environnement de pré-production fermé, séparé de la production technique ; base éphémère sans réseau pour l'outil (R-MSE37).
- **Données** : export n° 1 de la base Next.js et liste des fichiers S3, **anonymisés par Florian** avec l'outil R1 (aucune donnée réelle ne quitte son poste). En attendant, on travaille sur le jeu fictif au format Next.js (R0).
- **Déroulé** :
  1. (sam. 19 déc.) Florian dépose l'export anonymisé et son empreinte SHA-256.
  2. (21–22 déc.) `ent-reprise inspecter` : schéma attendu, 184 tables, collections, objets S3 ; rapport de comptages.
  3. (22–23 déc.) Simulation R2–R3 : décodage, rapprochement, file « à décider » ; mesure du taux de rapprochement automatique (cible ≥ 95 % sur les élèves et les personnels).
  4. (28–29 déc.) Application en pré-production, pour les modules prêts à ce moment (cœur, salles si AG3 est versé, voyages V0) ; parcours rapides par personas sur les données reprises.
  5. (30 déc.) **Exercice de retour arrière et de restauration** : sauvegarde de la base ScolIA de pré-production, destruction, restauration chronométrée ; annulation d'une exécution de reprise.
  6. (4 janv.) **Bilan** dans `docs/recette/REPETITION_1.md` : durées, refus, écarts, décisions demandées à Florian, corrections à planifier.
- **Démo** (D-034) : elle a lieu début décembre sur le jeu de démonstration (MS1) ; à Noël on peut la rejouer sur les données anonymisées.

### 5.2 Jour J (vacances d'hiver)

| Quand | Étape | Qui | Contrôle |
|---|---|---|---|
| jeu. 18 févr. 18 h | **Go/no-go** (§4) | Florian, avec le rapport du pilote | les 12 critères sont verts |
| ven. 19 févr. 18 h | Annonce aux personnels : « le Next.js passe en lecture seule ce soir ; ScolIA ouvre jeudi 25 » | Florian | — |
| ven. 19 févr. 20 h | **Gel du Next.js : lecture seule** (action de Florian, sans toucher à son code) ; heure de gel notée | Florian | plus aucune écriture après l'heure de gel (R-MSE42) |
| sam. 20 févr. matin | **Sauvegarde complète** de la base Next.js et copie du bucket S3, **puis restauration testée** sur une base à part ; empreintes notées | Florian (le pilote prépare les commandes) | restauration réussie, comptages identiques à la source |
| sam. 20 févr. | **Autorisation écrite de Florian** pour lire et transférer la base de production (D-057). [R] On **ne modifie pas** la base Next.js : on la lit à travers son export. Elle reste intacte et sert de retour arrière | Florian | message daté dans le journal de bascule |
| sam. 20 – dim. 21 févr. | `ent-reprise` : simulation sur l'export réel → **pré-approbation par la direction** (R-MSE38) → application en production ; copie des fichiers vers le dépôt S3 du socle 7 avec contrôle des empreintes | pilote (outil), Florian (approbation) | rapport : comptages, totaux, refus, échantillons |
| lun. 22 – mer. 24 févr. | **Contrôles** : Florian relit le rapport, vérifie des échantillons (voyages en cours, réservations futures, absences ouvertes, fiches de dialogue, conventions, fichiers des personnels) ; la file « à décider » est traitée par le secrétariat | Florian, secrétariat | rapport signé |
| jeu. 25 févr. | **Ouverture aux personnels** : invitations par lots (direction et secrétariat d'abord), passkeys, guichet d'aide ; redirections des anciennes URL actives | Florian | premières connexions suivies (MS6) |
| 25 févr. – 7 mars | Rodage pendant que les élèves sont en vacances ; correctifs d'urgence ; invitations aux familles concernées (voyages, stages, inscriptions, fiches de dialogue) | équipe | journal des incidents (MS9) |
| lun. 8 mars | Retour de tous les utilisateurs | — | — |
| fin mars | Le Next.js reste consultable en lecture seule, puis **Florian l'arrête** ; les exports et la base Next.js sont gardés selon les durées fixées avec le DPO | Florian | — |

**Retour arrière** [R] :
- **Avant l'ouverture du 25 février** : retour trivial. Florian remet le Next.js en écriture (sa base n'a pas bougé) ; la base ScolIA de production est vidée par restauration de la sauvegarde d'avant la reprise. On réessaie aux vacances de printemps.
- **Entre le 25 février et le 8 mars** : retour possible. Le journal de ScolIA liste les écritures faites depuis l'ouverture (peu nombreuses : personnels seulement, pendant les vacances) ; elles sont ressaisies dans le Next.js, qu'on remet en écriture. Seuil de décision : un défaut bloquant sans correctif en 48 heures.
- **Après le 8 mars** : on corrige en avant. Le retour reste possible en dernier recours, mais il coûterait de la ressaisie.

---

## 6. Risques, et ce qu'on coupe en cas de retard

### 6.1 Risques

| N° | Risque | Proba. | Effet | Parade [R] |
|---|---|---|---|---|
| RK-1 | **Socle N1 (Demandes) pas conçu** | élevée si rien n'est lancé | demandes et HSE sans remplaçant | conception Opus lancée cette semaine ; repli : demandes portées par 29 P4 et 16 M3 (boîtes de service), sans routage avancé |
| RK-2 | **File de CI saturée**, débit < 5/j | élevée | retard cumulé de 1 à 2 semaines | 3–4 branches au plus, lots enchaînés par socle, règles 16–17 ; on suit chaque semaine les unités versées par rapport au plan (§3) |
| RK-3 | **Limite des 5 heures et quotas** | moyenne | sessions arrêtées | relances planifiées ; audits Fable calés après une remise à zéro (D-060) |
| RK-4 | **E-mail sortant non choisi** (Scaleway TEM ou autre) | moyenne | invitations et avis bloqués | Q-PB3 avant le 13 décembre ; repli : Scaleway TEM, déjà utilisé par le Next.js |
| RK-5 | **Accord de l'établissement ou avis du DPO en retard** | moyenne | pas de transfert de données réelles | demande ouverte fin novembre ; sans accord le 31 janvier → report au printemps |
| RK-6 | **Hébergement de production en retard** (F-08) | moyenne | pas de PRA testé | liste exacte remise à Florian le 30 novembre ; production prête le 14 décembre |
| RK-7 | **Rapprochement des élèves et des personnels faible** (classes en texte libre, doublons) | moyenne | file « à décider » trop longue | mesuré dès la répétition n° 1 ; règles ajustées ; le secrétariat tranche avant le jour J |
| RK-8 | **JSON et fichiers S3** du Next.js mal décodés | moyenne | données perdues en silence | décodage strict, refus listés, zéro perte silencieuse (contrôle PB-4) |
| RK-9 | **Campagnes en cours** au 20 février (fiches de dialogue, inscriptions, portes ouvertes, voyages de printemps) | certaine | toute erreur se voit tout de suite | ces modules sont recettés en priorité ; reprise différentielle possible |
| RK-10 | **Santé** : des données réelles dans M13 | faible (D-054) | impossible à reprendre sans HDS | export chiffré hors ligne gardé par l'établissement ; aucun module santé N-2 à l'ouverture |
| RK-11 | **Mots de passe non repris** : les personnels doivent activer leur compte | certaine | friction le 25 février | invitations par lots, guichet, aide pas à pas ; ouverture le jeudi pour avoir des jours de rodage |
| RK-12 | **Contradiction avec D-034** (V1 de tous les socles fin janvier) | certaine | dispersion de l'équipe | Q-PB1 : la bascule passe d'abord ; la V1 des autres socles va de février à avril |
| RK-13 | Le socle 42 visait une bascule le 17 avril (§32), et `TRANSITION_NEXT_RUST.md` §3–4 une bascule module par module | — | documents contradictoires | D-057 fait foi ; mettre ces deux documents à jour (à faire par le pilote) |

### 6.2 Nécessaire et différable

**Nécessaire (on ne coupe pas, sinon la bascule est reportée)** : sécurité et RLS ; cœur 2, 4, 5, 6, 7 (jusqu'à D6), 8 (avec l'e-mail), 10 (T0–T5) ; reprise R0–R3 et rapports ; exploitation et PRA ; comptes (MS6) ; registre RGPD ; les modules **N** du §1.2.

**Ce qu'on coupe d'abord en cas de retard** (dans cet ordre), sans bloquer la bascule :
1. La V1 des socles qui n'entrent pas dans la bascule (9 interop hors X0, 11 mobile, 13, 17, 20 au-delà de C1, 21–22, 23 hors B0, B8 et B14, 26, 30–31, 33, 35–38, 43–51) : **report à février–avril**.
2. Dans les modules, les lots de confort : 10 T12b (écrans des modifications en lot, gardés en mode simple), 16 M4 (« écrire mieux »), 15 A9 (absences collectives : saisie une par une), 42 V9 (étranger : on garde la liste des participants et les autorisations de sortie du territoire, pas le reste), 19 AG5 réduit aux créneaux simples, 32 ET9.
3. Les N? non confirmés.
4. Les reprises de données **perdables** (photocopies, invitations, réservations passées, covoiturage, journaux, qui partent en archive figée seulement).

**Ce qu'on ne coupe jamais** : la sauvegarde vérifiée avant toute opération, l'autorisation écrite de Florian, le retour arrière répété, l'audit Fable complet, la recette sur la tête gelée.

**Si le retard dépasse ce qu'on peut couper** (au 29 janvier, plus de 25 unités nécessaires non versées) : on l'annonce à Florian le **1er février** et on vise les **vacances de printemps** (gel le 4 avril, audit Fable du 5 au 7 avril, go/no-go le 15 avril, bascule le 17 avril). Les répétitions de janvier et février restent utiles.

---

## 7. Questions pour Florian

| N° | Question | Recommandation |
|---|---|---|
| Q-PB1 | La bascule passe avant la V1 de tous les socles : la V1 hors bascule glisse de février à avril (modifie D-034) ? | oui |
| Q-PB2 | Usage réel de M03 (planning par domaines), M07 (registre SST), M14 (passages de cantine), P07 (messagerie familles) ; maturité des absences du personnel (Q-T0-14) | réponse avant le 27 novembre |
| Q-PB3 | E-mail sortant de l'ENT Rust : Scaleway TEM (déjà utilisé par le Next.js), ou autre ? | Scaleway TEM, en France, déjà connu |
| Q-PB4 | Confirmer que l'infirmerie (M13) et le bien-être (M18) du Next.js ne contiennent aucune donnée réelle | si des données existent : export chiffré hors ligne, sans reprise |
| Q-PB5 | Créer le socle N1 « Demandes et flux de travail » (Q-T0-04, Q-PO3) et le concevoir tout de suite | oui, plage 3000–3019 |
| Q-PB6 | Calendrier du jour J : gel le vendredi 19 février au soir, ouverture aux personnels le jeudi 25 février | oui |
| Q-PB7 | Sur la base de production, l'autorisation de D-057 sert-elle seulement à **lire et exporter** ? On ne modifie pas la base Next.js, pour garder un retour arrière trivial | oui, lecture et export seulement ; la lecture seule est mise par Florian |
| Q-PB8 | Pour l'EDT réel : import depuis le logiciel d'EDT de l'établissement (10 T10) ou reprise des copies du Next.js (C04) ? | reprise C04 pour février ; T10 avant la rentrée |

---

## 8. Faits, hypothèses, recommandations (synthèse)

- **Faits** : dates de D-057 ; 240 migrations sur ENT (tête de migration 2240) ; liste des lots versés et en cours (§2.1) ; environ 750 lots dans les conceptions ; débit observé de 5 à 12 lots par jour ; le socle N1 n'existe pas ; les personnels sont aujourd'hui les seuls utilisateurs connectés du Next.js.
- **Hypothèses** : débit moyen de 6 unités par jour, et de 3 à Noël ; un lot L compte pour 2 unités ; 30 % de reprises de travail ; internat, stages, RDV d'inscription et feuille de semaine en usage ; un seul établissement dans la base Next.js ; durée de reprise inférieure à 8 heures.
- **Recommandations** : bascule le 20 février jugée **réaliste, avec un risque moyen** ; priorité absolue aux vagues 1 à 3 ; conception N1 immédiate ; go/no-go le 18 février avec les 12 critères ; report au printemps sans drame si un critère est rouge.
- **Contradictions signalées** : D-034 (V1 complète fin janvier) n'est pas compatible avec la bascule au même débit ; le 42 §32 (bascule d'avril) et `TRANSITION_NEXT_RUST.md` §3–4 (bascule module par module) sont dépassés par D-057.
