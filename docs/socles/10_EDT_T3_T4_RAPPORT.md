# Socle 10 — lots T3 (modifier le réel) et T4 (notifications et « demain ») — rapport de code

Branches empilées `ent-socle-10-code-t3` puis `ent-socle-10-code-t4`, base `origin/ENT` 498fd08d (2026-11-14).
Conception : `docs/socles/10_EMPLOI_DU_TEMPS.md` §8.2, §9, §14, §16.5, §18, §21.3, §22 (M4), §23 (sur `origin/ent-mc0hoe`).
**Niveau : implémenté + testé en local (PostgreSQL 16) ; CI : une seule, au sommet de la pile ; non validé par Florian, pas prêt pilote.**

## Lot T3 — modifier le réel

### Migrations (plage 445–459, 452 non réutilisée)
| N° | Fichier | Contenu |
|---|---|---|
| 445 | `445_edt_t3_modifications` | `ent_edt_lot`, `ent_edt_lot_cible` (les cibles sont des lignes, pas un tableau), `ent_edt_modification` (journal en ajout seul : état avant/après en colonnes typées, motif public, origine, clé d'idempotence, forçage, levée), `ent_edt_modification_interne` (motif interne **dans une table à part**), déclencheur d'état `ent_edt_seance_derive` redéfini (états dérivés des modifications vivantes : `deplacee`, `neutralisee`, `modifiee`) |
| 446 | `446_edt_t3_reprise` (rejouée **après 2300**, qui crée les archives de levée) | fonction `ent_edt_modification_reprise()` exécutée à la migration : reprise des lots (171), exceptions (042/171), déplacements (170) vivants et des archives de levée (2300) |
| 453 | `453_edt_t3_rls_permissive` (après 452, donc après 580) | RLS d'établissement des 4 tables de 445 |

`MIGRATION_TAG` inchangé (`3045_…`, la plus haute du répertoire). Aucune migration existante modifiée. Aucune colonne JSON ni tableau, aucune date UTC du jour.

### Conception « pont » (écart assumé par rapport au §22 M4, « ancien chemin retiré »)
Les lecteurs historiques (semaine, appel, délégations, notifications d'`edt_reactions.rs`, socle 15) lisent `ent_edt_exception`
et `ent_edt_deplacement`. T3 ne les retire pas (c'est le lot T11) : le **journal** `ent_edt_modification` est la source du réel
et les tables historiques en sont la **projection**, écrite dans la même transaction. Les anciens écrivains
(`edt_annuler_seance`, `edt_changer_salle`, `edt_organiser_remplacement`, `edt_deplacer_seance`, `edt_lever_exception`,
`edt_annulation_masse`) **journalisent à leur tour** (`pont_journaliser`, `pont_lever`, `pont_lot`), si bien que le socle 15 n'a rien à changer.

### Actions EDT-160 à 176
| Action | Fonction (`ent-db`) | Droit |
|---|---|---|
| 160 annuler, 161 déplacer, 163 horaire, 164 ajouter (rattrapage), 165 fusionner, 166 autre cours, 167 intervenant, 172 neutraliser | `edt_modifier_seance` (une commande, un type par changement) | `edt.modifier` |
| 162 changer le lieu | idem, type `changement_lieu` | `edt.modifier_lieu` (+ CPE) |
| 168 lever | `edt_lever_modification` (une séance), `edt_lot_lever` (un lot) | `edt.modifier` / `edt.lot_appliquer` |
| 169 lot avec aperçu, 172 neutralisation en lot | `edt_lot_apercu`, `edt_lot_appliquer` | `edt.lot_appliquer` |
| 170 grève du jour | `edt_greve_candidats` (liste à cocher), `edt_greve_du_jour` | `edt.lot_appliquer` |
| 171 salle fermée | `edt_salle_fermee_propositions`, `edt_salle_fermee_appliquer` | `edt.lot_appliquer` + `edt.modifier_lieu` |
| 173 correction du passé | `edt_corriger_seance_passee` | `edt.corriger_passe` (direction) |
| 175 rattrapage | `edt_rattrapage_propositions` | `edt.modifier` |
| 176 séances non assurées | `edt_seances_non_assurees` | `edt.modifications_lire` |
| lecture du journal | `edt_modifications` (motif interne joint seulement avec `edt.motif_interne_lire`) | `edt.modifications_lire` ; un professeur : ses propres séances |
| port socle 15 | `edt_annuler_seances(séances, absence, clé)` ; ajout = rattrapage | `edt.lot_appliquer`, `edt.modifier` |

Droits ajoutés (`ent-auth/src/actions.rs`, fonction `socle_10_edt`, une ligne d'appel — règle 21) :
`edt.modifier`, `edt.modifier_lieu`, `edt.lot_appliquer`, `edt.corriger_passe`, `edt.motif_interne_lire`,
`edt.modifications_lire`, `edt.forcer_conflit`. Direction : tous ; secrétariat : tout sauf correction du passé et forçage ; CPE : lieu et lecture du journal ;
professeur, familles, autres profils : aucun.

Routes (`ent-api/src/edt_s2.rs`) : `POST/GET /edt/modifications`, `DELETE /edt/modifications/{id}`, `POST /edt/corrections-passe`,
`POST /edt/lots/apercu`, `POST /edt/lots`, `DELETE /edt/lots/{id}`, `GET /edt/greve/candidats`, `POST /edt/greve`,
`GET|POST /edt/salles/{id}/fermeture[/propositions]`, `GET /edt/seances/{id}/rattrapage`, `GET /edt/seances-non-assurees`.
Un conflit dur répond 409 avec la liste (code, décompte, séance en face : jamais de nom).

### Contrôles §9.3 (`controler_placement`)
Jour ouvré, public libre (inclusions comprises, `ent_public_cours_jour`), intervenants libres / disponibles / présents, lieu libre (séances, réservations du socle 19,
indisponibilités du socle 5), capacité (avertissement), déplacement entre sites (avertissement). Un conflit dur bloque sauf **forçage motivé** (motif obligatoire,
droit `edt.forcer_conflit`), journalisé avec les codes forcés. `creneau_deja_ce_jour` et `seance_passee` ne se forcent jamais.

### Limites et reports (honnêtes)
- **Clé historique (créneau, jour)** : une séance ajoutée le même jour qu'une séance du même créneau est refusée (`creneau_deja_ce_jour`) ; levée au lot T11 avec les projections.
- **Intervenant (EDT-167)** : passe par la commande de remplacement historique (délégation d'appel et verrous), sans forçage.
- **Distance (EDT-174)** : non fait (Q-EDT9 non tranchée). **Séances `non_assuree`** : listées (cours vacant) mais pas posées en état (réactions de T2 aux événements du socle 6 : non faites).
- **Neutralisation sur événement (EDT-172)** : le port `edt_lot_appliquer(nature = neutralisation, origine = sortie|examen|calendrier)` existe ; les réactions aux événements des socles 38 et 42 ne sont pas câblées.
- Le socle 15 (`absences_personnels/reponse.rs`) continue d'appeler les commandes historiques (désormais journalisées) ; son passage aux nouveaux ports est à faire dans sa propre branche (`edt_annuler_seances`, ajout).
- Séances d'un lot : jusqu'à 62 jours et 500 cibles ; lot de 2 000 séances mesuré en labo (voir `PROJECT_STATUS.md`).
- Aucun écran Leptos (T12b).

## Lot T4 — voir la section ajoutée avec la branche `ent-socle-10-code-t4`.
