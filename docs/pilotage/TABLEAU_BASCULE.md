# Tableau de la bascule Next.js → ScolIA, fonction par fonction

**État au 16 novembre 2026 au soir** (origin/ENT = c75d51af). Le relevé est tiré des preuves présentes sur ENT : commits de fusion, migrations et rapports de livraison. Une fonction notée « non trouvé sur ENT » n'est pas faite, ou n'est pas encore versée.

Source des fonctions et des lots : `docs/PLAN_BASCULE_FEVRIER.md` §1.2.

**Verdict de bascule** :
- **N** : nécessaire à la bascule ;
- **N?** : nécessaire si l'usage est confirmé par Florian ;
- **D** : peut attendre ;
- **A** : archive figée suffit.

| Fonction | Backend | Écrans | Reprise des données | Verdict | Ce qui manque pour février |
|---|---|---|---|---|---|
| **M01 Voyages et sorties** | partiel : V0a–V0d, V1 et formulaires FO0–FO1 faits ; V2–V9, FO2–FO3 et argent (24 F1–F4, F10, 25 P1) à faire | partiels (V0a–V0b) | séjours repris (V0d) ; photos et devis (R7) à faire | N | chaîne la plus longue : formulaires, argent, puis V2–V9 |
| **M02 Réservation de salles** | fait pour l'essentiel (19 AG0–AG3) | oui | AG11 à faire | N | reprise des réservations futures, puis recette |
| **M03 Planning par domaines** | pas commencé (AG6b) | non | AG11 à faire | N? | usage à confirmer par Florian |
| **M04 Demandes** | **fait** (N1-0 à N1-5, 29 P4) | oui | code fait (R9), testé seulement sur jeu fictif | N | reprise sur l'export réel, puis recette |
| **M05 Absences du personnel** | **presque fait** (A0–A9, A4bis) | oui (A11a–c ; A11d–f en cours) | A10 à faire | N | reprise A10, puis recette |
| **M06 RH** | annuaire fait (6 P0–P6) ; le reste au printemps | partiels | seulement si pièce réelle | D | rien pour février |
| **M07 Registre SST** | pas commencé | non | archive à produire | A | archive figée |
| **M08 HSE ; photocopies** | **fait** (N1-3) | oui | HSE de l'année dans R9 | N / D | reprise réelle des HSE |
| **M09 Certificats, vérification QR** | partiel : D0–D5b faits ; D6 (signature) et D11 à faire | non | anciens jetons et redirections à faire | N | D6, D11, écrans, reprise des jetons, redirections |
| **M10 Fiches de dialogue** | pas commencé (23 B0, B8) | non | B14 à faire | N (campagne en mars) | tout le socle 23 utile, après D6 |
| **M11 Internat** | partiel : 41 I0–I1 faits ; I2–I4 à faire | non | I11 à faire | N (si internat utilisé) | I2–I4, écrans, reprise |
| **M12 Stages** | partiel : 39 ST0, ST1(a) faits | non | à faire | N (conventions dès mars) | fin de ST1, ST2, ST8, écrans, reprise |
| **M13 Santé, infirmerie** | socle 34 pas commencé | anciens | aucune reprise sans hébergeur de données de santé | D | confirmer l'absence de données réelles |
| **M14 Cantine** | pas commencé (40 R0–R2) | non | à faire | N? | usage du self à confirmer |
| **M15 Facturation, trésorerie** | pas commencé (24–25) | anciens | vide probable | D | rien pour février |
| **M16 RDV d'inscription, préinscriptions, fiches de rentrée** | partiel : 3 I0–I2 faits ; I3–I6, I10, 19 AG4–AG6, 4 A8 à faire | anciens | campagne en cours à reprendre | N (saison janvier–juin) | **gros retard** : environ 9 lots, puis la reprise |
| **M17 Portes ouvertes** | pas commencé (3 I10, 19 AG6) | non | prospects | N / D | I10, AG6, reprise |
| **Accueil : absences et retards d'élèves** | socle 12 fait (AS0–AS6) ; socle 14 AS0–AS1 à faire | oui (AS14a–c, tableau de bord) ; rien du socle 14 | perdable | N | 14 AS0–AS1 |
| **M18 Bien-être, signalements** | pas commencé | non | archive si données | D | confirmer que c'est vide |
| **M19 Espace documents des personnels** | pas commencé (32 ET0, ET1, ET9) | non | ET14 et R7 à faire | N | **gros retard** : socle 32 utile, reprise des fichiers |
| **M20 Feuille de semaine de la direction** | **fait** (19 AG1-bis) | oui | perdable | N | recette par la direction |
| **M20 Covoiturage, affiches, site, répartition** | pas commencé | non | perdable | D | rien |
| **P06 Messagerie des personnels** | partiel : 16 M0–M4 faits ; M5, M6, M9, M10, M12, M14, M16 à faire | anciens, à mettre à jour | archive en lecture seule à faire | N | 7 lots, écrans, archive |
| **P07 / C03 Messagerie et espace famille** | partiel (16 ; 29 P3 fait) | partiels | fils en lecture seule | N? | usage à confirmer ; comptes famille (28 MS6) |
| **P04 Registre RGPD** | **fait** (27 G0–G2) | non | registre à reprendre | N | reprise du registre ; un écran si besoin |
| **P01 Comptes et rôles** | socle 1 fait ; 28 MS6 et table des 17 rôles à faire | oui | R3, R10 à faire | N | invitations et enrôlement (MS6), R3, R10, table des rôles |
| **P03 Journaux de sécurité** | — | — | export figé | A | produire l'export |
| **C01–C04 Établissement, élèves, familles, classes, EDT** | partiel : 2 E0–E6, 4 A0–A2, 5 S0–S7, 6 P0–P6, 10 T0–T5 faits ; EDT réel (T10) à faire | anciens ; écrans du cœur (2 E7–E9, 4 A11–A12, 5 S12, 6 P11, 10 T12) à faire | R0–R1 faits ; R2 en cours ; R3 à faire | N | R2, R3, écrans du cœur, EDT réel |
| **C06, C07 Notes, vie scolaire officielles** | Charlemagne reste officiel | — | perdable | D | rien |
| **P05, P08, P10 IA, abonnement, paramétrages** | — | — | perdable | D | liste lisible des réglages à remettre à Florian |

## Synthèse

1. **Prêtes côté code** : demandes et HSE (M04, M08), RGPD (P04), feuille de semaine de la direction (M20). Il reste la recette, et la reprise réelle pour les demandes.
2. **Presque prêtes** : absences du personnel (M05) et salles (M02). Il manque la reprise des données.
3. **À moitié** : voyages (M01), accueil, messagerie (P06), comptes (P01), cœur élèves, classes et EDT (C01–C04).
4. **Pas commencées, ou à peine, alors qu'elles sont nécessaires** :
   - fiches de dialogue (M10, mars) ;
   - espace documents (M19) ;
   - portes ouvertes (M17) ;
   - suite des inscriptions (M16), des stages (M12) et de l'internat (M11).
5. **Les trois risques pour février** :
   1. **La reprise des données** : R2 est en cours ; R3, R7, R10 et les reprises de modules restent à faire. Sans elles, ni la répétition de Noël ni la bascule ne sont possibles.
   2. **Les voyages** : la chaîne D6 → FO2–FO3 → F1–F10 et P1 → V2–V9 reste entière.
   3. **Les fonctions de saison** : inscriptions et portes ouvertes, fiches de dialogue de mars, espace documents. S'y ajoutent les écrans du cœur, des certificats et de la messagerie.

## Questions pour Florian (usage réel dans l'établissement)

- **M03** : le planning par domaines est-il utilisé ?
- **M14** : le self pointe-t-il chaque jour dans Next.js ?
- **P07** : les parents ont-ils un compte aujourd'hui ?
- **M11** : l'internat est-il géré dans Next.js ?
- **M13 et M18** : y a-t-il des données réelles de santé ou de bien-être ?
- **M07** : des fiches SST s'écrivent-elles chaque semaine ?
