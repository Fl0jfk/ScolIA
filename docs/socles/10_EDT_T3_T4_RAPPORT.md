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


## Lot T4 — notifications et « demain » (branche `ent-socle-10-code-t4`, empilée sur T3)

### Migrations
| N° | Fichier | Contenu |
|---|---|---|
| 447 | `447_edt_t4_localisation_journal` | `ent_edt_localisation_journal` (ajout seul : lecteur, élève, usage, **nature** de la réponse ; jamais le lieu) |
| 454 | `454_edt_t4_rls_permissive` (après 453) | RLS du journal |

Paramètre `edt.heure_message_veille` (défaut 18 h 30) ajouté au registre de `structure_etab.rs` (valeur typée, `valeur_heure`).

### Notifications : une seule voie, le socle 8
Consommateur `d4.notifications_edt_t4` (`edt_notifications.rs`, enregistré dans `consommateurs_standard`) : les événements du §18.1
(`edt.seance_modifiee`, `edt.seance_annulee`, `edt.seance_retablie`, `edt.seance_ajoutee`, `edt.lot_modifications`), publiés dans la transaction du fait
**par T3 et par le pont des écrivains historiques**, deviennent des messages par `notifier_personne` (ENT puis envois selon les préférences) — aucune
deuxième voie. Table des **alias** (`ALIAS_ANCIENS_NOMS`) : les anciens noms (`edt.exception`, `edt.exception_levee`, `edt.remplacement_organise`,
`edt.annulation_masse`) restent émis ; ils portent le `modification_id` (ou `lot_edt_id`) du journal et l'ancien consommateur `d4.notifications_edt` les
ignore : **un changement, un message**, quel que soit l'écrivain.

| Règle (§) | Réalisation |
|---|---|
| Destinataires par **public daté** (§14.2) | `ent_public_cours_jour` à la date de la séance (et du jour d'arrivée d'un déplacement) : inclusions, exclusions ; élèves (comptes), responsables au lien actif **si l'élève est mineur** et sans interdiction de communication, intervenants (titulaire, remplaçant, co-intervenants, nouveau remplaçant). Repli sur la classe ou le groupe du créneau quand le créneau n'a pas de cours (reprise M1) |
| **Urgence selon le délai** (§9.5) | séance d'aujourd'hui ou de demain avant 10 h : U2, envoi immédiat qui ignore les heures calmes ; plus lointain : ordinaire, regroupé dans le résumé (fonction pure `planifier`, testée) |
| **Message de la veille** | lendemain matin décidé avant l'heure du soir (`edt.heure_message_veille`) : envoi **différé à cette heure**, **rappelé à 6 h 30** (second envoi dédoublonné) ; heure de Paris (décision à 0 h 20 locale = 23 h 20 UTC la veille : testé) |
| Familles (§14.4 point 1, R-E36) | prévenues tout de suite seulement si le changement modifie l'arrivée, le départ ou la journée de l'enfant ; sinon résumé du jour ; l'élève est prévenu de tout |
| **Lot** | un seul message par personne, avec le nombre de séances qui la concerne |
| **Rétablissement** | à ceux qui avaient été prévenus d'un changement de ce créneau à ce jour (y compris par une décision antérieure que la levée remplace), même urgence ; rien si rien n'avait été annoncé (changement de comportement par rapport à l'ancien consommateur, qui rétablissait même sans annonce : un test de `tests_edt_s2.rs` est adapté) ; une modification levée avant la distribution de sa pose n'est jamais annoncée (l'état courant prime) |
| Contenu | motif **public** seulement, jamais le motif interne ; le nom du remplaçant n'est pas donné (Q-EDT8 non tranchée) ; correction du passé : aucun message |

### « Demain » et « maintenant » (`edt_demain.rs`)
- `edt_tableau_demain` (`edt.demain_lire` : direction, secrétariat, vie scolaire) : intervenants absents (absence validée) et leurs séances sans décision, séances non assurées, remplacements faits et à faire, salles fermées et séances à y déplacer, changements du jour, **permanences attendues** (effectifs par plage), notifications envoyées et à envoyer. Calculé à la demande : toujours à jour ; la veille à 18 h il suffit de l'ouvrir.
- `edt_eleves_sans_cours` (lecture d'EDT-177) ; la répartition (permanence, étude, CDI, sortie selon le régime) est une écriture du socle 14, non faite.
- `edt_maintenant_eleve` (`edt.maintenant_lire` : + accueil) : séance en cours (cours, lieu, intervenant) ou « pas de cours » ; **journalisé** (usage : remise, urgence, recherche, infirmerie) ; une personne sous interdiction de remise, de visite ou de contact, et un élève sous confidentialité renforcée (hors direction) donnent la **même** réponse « indisponible » ; `edt_maintenant_salle` (ce qui l'occupe, pas qui) ; `edt_maintenant_professeur` (présent oui / non, jamais où).
- Routes : `GET /edt/demain`, `/edt/sans-cours`, `/edt/maintenant/eleves|salles|professeurs/{id}`. Droits ajoutés (même fonction `socle_10_edt`) : `edt.demain_lire`, `edt.maintenant_lire`.

### Limites et reports de T4
Écran du hall (EDT-217), accusé de l'intervenant et alerte de la vie scolaire (EDT-238, `edt.changement_non_accuse`), absence longue par étapes (EDT-228,
`edt.remplacement_etat`), information du professeur principal et de l'AESH, état `non_assuree` posé par les événements du socle 6 et `edt.seance_non_assuree` : non faits.
Les sorties (socle 42) et examens (socle 38) ne figurent pas encore dans « demain ». Le tableau n'est pas figé à 18 h : il est recalculé à chaque lecture.
Aucun écran Leptos (T12c).
