# Point quotidien ScolIA

Un point par jour, le plus récent en haut. Toujours les mêmes rubriques : **sur ENT** (validé par la CI GitHub), **prêt mais pas encore sur ENT**, **en cours**, **conçu mais pas codé**, **pas commencé**, **décisions attendues de Florian**, **à fournir par Florian plus tard** (rappel de `A_FOURNIR_PAR_FLORIAN.md`), **ce qui a coincé**.

Légende des étapes d'un socle (D-021) : C = conception Opus, Pe = passe personas sur la conception, Co = code, R = recette personas sur le logiciel, F = relecture de Florian.

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
