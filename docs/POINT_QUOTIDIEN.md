# Point quotidien ScolIA

Un point par jour, le plus récent en haut. Toujours les mêmes rubriques : **sur ENT** (validé par la CI GitHub), **prêt mais pas encore sur ENT**, **en cours**, **conçu mais pas codé**, **pas commencé**, **décisions attendues de Florian**, **à fournir par Florian plus tard** (rappel de `A_FOURNIR_PAR_FLORIAN.md`), **ce qui a coincé**.

Légende des étapes d'un socle (D-021) : C = conception Opus, Pe = passe personas sur la conception, Co = code, R = recette personas sur le logiciel, F = relecture de Florian.

---

## 2026-11-13 (vendredi, nuit, 5 h 15 Paris)

### Passé sur `ENT` cette nuit (tête `cd12a6d2`)
- **Correctif du jour d'identité** (migration **295**) : le déclencheur de projection de l'identité prenait `current_date` (jour UTC du serveur ; sqlx impose `TimeZone=UTC`) alors que le Rust date au jour de Paris. Entre minuit et 1 h (2 h l'été), la version du jour n'était pas projetée sur la fiche. C'est la cause du rouge d'ENT `51c23382` et des rouges « élève » des sessions 15 et audit 2 (toutes leurs tâches rouges ont tourné entre 23 h et minuit UTC). Reproduit en SQL, puis corrigé avec `ent_aujourdhui_etab` (265). Le registre des migrations n'y était pour rien.
- **Socle 12, lots AS1 et AS2** (`bf340ef`) : référentiels d'assiduité, règle datée, appel v2 ; 2241–2243 ; ferme NJ-30, NJ-31, NJ-34, NJ-35. Fusion pilote : un entrelacement automatique faux dans `modele_par_defaut` a été corrigé à la main.
- **Correctifs d'audit 2** (`714bf2d3`, CI 43365111358 verte) : B-01 (relais /web et en-tête Host), B-03 (IBAN réservé à une session authentifiée), B-04 (démarrage refusé hors RLS stricte), A-01 (plus de mot de passe renvoyé : lien d'activation à usage unique), R-05 (pool de connexions borné), R-06, R-07 (envois hors transaction), R-09 (un établissement en erreur n'arrête pas les autres). Limites : A-01 en plusieurs écritures, pas de cache des droits, R-08 (pg_dump sous FORCE RLS) non traité, `ui.rs` affiche « undefined » à la place du mot de passe (non modifié, F-001).
- **Socle 15, lots A0 et A1** (`e6218f17`, CI 43368874049 verte) : absences du personnel ; fusion pilote sans conflit sur `601d05bc`. Suite : A2+ (brancher le port `edt_seances_touchees`).
- **Socle 19, lots AG3 et AG2 réduit** (`ef8d8e6e`) : réservation de toutes les salles (D-058), libre/occupé, créneau commun ; 2386 ; ferme NJ-62, NJ-76.

### Reste à faire (repéré cette nuit)
- **Autres `current_date` du socle élève** (280 reprise et resynchronisation, 281 identifiants, 282 numéro, 284 inscription, 286 options) : même risque entre minuit et 1 h. Petit lot à faire (une migration qui passe au jour d'établissement).
- ~~Test instable~~ `tests_eleve_scolarite::inscription_annuelle_et_pont_avec_l_ancienne_table` : **corrigé et versé** (ENT `601d05bc`, CI 187 verte 39/39). La reprise étant globale, l'idempotence se vérifie désormais sur l'établissement du test.
- Lots verts ou presque, qui doivent fusionner ENT puis relancer une CI : audit correctifs 1, 20 cahier, 29.

---

## 2026-11-12 (jeudi soir, 18 h 45 Paris)

### Passé sur `ENT` aujourd'hui (tête `51c23382`)
- **Messagerie M1–M2** (`c1ddae6d`, fusion pilote règle 17) : participants, contrôle d'accès, matrice et destinataires (482–484) ; ferme NJ-29, NJ-36, NJ-37, NJ-59 (partiel).
- **D-036 lot 3 + registre des migrations** (`51c23382`, fusion pilote, 67 tests rejoués en local sur base neuve) : 13 colonnes JSON retirées (668–669) ; chaque migration appliquée une seule fois, avec empreinte (audit Fable 1, R-01 à R-03).
- **Agenda AG0–AG1-bis** (`50f7e0eb`, fusion pilote) : correctifs, événements et vue à calques, feuille de semaine de la direction (2380–2381).
- **Stages ST0–ST1(a)** (`22501244`, fusion pilote) : correctifs RS-01 à RS-10, dispositifs, paramètres, organismes, périodes (2780–2782) ; ferme DN-02, DN-03, DN-09, DN-10, DN-18.
- **Internat I0–I1** (`aa43419c`, fusion pilote) : présence en faits, régime codé, zones, chambres, lits, séjours, affectations (2820–2823) ; ferme NJ-16, NJ-21, NJ-23, NJ-54.
- **Correctif SSO, passkeys et domaines (S-01, B-02)** (`fc469308`, fusion pilote ; seul rouge : test instable connu) : la faille la plus grave de l’audit Fable 1 fermée.
- **Voyages V0c** (`3f85e18d`, fusion pilote) : prestataires, devis, signature, courrier (644) ; ferme NJ-07 à NJ-13, NJ-27, NJ-28, NJ-38, NJ-55, NJ-70.
- **Emploi du temps réel T0–T2** (`efb3e085`) : versions, placements, séances enrichies, lieux et intervenants (440–444, 452) ; ports pour l'agenda, les salles libérées et les absences.
- Vérification de la fusion pilote d'hier (AS0 + portail) : verte.

### Code terminé, en file d'attente de la CI (aucun job démarré pour la plupart)
0000absences du personnel A0–A1 · correctifs audit « infrastructure » · stages ST0–ST1(a) · cahier de textes C0–C1 · agenda AG0–AG1-bis (feuille de semaine) · registre des migrations + retrait des colonnes JSON · assiduité AS1–AS2 · portail P2–P3.

### En cours
Correctifs audit « données sensibles » (reprise d'un échec).

### Plan de bascule
`docs/PLAN_BASCULE_FEVRIER.md` fusionné : ≈ 195 lots pour la bascule, vagues jusqu'au 18 février, 12 critères go/no-go, jour J du 19 au 25 février ; 20 février jugé tenable (risque moyen).

### Décisions attendues de Florian
- Q-PB1 à Q-PB8 du plan de bascule, dont **Q-PB2 avant le 27 novembre** (usage réel de M03, M07, M14, P07 ; maturité des absences du personnel).
- Allègement de la CI (recommandé) et GitHub Pro pour un mois (au choix).
- Adresses par UAI (Q-O1 à Q-O7) et paie (Q-RH42, Q-RH45, Q-RH50), recommandations appliquées par défaut.

### À fournir par Florian plus tard
Hébergement Scaleway de ScolIA (le plan le vise pour le 14 décembre) ; export anonymisé pour la répétition de Noël (anonymiseur livré début décembre) ; accord écrit de l'établissement et avis du DPO avant le 31 janvier ; inscription éditeur DNE ; comptes Apple et Google.

### Ce qui a coincé
- **File GitHub saturée** : 2 runs occupent les 20 machines, une dizaine attendent depuis 2 à 5 h ; une panne GitHub d'artefacts a fait échouer à tort une partie des tests SSO.
- **Quota** : l'audit Fable n° 2 a vidé la limite des 5 heures (≈ 183 $) ; Fable suspendu (règle 20).

---

## 2026-11-11 (mercredi soir, 20 h Paris)

### Passé sur `ENT` aujourd'hui (tête `a4bcbe0c`)
- **RLS stricte, domaine élèves** (`36e9e3c3`, nuit) et **correctif des tests à minuit Paris**.
- **Voyages V0b** (`78492105`) : budget, prix figés, documents, rappels (migration 643).
- **D-036 lot 2** (`c376da46`) : 7 colonnes JSON remplacées par des tables typées (661–667).
- **Portail « Aujourd'hui » P1** (`3e8483aa`) : accueil par profil.
- **Assiduité AS0** (`a4bcbe0c`) : correctifs urgents de l'appel et des absences (2240) ; fusion pilote sans relance (règle 17), passage de vérification sur ENT en cours.

### Prêt ou presque, pas encore sur ENT
- **Internat I0–I1** : CI relancée après deux oublis d'inventaire corrigés.
- **Messagerie M1–M2**, **agenda AG0–AG1** : CI en cours.
- **Voyages V0c**, **stages ST0–ST1(a)** : CI rouge, correction en cours.

### En cours (14 sessions de code)
Emploi du temps T0–T2, absences du personnel A0–A1, cahier de textes C0–C1, registre des migrations + retrait des colonnes JSON, portail P2–P3, assiduité AS1–AS2, trois sessions de correctifs de l'audit Fable 1 (données sensibles, infrastructure, SSO/passkeys/domaines).

### Conception
**Terminée en première version pour tout le périmètre** (50 domaines) ; ajouts du jour : paie multi-régimes (49 §36–37), adresses par UAI et groupes scolaires (`ORGANISME_ADRESSES.md`), cloud factures façon Zeendoc (48 §37), salles libérées (19, D-058), agenda à calques et feuille de semaine (19, D-059).

### Next.js en production
20 failles fermées ou réduites et en ligne (dont la prise de compte sans e-mail et les secrets de repli) ; le reste est renvoyé à la version Rust (D-054).

### Audit Fable n° 1 (cœur sur ENT)
Aucun critique ; 21 élevés, 34 moyens, 48 faibles (`docs/audits/AUDIT_FABLE_1.md`) ; correctifs lancés.

### Décisions du jour
D-047 à D-060 : compta et paie dans ScolIA en version légère (multi-régimes), cloud factures intégré, ArtCraft sans coédition, Next.js corrigé seulement pour l'externe, périmètre écoles/collèges/lycées GT, bascule unique aux vacances d'hiver 2027, tout-en-un, audits Fable réguliers.

### Décisions attendues de Florian (rien de bloquant)
- Adresses par UAI : Q-O1 à Q-O7 (recommandations appliquées par défaut).
- Paie : Q-RH42 (contrat simple), Q-RH45 (pas de paie publique), Q-RH50 (part État/OGEC du chef du primaire).

### À fournir par Florian plus tard (rien de nécessaire cette semaine)
Inscription éditeur DNE (quand il aura le temps) ; hébergement Scaleway de ScolIA avant la répétition de Noël ; exports Charlemagne anonymisés ; comptes Apple et Google ; certificat et DNS des sous-domaines avant la démo.

### Ce qui a coincé
- **File GitHub saturée** (12 branches) : passages de 1 h 30 à 2 h ; plusieurs passages annulés par des relances → règles 16 (ne pas gaspiller) et 17 (fusion pilote sans relance).
- **Limite des 5 heures** : sessions arrêtées deux fois, relancées.
- Le dépôt GitHub est **public** : pris en compte dans les correctifs Next.js.

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
