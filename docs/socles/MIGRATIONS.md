# Registre des numéros de migration

Chaque session qui écrit des migrations reçoit une plage réservée ici, pour éviter les collisions entre branches (cf. `03_INSCRIPTIONS.md` §2.2 point 22). `MIGRATION_TAG` vaut toujours la plus haute migration présente (255 après le lot 7, car 250–259 sont plus hautes que les plages 170–219 ; la liste de `migrate.rs` reste en ordre croissant).

| Plage | Attribution | Branche | État (2026-11-06) |
|---|---|---|---|
| 001–134 | socle 1 et lots S2-0 à S2-4 | `ENT` | sur ENT |
| 140–145, 150 | S2-3 inscriptions, S2-6 cahier de textes | `ent-integration-lot5` | en attente du lot I0 |
| 170–179 | S2-5 emploi du temps, remplacements | `ent-socle2-edt` | 170–172 écrites ; fusionnée dans `ent-integration-lot7` |
| 180–189 | S2-7 évaluations | `ent-socle2-evaluations` | 180–182 écrites ; fusionnée (copie sans `target-s27`) dans `ent-integration-lot7` |
| 190–199 | S2-8 communication | `ent-socle2-communication` | 190–193 écrites ; fusionnée dans `ent-integration-lot7` |
| 200–209 | S2-9 finances | `ent-socle2-finances` | 200–206 écrites ; fusionnée dans `ent-integration-lot7` |
| 210–219 | S2-11 santé | `ent-socle2-sante` | 210–215 écrites ; **216 (module santé désactivé par défaut) ajoutée par l'intégration** ; fusionnée dans `ent-integration-lot7` |
| 240–249 | socle 2 élève, lots E0–E1 | `ent-socle-eleve-e0-e1` | en cours |
| 170–179 | S2-5 emploi du temps, remplacements | `ent-socle2-edt` | branche |
| 180–189 | S2-7 évaluations | `ent-socle2-evaluations` | branche |
| 190–199 | S2-8 communication | `ent-socle2-communication` | branche |
| 200–209 | S2-9 finances | `ent-socle2-finances` | branche |
| 210–219 | S2-11 santé | `ent-socle2-sante` | branche |
| 240–249 | socle 2 élève, lots E0–E1 | `ent-socle-eleve-e0-e1` | fait (240), dans E2 |
| 250–254 | socle 3, lot I0 (correctifs S2-3) | `ent-socle-inscriptions-i0` | 250 écrite (`250_inscriptions_i0.sql`) ; 251–254 libres ; fusionnée dans `ent-integration-lot6` |
| 255–259 | correctifs revue finale socle 1 (RF-01, RF-04) | `ent-socle1-revue-correctifs` | 255 écrite ; fusionnée dans `ent-integration-lot6` ; `MIGRATION_TAG` = `255_email_confirmation` |
| 260–279 | socle 4 dossier administratif (conception : `04_ADMINISTRATIF.md` §16) | `ent-socle-administratif-a0` (lot A0), `ent-socle-administratif-a0-bis` (lot A0 bis) | **260** écrite (A0) ; **265** `etablissement_fuseau`, **266** `personne_reserve`, **267** `adresse_protegee`, **268** `lien_confirmation_voie`, **269** `inscription_responsable_reserve` écrites (A0 bis) ; 261–264 et 275–279 réservées aux lots suivants (270–274 : lot A1, ligne suivante) (la numérotation du §16 de la conception est décalée en conséquence) ; `MIGRATION_TAG` = `269_inscription_responsable_reserve` sur cette branche |
| 270–274 | socle 4, lot A1 (droits et minimisation) | `ent-socle-administratif-a1` | **270** `conflit_interet`, **271** `eleve_interdiction`, **272** `contact_declarant` écrites ; 273–274 réservées (RLS du lot A14) ; `MIGRATION_TAG` = `272_contact_declarant` sur cette branche |
| 280–289 | socle 2 élève : 280 (lot E2, identité versionnée) ; **281–282 (lot E3 : identifiants, numéro)** ; **283–286 (lot E4 : scolarisations et statut dérivé, inscriptions, capacité et majorité, options)** ; **287–289 (lot E5 : paramétrage des champs, propositions, garde de source externe)** | `ent-socle-eleve-e2` (280), `ent-socle-eleve-e3-e4` (281–286), `ent-socle-eleve-e5` (287–289) | 280 sur E2 ; 281–286 intégrées dans `ent-integration-lot8b` ; **287 `eleve_param_champ`, 288 `eleve_proposition`, 289 `eleve_proposition_garde` écrites (lot E5), testées en local** ; plage 280–289 **épuisée** ; `MIGRATION_TAG` reste `370_edt_creneau_groupe` (370 est plus haute que 289) |
| 290–299 | socle 2 élève : **290–294 (lot E6 : recherche, doublons, fusion)** ; 295–299 réservées aux lots E7 et suivants | `ent-socle-eleve-e6` | **290** `eleve_recherche`, **291** `eleve_doublon`, **292** `eleve_fusion`, **293** `eleve_fusion_gardes`, **294** `eleve_fusion_securite` écrites ; 295 réservée (E6, non utilisée) ; `MIGRATION_TAG` inchangé = `370_edt_creneau_groupe` (plus haute) |

| 300–329 | socle 3 inscriptions, lots I1 et suivants | `ent-socle-inscriptions-i1` (lot I1) | **300** `inscription_annuelle_prevue` (parcours, source_ref, sources `admission`/`reinscription` sur `ent_eleve_scolarite_annee`) et **301** `inscriptions_i1_dossier` (trace de l'inscription prévue sur le dossier, source `import` des liens en attente) écrites ; **302–309 : lot I2** (branche `ent-socle-inscriptions-i2`, voir la ligne suivante) ; 310–329 réservées aux lots I3 et suivants ; `MIGRATION_TAG` = `301_inscriptions_i1_dossier` sur la branche I1 |
| 302–309 | socle 3 inscriptions, **lot I2** (campagnes, offre et places, démarches, dossiers, déclarations, fermetures, annulation de validation) | `ent-socle-inscriptions-i2` | **302** `inscription_campagne_modele` (année du référentiel, règles, versions, rapport de reprise), **303** `inscription_offre` (offres, historique de capacité, unités d'hébergement), **304** `inscription_demarche_dossier` (démarche, `etat`, références atomiques, vœux, décisions, synchronisation `statut`/`etat`), **305** `inscription_declaration_responsable` (déclarations par démarche, rangs 1 à 6), **306** `inscription_offre_reprise_places` (reprise des niveaux en offres, vœux, vue du compteur de places), **307** `inscription_fermeture_prolongation`, **308** `inscription_validation_annulee`, **309** `inscription_envoi_information` écrites ; **aucune libre** dans 302–309 ; `MIGRATION_TAG` reste `370_edt_creneau_groupe` (la plus haute du répertoire : 302–309 sont plus basses) |
| 330–349 | socle 5 structure de l'établissement et année scolaire (conception : `05_STRUCTURE.md` §18) | `ent-socle-structure-conception` (conception seulement) | réservé |
| 350–369 | socle 6 personnels, services et équipes (conception : `06_PERSONNELS.md` §17 ; 350–351 et la partie accès de 357 touchent des tables du socle 1) | `ent-socle-personnels-conception` (conception seulement) | réservé |
| 370–374 | D-022 : portée du professeur au groupe (370 : `ent_edt_creneau.groupe_id`) | `ent-d022-portee-groupes` | 370 écrite ; `MIGRATION_TAG` = `370_edt_creneau_groupe` ; 371–374 libres |

Les plages proposées dans `02_ELEVE.md` (140–149, 160–169) et `03_INSCRIPTIONS.md` (200–219) sont remplacées par ce registre.

## Mise à jour du 2026-11-07 (contradicteur du pilotage)
| Plage | Attribution | Branche | État |
|---|---|---|---|
| 216 | module santé désactivé par défaut | `ent-integration-lot7` | fait |
| 250 | inscriptions I0 | sur ENT (lot 6) | fait |
| 255 | changement d'e-mail confirmé (RF-01) | sur ENT (lot 6) | fait |
| 256–259 | failles faibles socle 1 | `ent-socle1-revue-faibles` | à vérifier |
| 260–269 | socle 4 A0 et A0 bis | `ent-socle-administratif-a0-bis` | fait |
| 270–274 | socle 4 A1 | `ent-socle-administratif-a1` | en cours |
| 280 | socle 2 E2 | `ent-socle-eleve-e2` | fait |
| 281–289 | socle 2 E3–E4 | `ent-socle-eleve-e3-e4` | en cours |
| 300–304 | socle 3 I1 | `ent-socle-inscriptions-i1` | en cours |
| 375 | séparation des foyers dans la messagerie (`ent_msg_fil.prive_user_id`, `foyer_id`) — RP1-11 | `ent-recette-1-correctifs` | fait (non fusionné) |

| 330–331 | socle 5 S0 (garde-fous d'état et de cohérence, archive des affectations) et S1 (cours : membres individuels, `ent_public_cours_jour`) ; 332–339 restent réservées au socle 5 (S2 et suivants) | `ent-socle-structure-s0-s1` | fait (S0 et S1) |
| 370 | D-022 (créneau rattaché à un groupe) — **pris sans réservation, empiète sur la plage du socle 6 (350–369) : 370–374 sont désormais réservés à D-022 et D-022 bis, et le socle 6 reste 350–369** | `ent-d022-portee-groupes`, `ent-d022-bis-enseignements` | fait / en cours |
| 3700–3799 | socle 7 documents, pièces et signatures (conception : `07_DOCUMENTS.md` §23), **si la règle « 3000 + 100 × socle » (Q-B2 d'`ARCHITECTURE_BRIQUES.md`) est adoptée** ; 3700–3712 attribuées, 3713–3789 réserve, 3790–3799 retrait des anciennes tables. La consigne de la session indiquait 4000–4099, qui reviendrait au socle 10 (Q-D1) | `ent-socle-documents-conception` (conception seulement) | réservé |
| 380–399 | socle 7, **repli** si Q-B2 n'est pas adoptée (380–392 attribuées, 399 retrait) | `ent-socle-documents-conception` (conception seulement) | réservé |

## Mise à jour du 2026-11-08 (socle 4, lot A2)
| Plage | Attribution | Branche | État |
|---|---|---|---|
| 273–276 | socle 4, lot A2 « personnes et coordonnées » (plage du socle 4 ; le §16 de la conception les numérotait 263–264, déjà prises par A0 bis) : **273** `personne_coordonnee` (coordonnées datées, historique sans valeur, fonction `ent_tel_e164`), **274** `personne_coordonnee_projection` (projection colonnes héritées ↔ coordonnées, reprise unique), **275** `personne_verification_preferences` (code de vérification, préférences, consentements), **276** `personne_version_divergence` (identité versionnée, décisions sur les divergences compte ↔ fiche). 273–274, qui étaient réservées au lot A14 (RLS), sont consommées par A2 : **le lot A14 prendra 277–278** (permissif puis strict) | `ent-socle-administratif-a2` | **écrites et testées en local** ; `MIGRATION_TAG` reste `370_edt_creneau_groupe` (la plus haute) ; 277–278 réservées à A14 (RLS), 279 libre |

Règle : aucune session ne démarre sans sa ligne ici. À terme, `MIGRATION_TAG` sera calculé automatiquement à partir du répertoire des migrations (voir D-026).


## Mise à jour du 2026-11-07 (intégration du lot 8 bis)
Intégrées dans `ent-integration-lot8b` : 240 ; 260–269 (A0, A0 bis) ; 270–272 (A1 : 273–274 restent réservées) ; 280–286 (E2, E3, E4) ; 300–301 (I1) ; 370 (D-022). Liste de `migrate.rs` = répertoire `migrations/`, ordre croissant, aucune collision ; `MIGRATION_TAG` = `370_edt_creneau_groupe`. D-022 bis n'a pas ajouté de migration au-delà de 370 (371–374 restent réservées).

## Mise à jour du 2026-11-07 (lot E5, `ent-socle-eleve-e5`)
Plage 287–289 utilisée en totalité par le lot E5 (socle 2 : propositions et paramétrage) : 287 `eleve_param_champ` (profil, réglage par champ, règles automatiques), 288 `eleve_proposition` (propositions et lignes de champs, valeurs scellées, immuabilité après décision), 289 `eleve_proposition_garde` (une version d'identité de source `import`, `famille` ou `eleve` ne s'insère que si elle référence une proposition décidée). Dans `migrate.rs` elles sont insérées après 286 et avant 300 (ordre croissant) ; `MIGRATION_TAG` ne change pas (`370_edt_creneau_groupe`). **La plage 280–289 du socle 2 est épuisée** : un lot suivant du socle 2 (E6 et au-delà) doit recevoir une nouvelle plage ici avant de démarrer (290–299, indiquée « réservée aux lots E5 et suivants » plus haut, est libre).




## Mise à jour du 2026-11-07 (conception du socle 8)
| Plage | Attribution | Branche | État |
|---|---|---|---|
| 3800–3899 | socle 8 notifications et diffusion (conception : `08_NOTIFICATIONS.md` §19 et §27), **si la règle « 3000 + 100 × socle » (Q-B2) est adoptée** ; 3800–3810 attribuées (paramètres, avis et destinataires, remises et résumés, préférences et appareils, diffusions, urgences, budget SMS et SMS entrants, retours fournisseurs, gels et liens, vues, reprise), 3811–3889 réserve, 3890–3899 retrait des anciennes tables (055, 057, 058 pour les journaux d'envoi, `ent_notification` de 102, 190, 191) après la durée d'archive | `ent-socle-notifications-conception` (conception seulement) | réservé |
| 400–419 | socle 8, **repli** si Q-B2 n'est pas adoptée (400–410 attribuées, 419 retrait) | `ent-socle-notifications-conception` (conception seulement) | réservé |

## Mise à jour du 2026-11-08 (correctifs de la recette 2)
| Plage | Attribution | Branche | État |
|---|---|---|---|
| 376–379 | correctifs de la recette personas 2 (375 est prise par la recette 1) | `ent-recette-2-correctifs` | réservée, **aucune migration écrite** : les correctifs RP2-02/03/04/05/10/11/12/14 n'ont demandé aucun changement de schéma (`MIGRATION_TAG` inchangé : `370_edt_creneau_groupe`) |

## Mise à jour du 2026-11-08 (lot E6, `ent-socle-eleve-e6`)
Plage **290–294** utilisée par le lot E6 (socle 2 : recherche instantanée, doublons, fusion de fiches), insérées dans `migrate.rs` après 289 et avant 300 : **290** `eleve_recherche` (extensions `pg_trgm` et `unaccent`, fonctions `ent_unaccent` / `ent_cle_recherche`, colonnes `cle_recherche` et `cle_recherche_civil` maintenues par déclencheur, colonne `fusionne_dans`, table `ent_eleve_confidentialite` minimale du §7.6 réutilisée par le lot E7), **291** `eleve_doublon` (`ent_eleve_doublon`, `ent_eleve_non_doublon`, `ent_eleve_doublon_passage`), **292** `eleve_fusion` (`ent_eleve_fusion`, `ent_eleve_fusion_ligne`, registre en ajout seul), **293** `eleve_fusion_gardes` (le drapeau local `ent.fusion_eleve` autorise le repointage de `eleve_id` — et rien d'autre — dans les gardes de scolarisation, identifiant et capacité, et coupe le pont des options), **294** `eleve_fusion_securite` (pas de boucle d'absorption, clé composée de `fusionne_dans`). 295 reste libre pour le lot E6 ; le lot E7 repart de 295. `MIGRATION_TAG` reste `370_edt_creneau_groupe`, plus haute migration présente.

## Réservations du 2026-11-08 (matin)
| Plage | Attribution | Branche | État |
|---|---|---|---|
| 273–276 | socle 4 A2 (personnes et coordonnées) | `ent-socle-administratif-a2` | en cours |
| 287–289 | socle 2 E5 (propositions) | `ent-socle-eleve-e5` | fait |
| 290–295 | socle 2 E6 (recherche, doublons, fusion) | `ent-socle-eleve-e6` | en cours |
| 302–309 | socle 3 I2 (campagnes, dossiers) | `ent-socle-inscriptions-i2` | en cours |
| 330–339 | socle 5 S0–S1 (structure, objet « cours ») | `ent-socle-structure-s0-s1` | en cours |
| 3800–3899 | socle 8 notifications (conception) | — | réservé |
| 3900–3999 | socle 9 interopérabilité (conception) | `ent-socle-interop-conception` | réservé |

## Mise à jour du 2026-11-08 (conception du socle 9)
| Plage | Attribution | Branche | État |
|---|---|---|---|
| 3900–3999 | socle 9 interopérabilité nationale et identité fédérée (conception : `09_INTEROP.md` §15), **si la règle « 3000 + 100 × socle » (Q-B2) est adoptée** ; 3900–3914 attribuées (moteur d'échanges 3900–3908 dont réception centrale de plateforme 3908, référentiels globaux `ref_*` 3909, fédération 3910 — **tables du noyau sécurité**, à renuméroter dans 1000–1199 si Q-B2 les y range, Q-X2 —, fournisseur d'identité 3911, GAR 3912, annuaires sortants et partenaires 3913, calendriers 3914), 3915–3989 réserve, 3990–3999 retrait des anciennes tables (`ent_import_lot`, `ent_import_ligne` de 144 ; `ent_edt_import` de 060) après la durée d'archive | `ent-socle-interop-conception` (conception seulement) | réservé |
| 420–439 | socle 9, **repli** si Q-B2 n'est pas adoptée (420–434 attribuées dans le même ordre, 439 retrait) | `ent-socle-interop-conception` (conception seulement) | réservé |
| 375 | recette 1, correctifs (fil privé des messages, parents séparés) | `ent-recette-1-correctifs` | fait |
| 376–379 | recette 2, correctifs P1 (RP2-02 à RP2-14) | `ent-recette-2-correctifs` | en cours |

## Mise à jour du 2026-11-07 (intégration du lot 8 bis)
Intégrées dans `ent-integration-lot8b` : 240 ; 260–269 (A0, A0 bis) ; 270–272 (A1 : 273–274 restent réservées) ; 280–286 (E2, E3, E4) ; 300–301 (I1) ; 370 (D-022). Liste de `migrate.rs` = répertoire `migrations/`, ordre croissant, aucune collision ; `MIGRATION_TAG` = `370_edt_creneau_groupe`. D-022 bis n'a pas ajouté de migration au-delà de 370 (371–374 restent réservées).

## Réservation du 2026-11-09 (conception du socle 10, rattrapage)
| Plage | Attribution | Branche | État |
|---|---|---|---|
| 4000–4099 | socle 10 emploi du temps (conception : `10_EMPLOI_DU_TEMPS.md` §23.2), **si la règle « 3000 + 100 × socle » (Q-B2) est adoptée** ; 4000–4011 attribuées (T0 correctifs et RLS permissive ; T1 versions, besoins, pièces, placements ; T2 séance enrichie et ses lieux, intervenants, fusions — **4003–4004 sont des tables du noyau** (Q-B15), à renuméroter dans 1200–1399 si Q-B2 les y range, comme la 3910 du socle 9 ; T3 modifications et lots ; T4 journal de localisation ; T5 publication ; T6 contraintes et vœux ; T8b scénarios ; T13 demandes), 4012–4089 réserve, 4090–4099 RLS stricte et retrait des anciennes tables (`ent_edt_exception`, `ent_edt_deplacement`, `ent_edt_annulation_masse`, `ent_edt_calendrier_ab`, `ent_edt_contrainte`) après la durée d'archive | `ent-socle-edt-conception` (conception seulement) | réservé |
| 440–459 | socle 10, **repli** si Q-B2 n'est pas adoptée (440–451 attribuées dans le même ordre, 452–457 réserve, 458–459 RLS et retraits) ; vérifié libre sur toutes les branches distantes le 2026-11-09 | `ent-socle-edt-conception` (conception seulement) | réservé |

Note : le lot T0 (correctifs urgents, 4000 / 440) peut partir avant toute décision sur Q-B2 ; il prend alors **440** et la suite du socle garde la même correspondance.



## Mise à jour du 2026-11-08 (socle 5, lots S0 et S1)
330 (`330_structure_garde_fous`) et 331 (`331_cours_membres`) ajoutées dans `migrate.rs` avant 370 (ordre croissant). `MIGRATION_TAG` restait `370_edt_creneau_groupe` sur cette branche (375 est apparue à l'intégration du lot 9) : 370 est la plus haute du répertoire. Plage 332–339 : non consommée, réservée à S2 et suivants. Les numéros 340–349 du plan d'origine de `05_STRUCTURE.md` §18 restent à répartir.

## Mise à jour du 2026-11-08 (intégration du lot 9, `ent-integration-lot9`)
Intégrées dans l'ordre croissant de `migrate.rs`, sans doublon de numéro : 287–289 (E5), 290–294 (E6), 302–309 (I2), 330–331 (S0–S1), 375 `msg_fil_prive` (correctifs de la recette 1). `MIGRATION_TAG` = `375_msg_fil_prive` (plus haute migration du répertoire). Plages inutilisées à ce jour : 295 (fin de la plage E6), 332–339 (reste de S0–S1), 376–379 (réservée à la recette 2, **aucune migration écrite** : les correctifs RP2 n'ont pas eu besoin de schéma). 273–276 restent réservées au socle 4 A2 (non intégré).
330 (`330_structure_garde_fous`) et 331 (`331_cours_membres`) ajoutées dans `migrate.rs` avant 370 (ordre croissant). `MIGRATION_TAG` reste `370_edt_creneau_groupe` : 370 est la plus haute du répertoire. Plage 332–339 : non consommée, réservée à S2 et suivants. Les numéros 340–349 du plan d'origine de `05_STRUCTURE.md` §18 restent à répartir.

## Mise à jour du 2026-11-08 (socle 6, lot P0)
350 (`350_acces_etat` : état, motif, fin d'un accès ; `ent_acces_ouvert` ; déclencheur du dernier administrateur) et 351 (`351_attributions_datees` : attributions de rôle identifiées, datées, closes au lieu d'effacées ; contrainte d'exclusion) écrites sur `ent-socle-personnels-p0` (base `ent-socle-structure-s0-s1`) et ajoutées dans `migrate.rs` avant 370. `MIGRATION_TAG` reste `370_edt_creneau_groupe` (plus haute migration présente). 352–369 restent réservées au socle 6 (P1 et suivants).

## Mise à jour du 2026-11-08 (socle 6, lot P1)
352 (`352_agents` : `ent_agent`, `ent_agent_engagement` avec exclusion par (agent, employeur, organisme, nature), `ent_agent_engagement_suspension`, `ent_agent_etat`, `ent_agent_acces_suspendu`, `ent_acces_ouvert` étendue, déclencheur « un agent par compte du personnel », reprise ; RLS), 353 (`353_agents_fonctions` : catalogue `ent_fonction` + fonctions fournies, `ent_agent_fonction`, `ent_agent_discipline`, `ent_agent_qualification`, `ent_agent_contact_urgence`, fonction principale de la reprise ; RLS) et 361 (`361_agent_identifiants` : `ent_agent_identifiant` ; RLS) écrites sur `ent-socle-personnels-p1` (base `ent-socle-personnels-p0`) et ajoutées dans `migrate.rs` avant 370. `MIGRATION_TAG` reste `370_edt_creneau_groupe` (plus haute migration présente). Restent réservées au socle 6 : 354–360, 362–369 (354 services, 355 missions, 356 groupes, 357 casquettes, 358 équipes, 359 délégations, 360 parcours, 362 agent comme sujet, 363 revue, 364 paramétrage, 365–366 RLS globale, 367 bascule, 368–369 retrait). 365–366 « RLS » : les tables du lot P1 portent déjà leur politique (même texte que 100 et 215).


## Mise à jour du 2026-11-08 (socle 5, lot S2, `ent-socle-structure-s2`)
Plage **332** (`332_structure_profil_parametres` : profil de l'établissement, paramètres versionnés, journal de structure en ajout seul) et **337** (`337_structure_lieux` : sites enrichis, bâtiments, salles typées liées à leur site, lieux extérieurs, disponibilités, indisponibilités) consommées, insérées dans `migrate.rs` après 331 et avant 370 (ordre croissant). **333–336 et 338–339 restent réservées** (S3 calendrier, S7 périodes et grilles, S5 offre, S6 groupes). `MIGRATION_TAG` reste `375_msg_fil_prive` : 375 est la plus haute migration du répertoire. Sur la base `ent-integration-lot9`, aucun autre lot n'a pris 332 ni 337.

## Mise à jour du 2026-11-08 (intégration du lot 10, `ent-integration-lot10`)
Intégrées dans l'ordre croissant de `migrate.rs`, sans doublon : 273–276 (A2), 332 et 337 (S2), 350–353 et 361 (P0 + P1). `MIGRATION_TAG` = `375_msg_fil_prive` (la plus haute du répertoire). Plages 277–278 réservées à A14 (RLS), 279 libre ; 333–336 et 338–339 libres (socle 5) ; 354–360 et 362–369 libres (socle 6) ; 371–374 réservées à D-022 / D-022 bis.

Après la reprise de la CI du lot 10 (fusion de `ent-integration-lot9`) : aucune migration ajoutée ; `MIGRATION_TAG` = `375_msg_fil_prive` reste la plus haute, aucun numéro en double (vérifié).

## Réservation du 2026-11-09 (conception du socle 21, rattrapage)
| Plage | Attribution | Branche | État |
|---|---|---|---|
| 5100–5199 | socle 21 évaluations et notes (conception : `21_EVALUATIONS.md` §23.2), **si la règle « 3000 + 100 × socle » (Q-B2) est adoptée** ; 5100–5107 attribuées (V0 correctifs urgents : RLS permissive, unicité des résultats, neutralisation de `absent_compte_zero`, libellés des mentions — touche aussi les tables actuelles du conseil, du bulletin et des compétences ; réglages et coefficients ; évaluations ; résultats et historique ; états de période et instantanés ; appréciations ; demandes et consultations ; reprise et vues de compatibilité), 5108–5189 réserve, 5190–5199 RLS stricte et retrait des anciennes tables (`ent_devoir`, `ent_note`, `ent_note_periode`, `ent_note_appreciation`, `ent_eva_regle`, `ent_eva_coef_matiere`) après la durée d'archive | `ent-socle-evaluations-conception` (conception seulement) | réservé |
| 460–479 | socle 21, **repli** si Q-B2 n'est pas adoptée (460–467 dans le même ordre, 468–477 réserve, 478–479 RLS stricte et retraits) ; vérifié libre sur toutes les branches distantes le 2026-11-09 (plus haute migration trouvée : 377) | `ent-socle-evaluations-conception` (conception seulement) | réservé |

Note : le lot V0 (correctifs urgents, 5100 / 460) peut partir avant toute décision sur Q-B2 ; il prend alors **460**. Les socles 22 et 23 recevront leur plage (5200–5299, 5300–5399, ou un repli) à leur propre conception.

## Réservation du 2026-11-09 (conception du socle 16, rattrapage, avec l'esquisse du socle 17)
| Plage | Attribution | Branche | État |
|---|---|---|---|
| 4600–4699 | socle 16 messagerie et conversations (conception : `16_MESSAGERIE.md` §27.2), **si la règle « 3000 + 100 × socle » (Q-B2) est adoptée** ; 4600–4611 attribuées (M0a correctifs urgents : RLS permissive sur toutes les tables de communication, colonne `audience` provisoire, fils ouverts par un élève rendus privés, longueurs ; M0b pagination et taille ; fil enrichi et clés composites ; participants et reprise ; messages enrichis et versions ; boîtes et traitement ; lots et envois différés ; pièces en stockage objet ; carnet et cartes ; signalements, gels, restrictions, supervisions ; absences et paramètres ; vues filtrées), 4612–4689 réserve, 4690–4699 RLS stricte et retrait des anciennes tables et colonnes (`ent_msg_accuse`, `ent_carnet_entree`, contenu `BYTEA` des pièces, `prive_user_id`, `foyer_id`, `archive`, `priorite`, `dossier`) après la durée d'archive | `ent-socle-messagerie-conception` (conception seulement) | réservé |
| 480–499 | socle 16, **repli** si Q-B2 n'est pas adoptée (480–491 dans le même ordre, 492–497 réserve, 498–499 RLS stricte et retraits) ; vérifié libre sur toutes les branches distantes le 2026-11-09 (plus haute migration trouvée : 460) | `ent-socle-messagerie-conception` (conception seulement) | réservé |
| 4700–4799 | socle 17 informations, annonces et campagnes (publications, lot M15 de `16_MESSAGERIE.md`), si Q-B2 ; à détailler à sa conception propre | — | réservé |
| 500–519 | socle 17, **repli** si Q-B2 n'est pas adoptée | — | réservé |

Note : le lot **M0a** (correctifs urgents, 4600 / 480) peut partir avant toute décision sur Q-B2 ; il prend alors **480** (et M0b **481** s'il en a besoin), et la suite du socle garde la même correspondance.
## Mise à jour du 2026-11-09 (socle 16 messagerie, lot M0a, `ent-socle-messagerie-m0a`)
Q-B2 n'étant pas tranchée, M0a prend le **repli 480–499** prévu par `16_MESSAGERIE.md` §27.2. **480** `msg_m0a_fuites` écrite (vues `ent_eleve_lien_communicant` et `ent_eleve_lien_sollicitable` ; colonnes `ent_msg_fil.audience` et `ent_msg_fil.vie_scolaire` avec reprise des fils ouverts par un élève ; RLS permissive sur 17 tables de communication), insérée dans `migrate.rs` après 375. **`MIGRATION_TAG` = `480_msg_m0a_fuites`** (plus haute du répertoire). **481** réservée à M0b ; 482–491 aux lots M1 et suivants (§27.2, dans le même ordre) ; 492–497 réserve ; 498–499 RLS stricte et retraits (M17).
## Mise à jour du 2026-11-09 (correctifs de la recette de sécurité 3)

Plage 376–379 (réservée à la recette 2, dont aucune migration n'a été écrite) reprise par la recette de sécurité 3 (`ent-recette-securite-3`) : **376** `sepa_unicite_encaissement` (RS3-02 : index uniques empêchant deux encaissements pour un même prélèvement) et **377** `edt_exception_absence_etab` (RS3-04 : clé étrangère composite `(etablissement_id, absence_rh_id)`, `NOT VALID`). Insérées dans `migrate.rs` après 375 ; `MIGRATION_TAG` = `377_edt_exception_absence_etab`. **378–379 restent libres.** À l'intégration, vérifier qu'aucune autre branche n'a pris 376 ou 377.
## Mise à jour du 2026-11-08 (socle 5, lot S5, `ent-socle-structure-s5`)
Plage **338** (`338_structure_offre` : matières enrichies, offre d'enseignements par année, cibles, groupes porteurs, règles de choix, incompatibilités, prérequis ; extension de la liste fermée d'objets du journal de structure) et **340** (`340_eleve_enseignement` : enseignements suivis par l'élève, historique en ajout seul, reprise idempotente des options historiques, contrôle « choix sans groupe ») consommées, insérées dans `migrate.rs` après 337 et avant 370 (ordre croissant). **333–336, 339 et 341–349 restent réservées** (S3, S7, S6, S4, S8 à S13). `MIGRATION_TAG` reste `375_msg_fil_prive` : 375 est la plus haute migration du répertoire. Sur la base `ent-socle-structure-s2` (qui contient 332 et 337), aucun autre lot n'a pris 338 ni 340.

## Mise à jour du 2026-11-09 (socle 5, lot S3, `ent-socle-structure-s3`)
Plage **333** (`333_structure_calendrier` : référentiel officiel global `ref_calendrier_officiel`, événements du calendrier et cibles, règle de semaines A/B, semaines matérialisées, index composites `(id, etablissement_id)` sur année, site, niveau, groupe et classe, objets de journal du calendrier, RLS de 4 tables) et **334** (`334_structure_fermeture` : contrôles de la fermeture exceptionnelle, garde « jamais supprimée, jamais rouverte », historique des fermetures en ajout seul avec RLS) consommées, insérées dans `migrate.rs` après 332 et avant 337 (ordre croissant). **`338_structure_offre.sql` a été modifiée** : sa liste fermée des objets du journal de structure (redéfinie après 333) contient maintenant aussi les cinq objets du calendrier ; sans cela les écritures du calendrier étaient refusées par la base. `MIGRATION_TAG` reste `375_msg_fil_prive` : 375 est la plus haute migration du répertoire. **335, 336, 339 et 341–349 restent réservées** (S7, S4, S6, S8 à S13). Aucune table de ce lot ne figure dans la liste des tables retirées.

## Mise à jour du 2026-11-09 (socle 5, lot S7, `ent-socle-structure-s7`, base `ent-socle-structure-s5`)
Plage **335** (`335_structure_periodes` : périodisations, cibles, enrichissement de `ent_periode`, fonction de résolution `ent_periodisation_pour_classe`) et **336** (`336_structure_grilles` : grille horaire versionnée, cibles, créneaux, gel des versions commencées, fonction `ent_grille_pour`, reprise de `ent_grille_horaire`) consommées, insérées dans `migrate.rs` après 332 et avant 337 (ordre croissant). **333–334 restent réservées à S3** (calendrier, branche parallèle) ; 339 et 341–349 restent réservées (S6, S4, S8 à S13). `MIGRATION_TAG` reste `375_msg_fil_prive` : 375 est la plus haute migration du répertoire.

Points d'intégration à connaître (migrations **rejouées à chaque démarrage**) :
1. **Liste fermée des objets du journal de structure** (`ent_structure_journal_objet_check`) : redéfinie par 332 (création), 338 (S5) et, côté S3, par 333/334. Une migration placée *avant* la dernière ne peut pas poser une liste plus courte que les lignes déjà écrites (échec « violated by some row » au redémarrage). 335 assouplit donc la contrainte en contrôle de format (`^[a-z_]{1,40}$`) ; **338 (modifiée) repose la liste fermée complète**, qui contient désormais `periodisation`, `periode`, `grille`. À l'intégration : la liste de la dernière migration qui la pose doit être l'**union** des objets de S2, S3, S5, S7. Recommandation : remplacer à terme la liste fermée par un registre.
2. **Index `ent_periode_code_uq`** : créé par 110 (unicité par niveau et code), **conservé sous le même nom** avec la définition `(periodisation_id, code)` — 110 est rejouée et le recréerait (`IF NOT EXISTS` sur le nom) avec l'ancienne définition, impossible dès qu'il existe plusieurs périodisations.
3. Contrainte d'exclusion `ent_periode_chevauchement` **non différable** (les anciens écrivains font `ON CONFLICT DO NOTHING`, refusé par Postgres avec une contrainte différable) ; le service sort les lignes de la contrainte (origine `reprise`) le temps d'un décalage de périodes voisines.


## Mise à jour du 2026-11-09 (socle 5, lot S4, `ent-socle-structure-s4`)
Plage **341** (`341_cours` : objet `ent_cours`, `ent_service_enseignement.cours_id`, `ent_edt_creneau.cours_id`, membres individuels rattachés au cours, `ent_public_cours_jour` redéfinie sur le cours, `ent_cours_vacants`, `ent_cours_reprendre()` exécutée à chaque démarrage, `ent_cours_a_traiter`, clé primaire de l'archive `ent_affectation_supprimee`, journal de structure étendu à `cours`, `cours_membre`, `cours_service`) consommée, insérée dans `migrate.rs` après 340 et avant 370 (ordre croissant). **Retouches de 338 et de 331** : la contrainte `ent_structure_journal_objet_check` est désormais `NOT VALID` en 338 et en 341 (les migrations sont rejouées à chaque démarrage ; un lot suivant étend la liste et sa propre contrainte complète la remplace, elle aussi `NOT VALID`) ; 331 ne redéfinit plus ses deux fonctions (membres, public du cours) quand `ent_cours` existe, pour que le rejeu n'écrase pas la version de 341. **333–336, 339 et 342–349 restent réservées** (S3 calendrier, S7 périodes, S6 groupes, S8 à S13). `MIGRATION_TAG` reste `375_msg_fil_prive` : 375 est la plus haute migration du répertoire. Sur la base `ent-socle-structure-s5`, aucun autre lot n'a pris 341. S3 (333–334) et S7 (335–336) se font en parallèle sur d'autres branches : aucun calendrier ni période touché ici ; **si S3 ou S7 étendent aussi la liste du journal de structure, la dernière à fusionner doit la redéfinir complète** (voir S4 ci-dessus).

## Mise à jour du 2026-11-08 (socle 6, lot P2)
357 (`357_casquettes_invitations` : `ent_membership_casquette`, `ent_session.casquette`, `ent_agent_invitation`, fonctions `ent_casquette_ouverte` et `ent_agent_avant_arrivee` ; RLS sur les deux tables) écrite sur `ent-socle-personnels-p2` (base `ent-socle-personnels-p1`) et ajoutée dans `migrate.rs` entre 353 et 361 (ordre croissant). `MIGRATION_TAG` reste `370_edt_creneau_groupe` (plus haute migration présente). Restent réservées au socle 6 : 354–356, 358–360, 362–369.

## Mise à jour du 2026-11-09 (intégration du lot 11, `ent-integration-lot11`)
Intégrées dans l'ordre croissant de `migrate.rs`, sans doublon (204 fichiers, 204 entrées) : 333–334 (S3), 335–336 (S7), 338 et 340 (S5), 341 (S4), 357 (P2). `MIGRATION_TAG` = `375_msg_fil_prive` (la plus haute du répertoire : 333–357 sont plus basses). Aucune de 331, 338, 273 n'était sur `ENT` (vérifié par `git log origin/ENT -- <fichier>`) : leur contenu a pu être modifié sur place, sans migration de rattrapage. Corrections d'intégration dans des migrations : **journal de structure** — la liste fermée `ent_structure_journal_objet_check` est reposée par 332, 333, 335 (format seul), 338 et 341 ; la dernière (341) est désormais l'**union de tous les lots** (S2, S5, S3, S7, S4), et 333 et 338 sont en `NOT VALID` (rejeu à chaque démarrage sur un journal qui contient déjà des objets de lots suivants). **`ent_tel_e164` (273)** : international aligné sur `normaliser_telephone` (9 à 15 chiffres ; le même correctif a été fait en parallèle sur `ent-integration-lot10`, fusionné sans conflit). Plages libres : 339, 342–349 (socle 5), 354–356, 358–360, 362–369 (socle 6), 371–374 (D-022), 376–379, 277–279.

## Mise à jour du 2026-11-09 (socle 21, lot V0, `ent-socle-evaluations-v0`)
**460** `eva_v0_correctifs` (repli 460–479 du socle 21 : la règle Q-B2 n'est pas adoptée) : dédoublonnage des résultats et unicité `(devoir, élève)` (archive `ent_note_doublon_archive`), statut de résultat `a_rattraper`, `absent_compte_zero` forcé à faux (contrainte), journal en ajout seul des changements de statut de période (`ent_note_periode_journal`), mentions `mise_en_garde_*`, RLS permissive sur 18 tables d'évaluation, de conseil, de bulletin et de compétences. 461–467 libres pour V1 à V7 (même ordre que la conception) ; `MIGRATION_TAG` = `460_eva_v0_correctifs` sur cette branche (la plus haute du répertoire).


## Mise à jour du 2026-11-10 (fusion de `origin/ENT` dans `ent-integration-lot11`)
Fusion de la migration **460** (`eva_v0_correctifs`, V0 évaluations) avec 333–357 du lot 11 : ordre croissant dans `migrate.rs`, aucun doublon. `MIGRATION_TAG` = `460_eva_v0_correctifs` (la plus haute du répertoire). Compte des tables avec RLS : +18 (V0) ajoutés à celui du lot 11.

## Mise à jour du 2026-11-10 (conception des socles 24 et 25, finances des familles)
| Plage | Attribution | Branche | État |
|---|---|---|---|
| 5400–5499 | socle 24 facturation (conception : `24_FINANCES.md` §25.2), **si la règle « 3000 + 100 × socle » (Q-B2) est adoptée** ; 5400–5410 attribuées, 5411–5413 F14–F16 (personas) (F0 correctifs urgents communs aux socles 24 et 25 : `RESTRICT` au lieu de `CASCADE`, déclencheurs d'immutabilité des pièces émises, numéro nul des brouillons, série, IBAN du créancier chiffré, relances sans frais, quantités ≥ 0 ; F1 journal et droits ; F2 comptes débiteurs ; F3 grilles versionnées, nature légale, TVA ; F4 engagements, sources, lignes en attente ; F5 pièces et documents ; F6 consommations ; F8 aides affectées ; F9 périodes ; F10 voyages ; F11 organismes et Chorus Pro), 5411–5489 réserve, 5490–5499 RLS stricte et retrait des tables historiques | `ent-socle-finances-conception` (conception seulement) | réservé |
| 520–539 | socle 24, **repli** si Q-B2 n'est pas adoptée (520 = F0, 521–530 dans le même ordre, 531–537 réserve, 538–539 RLS et retraits) ; **F0 part en 520** (Q-B2 non adoptée à ce jour) ; ~~le lot F0 rapatrie aussi 376 et 377~~ : **376 et 377 sont sur `ENT` depuis `ee7706d3`** (mise à jour personas du 2026-11-10), F0 les **vérifie** ; **(personas)** 531 = F14 (cohabitation an 1, appliquée hors ordre après F0, comme 376–377 après 460), 532 = F15 (confidentialité, contrôle interne), 533 = F16 (soldes de sortie), 534–537 réserve | `ent-socle-finances-conception` (conception seulement) | réservé |
| 5500–5599 | socle 25 paiements, prélèvements, recouvrement (conception : `25_PAIEMENTS.md` §23.2), si Q-B2 ; 5500–5509 attribuées (P1 à P10), 5510–5511 P13–P14 (personas), 5512–5589 réserve, 5590–5599 retrait | idem | réservé |
| 540–559 | socle 25, **repli** (540–549 dans le même ordre, 550–557 réserve, 558–559 retrait) ; **(personas)** 550 = P13 (surendettement, recours), 551 = P14 (régies, agent comptable), 552–557 réserve | idem | réservé |
| 5600–5699 / 560–579 | socle 26 bourses et aides, réservé pour éviter une collision (à détailler à sa conception) | — | réservé |

Vérifié libre sur **toutes les branches distantes** le 2026-11-10 : plus haute migration écrite trouvée **480** (socle 16 M0a) ; plus haute réservation **500–519** (socle 17). Aucun fichier de migration ni ligne de `migrate.rs` n'a été ajouté par cette conception.
