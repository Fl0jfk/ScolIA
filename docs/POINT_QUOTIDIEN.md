# Point quotidien ScolIA

Un point par jour, le plus récent en haut. Toujours les mêmes rubriques : **sur ENT** (validé par la CI GitHub), **prêt mais pas encore sur ENT**, **en cours**, **conçu mais pas codé**, **pas commencé**, **décisions attendues de Florian**, **à fournir par Florian plus tard** (rappel de `A_FOURNIR_PAR_FLORIAN.md`), **ce qui a coincé**.

Légende des étapes d'un socle (D-021) : C = conception Opus, Pe = passe personas sur la conception, Co = code, R = recette personas sur le logiciel, F = relecture de Florian.

---

## 2026-11-10 (mardi soir, 18 h 45 Paris)

### Passé sur `ENT` aujourd'hui (CI verte à chaque fois ; tête `88c1caae`)
- **Lot 11** (`5366c196`) : socle 5 S3 calendrier, S4 « cours », S5 offre, S7 périodes et grille ; socle 6 P2 invitations et casquettes.
- **Recette sécurité 3** (`ee7706d3`) : RS3-01 (parent non payeur et SEPA), RS3-02 (double encaissement), RS3-04 (EDT).
- **Socle 16 M0a** (`3d2b358b`) : interdiction de contact, fils privés, fils de vie scolaire réservés aux responsables autorisés.
- **Socle 6 P3** (`3ba1231b`) : services d'enseignement et interventions (titulaire, suppléant, co-enseignant), cours vacants.
- **Finances F0** (`e9b574e6`) : un seul chemin d'encaissement, plus de double prélèvement, numéro de facture serveur, pièces comptables immuables, mandats révoqués exclus, export SEPA en POST sans IBAN dans l'URL (RS3-03 soldée), IBAN chiffré, frais de relance désactivés.
- **Stockage des fichiers S3** (`a32ee605`) : S3 obligatoire en production (Scaleway et OVHcloud), objets immuables, empreinte SHA-256 vérifiée, aucun échec ignoré.
- **Registre de toutes les tables** (`16354f36`, D-037) : 330 tables classées, test bloquant, suspension des envois à la restauration ; défaut corrigé (appréciations invisibles si le code de période a des minuscules).
- **RLS sur 150 tables de plus** (`30b8414c`) : élèves, santé et assiduité, finances, communication, vie scolaire, évaluations, inscriptions, structure (migrations 580–587).
- **D-036, JSON → tables** (`88c1caae`) : droits délégués, résolutions d'inscription, scolarité actuelle, signalements d'assiduité ; test de garde contre tout nouveau JSON.

### Prêt mais pas encore sur `ENT`
- Rien (tout ce qui était vert est passé).

### En cours
- **Socle 16 M0b** (messagerie, défauts résiduels ; + étape labo « fil d'alerte seuil ») : CI en file.
- **RLS stricte** : faire passer les accès par le contexte d'établissement (élèves, finances, inscriptions d'abord) pour activer le mode strict.

### Conçu aujourd'hui (docs sur `ent-mc0hoe`)
- **Socles 24–25 Finances et paiements** : conception + personas (302 actions, 31 lots).
- **Socle 34 Santé** : conception + personas (mode « sans donnée de santé » strict, IA exclue, établissement sans infirmière).
- **Socle 42 Sorties et voyages** : conception à partir du module Next.js + personas (parité des 52 routes, bascule visée le 17 avril 2027).
- **Socle 16 Messagerie** : personas.
- **Transition Next.js → Rust** : plan (`TRANSITION_NEXT_RUST.md`, fusion dans le cœur, bascules pendant les vacances) et inventaire T0 de la base Next.js (maturité des modules, JSON sur S3).
- **PRA, sauvegardes et portabilité** (`PRA_SAUVEGARDES.md`, `PORTABILITE_EXPORTS.md`) : double destination Scaleway + OVHcloud, restauration par établissement, export à double validation ; exigences E1–E14 imposées à tous les lots (règle 13).
- Décisions inscrites : D-034 (feuille de route), D-035 (add-on Next.js repris en Rust et branché sur le cœur), D-036 (plus de JSON-mémoire) ; D-037 à D-046 proposées.

### Avancement par socle
| Socle | Conçu | Personas | Sur ENT | Recette |
|---|---|---|---|---|
| 1 Sécurité | — | — | oui + registre + RLS 150 tables | recettes 1 et 3 |
| 2 Élève, 3 Inscriptions, 4 Dossier | oui | oui | oui | recette 2 |
| 5 Structure | oui | oui | S0–S5, S7 | non |
| 6 Personnels | oui | oui | P0–P3 | non |
| 7, 8, 9 | oui | oui | non | — |
| 10 Emploi du temps | oui | oui | ancien | recette 3 |
| 16 Messagerie | oui | **oui** | ancien + M0a | recette 3 |
| 21 Évaluations | oui | oui | ancien + V0 | recette 3 |
| 24–25 Finances | **oui** | **oui** | ancien + F0 | recette 3 |
| 34 Santé | **oui** | **oui** | ancien (désactivé) | — |
| 42 Sorties et voyages | **oui** | **oui** | non (Next.js en prod) | — |

Environ **27 %** d'un ENT au niveau Charlemagne sur `ENT` et validé ; 16 socles sur 51 conçus en profondeur. Feuille de route D-034 : démo début décembre, V1 complète fin janvier–février 2027.

### Décisions attendues de Florian (rien de bloquant aujourd'hui)
- **Urgent, production Next.js (à toi seul)** : module santé exposé (données de soins en clair, lisibles par direction, secrétariat, CPE, surveillants, sans journal) — l'utilises-tu ? restreindre ou couper (Q-S34) ; voyages : `send-transport` et `generate-circular` ne vérifient que la session.
- **PRA** : valider D-037 à D-046 ; demander par écrit à Scaleway si leur base managée permet un retour à un instant précis (Q-PRA1) ; vérifier que le DNS de `scolia.fr` ne dépend pas de Scaleway ; format d'archive d'export ZIP chiffré ou `age` (Q-EXP1).
- **Inventaire Next.js** : absences du personnel fiables ou ébauche ? facturation des familles et covoiturage ébauches ?
- **Finances F0** : droits d'encaissement du secrétariat ; rapprocher les prélèvements annulés des mandats révoqués avant données réelles.
- **P3** : choix sur les services d'enseignement à trancher avant P4 (par défaut sinon).
- **DPO / juriste** (plus tard) : durées de conservation, avis santé avant fin janvier 2027, frais de rejet, TVA des voyages.

### À fournir par Florian (rappel)
- **Demain mercredi** : inscription simple comme éditeur (rappel programmé à 18 h 37, aide à la rédaction du courriel).
- **Avant Noël (phase T1)** : un conteneur et une base PostgreSQL séparés pour ScolIA chez Scaleway (F-08) ; je donnerai la liste exacte deux semaines avant.
- Plus tard : exports anonymisés Charlemagne et base Next.js (F-01 à F-05, F-07), avec l'outil d'anonymisation fourni d'abord.

### Ce qui a coincé
- Nuit : création de sessions bloquée en attente d'approbation (≈ 4 h perdues) → réglé par le mode Auto et `.claude/settings.json`.
- File de la CI GitHub saturée par 3–4 branches en parallèle : chaque avancée d'`ENT` oblige les autres branches à refusionner et repasser la CI (≈ 30–40 min chacune).

---

## 2026-11-09 (lundi soir, 18 h 45 Paris)

### Passé sur `ENT` aujourd'hui (CI verte : 27 jobs, 1 137 tests, 30 min)
- **Lots 9 et 10** (`e8a69c53`) : correctifs des recettes 1 et 2 ; socle 2 E5–E6 (propositions de la famille, recherche, doublons, fusion annulable) ; socle 3 I2 (campagnes, dossiers) ; socle 4 A2 (coordonnées) ; socle 5 S0–S2 (structure, objet « cours », profil, sites, salles, fuseau) ; socle 6 P0–P1 (accès sécurisés, agents et contrats) ; écrans de la démo P1 ; **CI parallèle** (30 min au lieu de 3 h 30). Exception d'audit RUSTSEC-2026-0459 vérifiée (SQLite non compilé).

### Prêt mais pas encore sur `ENT`
- **Correctifs recette sécurité 3** (`ent-recette-securite-3`) : RS3-01 (parent non payeur et SEPA), RS3-02 (double encaissement), RS3-04 (EDT) ; RS3-03 encore ouvert.
- **Lot 11** (`ent-integration-lot11`) : socle 5 S3 calendrier, S4 « cours », S5 offre, S7 périodes et grille ; socle 6 P2 invitations et casquettes ; assemblé, nouvelle base fusionnée, dernières suites en cours avant la CI.

### En cours
- **Socle 21 V0** : correctifs urgents des notes (bulletin d'un parent privé d'accès, appréciations avant le conseil, réouverture sans trace, zéro d'absence).
- **Socle 16 M0a** : correctifs urgents de la messagerie (interdiction de contact, fils privés lisibles par tout personnel).

### Conçu aujourd'hui (rattrapage des domaines d'avant la méthode)
- **Socle 10 Emploi du temps** : conception + personas (D-030 : ScolIA fait lui-même l'EDT avec l'IA ; D-033).
- **Socle 21 Évaluations** : conception + personas (D-032).
- **Socle 16 Messagerie** (avec le 17 annonces) : conception.
- **Recette sécurité 3** : cloisonnement multi-établissements vérifié sur 714 routes, aucune faille critique.
- Vérifications sur sources officielles : réglementaire des socles 2 à 8, échanges nationaux (socle 9), OMOGEN.

### Avancement par socle
| Socle | Conçu | Personas | Codé | Sur ENT | Recette |
|---|---|---|---|---|---|
| 1 Sécurité | — | — | oui | oui | recettes 1 et 3 |
| 2 Élève | oui | oui | E0–E6 | E0–E6 | recette 2 |
| 3 Inscriptions | oui | oui | I0–I2 | I0–I2 | recette 2 |
| 4 Dossier administratif | oui | oui | A0–A2 | A0–A2 | recette 2 |
| 5 Structure | oui | oui | S0–S5, S7 | S0–S2 | non |
| 6 Personnels | oui | oui | P0–P2 | P0–P1 | non |
| 7, 8, 9 | oui | oui | non | — | — |
| 10 Emploi du temps | oui | oui | ancien (lot 7) | ancien | recette 3 |
| 16 Messagerie | oui | à faire | ancien + M0a | ancien | recette 3 |
| 21 Évaluations | oui | oui | ancien + V0 | ancien | recette 3 |
| 24–25 Finances, 34 Santé | à faire | — | ancien | ancien | recette 3 (finances) |

Environ **20 %** d'un ENT au niveau Charlemagne sur `ENT` et validé ; 13 socles sur 51 conçus en profondeur.

### Recommandations appliquées par défaut aujourd'hui (D-031)
D-032 (socle 21 : V0 d'abord, notes et bulletins dans Charlemagne l'an 1, publication explicite puis automatique à l'arrêt, fenêtre du conseil, IA qui propose sans décider) ; D-033 (socle 10 : choix des élèves qui construisent les barrettes, explications de l'IA sans contrainte d'un collègue, EDT temporaires) ; socle 16 : un fil par foyer, messagerie de l'élève non lue par ses parents, pas de messagerie libre entre élèves, plage calme nationale.

### Décisions attendues de Florian
- Aucune urgente. Pour le DPO plus tard : durées de conservation (messages, notes, journaux), module santé.

### À fournir par Florian (rappel)
- **Cette semaine** : inscription simple comme éditeur (Pléiade ou communication-editeurs@education.gouv.fr) — rappel programmé mercredi 11 nov. à 18 h 37.
- Plus tard : exports Charlemagne et XML anonymisés (F-01 à F-05, F-07), avec un outil d'anonymisation fourni par le pilote.

### Ce qui a coincé
- Lots 9 et 10 : cinq passages en CI rouges avant le vert (un ou deux tests cassés par les fusions à chaque fois) ; réglé par la CI parallèle.
- Redémarrages du pilote : environ 5 h perdues la nuit dernière, plusieurs coupures dans la journée (suivi désormais planifié avant toute action).

---

## 2026-11-09 (lundi, 11 h UTC)

### Sur `ENT` (validé par la CI GitHub)
- **Socle 1 Sécurité** : fait, revue de sécurité corrigée (lot 6).
- **Lot 7** (code d'avant la méthode par socles) : évaluations, finances et prélèvements SEPA, communication, santé **désactivée par défaut**, emploi du temps. Ces domaines (socles 10, 16, 21, 24, 25, 34) existent mais n'ont **pas encore** eu leur conception de fond ni leur recette.
- **Lot 8 bis** (8 nov.) : socle 2 élève E0–E4, socle 4 dossier administratif A0–A1, socle 3 inscriptions I1, D-022 bis (professeur limité à son groupe).

### Prêt mais pas encore sur `ENT`
- **Lot 9** (dans le lot 10) : correctifs des recettes 1 et 2 (parents séparés, « ne pas solliciter », secrétariat qui valide, inscription visible de la scolarité), socle 2 E5–E6 (propositions de la famille, recherche, doublons, fusion annulable), socle 3 I2 (campagnes, dossiers), socle 5 S0–S1 (structure, objet « cours » D-022).
- **Lot 10** : socle 4 A2 (coordonnées), socle 6 P0–P1 (accès personnels sécurisés, agents et contrats), socle 5 S2 (profil, sites, salles, fuseau), écrans de la démo P1.
- État : 5 passages en CI rouges depuis hier (1 ou 2 tests cassés par les fusions à chaque fois, aucun défaut de sécurité) ; 6e passage en cours (lancé à 10 h 04 UTC).

### En cours
- **Lot 11** (assemblage) : socle 5 S3 calendrier, S4 « cours », S5 offre et enseignements suivis, S7 périodes et grille ; socle 6 P2 invitations et casquettes. Tous codés et testés sur leur branche.
- **CI accélérée** : suite backend découpée en 20 tranches parallèles ; objectif < 1 h au lieu de 3 h 30.

### Conçu mais pas codé
- **Socle 7 Documents et signatures** : C + Pe faits.
- **Socle 8 Notifications** : C + Pe faits ; vérifié réglementairement (avis d'absence obligatoire, etc.).
- **Socle 9 Interopérabilité** : C + Pe faits, vérifié sur sources officielles le 9 nov. ; D-029 (public avant 2028, ÉduConnect avant GAR).
- **IA au cœur** (IA_COEUR.md) et **architecture en briques** (D-026, prototype `ent-kit`) : conçus, refonte à faire après stabilisation.
- Socles 5 et 6 : lots restants (S6 groupes, S8 bascule d'année, S9 constitution des classes, S10–S13 ; P3 à P16).

### Avancement par socle
| Socle | C | Pe | Co | Sur ENT | R | F |
|---|---|---|---|---|---|---|
| 1 Sécurité | — | — | fait | oui | oui (recette 1) | — |
| 2 Élève | fait | fait | E0–E6 | E0–E4 | recette 2 | à faire |
| 3 Inscriptions | fait | fait | I0–I2 | I0–I1 | recette 2 | à faire |
| 4 Dossier administratif | fait | fait | A0–A2 | A0–A1 | recette 2 | à faire |
| 5 Structure | fait | fait | S0–S5, S7 | non | non | à faire |
| 6 Personnels | fait | fait | P0–P2 | non | non | à faire |
| 7 Documents | fait | fait | non | — | — | — |
| 8 Notifications | fait | fait | non | — | — | — |
| 9 Interopérabilité | fait + vérif. | fait | non | — | — | — |
| 10–51 | non (sauf code ancien du lot 7 pour 10, 16, 21, 24, 25, 34) | | | | | |

Estimation honnête : environ 10 à 15 % d'un ENT au niveau Charlemagne est sur `ENT` et validé ; 20 à 25 % en comptant ce qui est prêt sur les branches.

### Décisions attendues de Florian (regroupées dans une liste unique après l'arrivée des lots 9–11 sur ENT)
- Points de validation des lots : S2 (5), S3 (5), S4 (6), S5 (9), S7, P0 (8), P1 (7), P2, démo P1 (2), I2 (10 écarts).
- Socles 7–9 : bloquants des passes personas (Q-D5 Typst, Q-N5 SMS d'absence, Q-N22 SMS facturés, Q-X33 à X35, Q-X45 GAR pour le privé).
- Architecture en briques : Q-B1 à Q-B4.
- Juriste / DPO (vérification réglementaire) : archives du privé, signature par passkey, exeat et impayés, élève majeur, droit à l'image, durées de conservation, module santé.

### Ce qui a coincé
- **CI trop lente** (3 h 30 par passage) : chaque test cassé coûte un tour complet → remède en cours (CI parallèle).
- **Redémarrages du pilote** : environ 5 h perdues cette nuit, 1 h 30 hier après-midi → suivi replanifié avant toute action.
- Accès Internet ouvert ce matin : les conceptions écrites « de mémoire » ont été vérifiées.
