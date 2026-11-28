# Socle 27, palier G3b + G4 — **suites** (moyens et faibles en file) — livraison

Branche `claude/project-thread-kf54t1`, partie de `ent-socle-27-palier-g3b-g4` (PR 165) : tout se règle dans les
migrations **2545** et **2546**, qui ne sont **pas versées** (absentes de `migrations_versees.txt`, en fin de tableau
de `migrate.rs`). Aucune nouvelle migration, aucun déplacement dans `migrate.rs`, `MIGRATION_TAG` inchangé (3808).
Source de la file : `docs/pilotage/revues/27_G3B_G4_LOTS_1_2_CONTRE_REVUE.md` (M2, M4 à M6, F1 à F8) et
`docs/socles/27_G3B_G4_LIVRAISON.md` (faibles du lot 5).

## Ce qui est fait

| Réf. | Correctif | Où | Test |
|---|---|---|---|
| **M2** | Une décision n'est reportée sur la version suivante que si l'élément est **identique** : empreinte à clé (HMAC serveur, usage `marquage-rgpd:v1`, liée à la demande) de la cellule ou de la ligne, colonne `rgp_extrait_masquage.element_empreinte`. Une ligne avec identifiant se retrouve par son identifiant quel que soit son rang ; une **levée** n'est jamais reportée sur une ligne sans identifiant. La valeur n'est jamais stockée. | `droits_extrait_masquage.rs` (`persister_marquages`, `empreinte_element`), 2545 | `g3b4_decision_non_reportee_quand_l_element_change`, `l_element_marque_suit_le_contenu` |
| **M4** | R-RG46 : pour un lecteur sans `droits.arbitrer`, une protection est **absente** (cellule ou ligne retirée, marquage non listé, ni dans la lecture ni dans la liste des marquages) ; un compteur `elements_reserves_direction` (toujours présent) dit seulement combien. L'alias `a_arbitrer` couvre **toutes** les raisons réservées à la direction, il n'équivaut plus au code. Écran : encart « N élément(s) réservé(s) à la direction ». | `cacher_protections`, `raison_visible`, `sans_protections_si`, `ent-types`, écran Leptos | `g3b4_le_secretariat_ne_situe_pas_une_protection`, `une_protection_est_absente_de_la_vue_du_secretariat` |
| **M5** | Ajout manuel : `rang` nullable (ni ligne ni rang = toutes les lignes ; `rang` pour une ligne sans identifiant), élément **validé** contre le contenu reconstruit (table, ligne, colonne présentes ; un lecteur sans `droits.arbitrer` ne peut désigner ce qu'il ne voit pas) ; doublon refusé (409). Corps HTTP : champ `rang` facultatif. | `droits_masquage_ajouter`, `element_existe`, `droits_contenu_reconstruit` | `g3b4_ajout_manuel_valide_et_masque_toutes_les_lignes`, `un_masquage_sans_ligne_vise_toute_la_table` |
| **M6** | Sujets « personne » et « agent » marqués : pièce dont le type n'est pas propre au porteur (foyer : livret de famille, justificatif de domicile ; décision de justice ; type inconnu) → `piece_tiers` ; élève dont la personne n'est pas responsable → `autre_eleve`. | `faits_personne`, `marquer`, `marques_calculees` | `sujet_personne_pieces_de_tiers_et_autres_eleves` |
| **F1** | `rgp_extrait_masquage` : garde (pas de suppression, élément figé, une décision ne revient pas « à décider ») et index unique de l'élément (`NULLS NOT DISTINCT`). | 2545 | `g3b4_gardes_des_marquages_et_des_extraits` |
| **F2** | `rgp_extrait_garde` fige aussi `periode_du`, `periode_au`, `assemble_par_compte_id`, `consultations_jusqu_au` ; un extrait périmé le reste. | 2545 | idem |
| **F3** | Tables de messagerie sans règle (`ent_msg_telephone`, `ent_msg_lien_decision`, `ent_msg_brouillon`, `ent_msg_lecture`, `ent_msg_fil_eleve`) **déclarées** dans les exclusions de la section, avec leur motif (élève et personne). | `exclusions_messagerie` | `g3b4_tables_de_messagerie_ecartees_declarees` |
| **F4** | `GET …/contenu` : `Cache-Control: private, no-store` ; refus 403 **avant toute écriture** quand le navigateur annonce `Sec-Fetch-Site: cross-site` (lien ou image tiers que `SameSite=Lax` laisserait passer). Les applis (Bearer) n'envoient pas l'en-tête. | `ent-api/src/rgpd_extrait.rs` | `g3b_http_extrait_parcours_et_refus` (complété) |
| **F5** | Une décision ou un ajout de marquage après l'avis du DPO **efface** l'avis (la soumission reste ; journal « avis du DPO retiré »). | `effacer_avis_dpo` | `g3b4_avis_dpo_efface_et_refus_neutre` |
| **F6** | DPO sans `droits.traiter`, extrait non soumis : **404** (comme un identifiant inconnu), en lecture du contenu et des marquages. | `droits_extrait_lire`, `droits_masquages` | idem |
| **F7** | `autre_eleve` marque aussi le **nom** d'un autre élève (cité par son identifiant dans le contenu) dans les textes : mot entier, sans casse, `prénom nom` puis `nom` (≥ 3 lettres). | `noms_des_autres_eleves`, `cite_un_nom` | `le_nom_d_un_autre_eleve_dans_un_texte_est_marque` |
| **F8** | Rien à changer : conforme à l'intention (double suppression idempotente, un seul `piece_detruite`). | — | — |
| Lot 5, alertes | Le balayage horaire repart du dernier balayage réussi (moins 2 h de recouvrement, au plus 35 jours) : un arrêt de plus de 26 h ne laisse plus de trou. Table `acc_balayage` (2546, RLS posée, registre). L'instant ne recule jamais. | `acces_alertes_evaluer_a`, `fenetre_de_balayage` | `g4b_le_balayage_des_alertes_note_sa_reprise`, `la_fenetre_reprend_apres_un_arret_long_sans_remonter_au_dela_de_35_jours` |

## Contre-revue indépendante (lecture seule, sur le premier commit) et suites

Relecteur indépendant (agent de revue, lecture seule). Deux élevés, corrigés avec un test chacun :

- **E1** : sur la ligne d'un responsable protégé, le marquage `autre_parent` (ou `parent_donnees_propres`) qui double
  la protection restait listé (alias `a_arbitrer`) et désignait une ligne absente du contenu : signal distinctif.
  Correctif : `Reserve` (calculée depuis les seuls marquages, donc aussi pour la liste sans contenu) cache **tout**
  marquage dont la cible (ligne ou cellule) est retirée ; le compteur compte tous les marquages cachés. Test
  `aucun_marquage_visible_ne_situe_une_ligne_protegee`.
- **E2** : le retrait des lignes décalait les rangs montrés au secrétariat ; un ajout manuel par rang pouvait viser
  la ligne protégée et laisser partir la ligne voulue. Correctif : rangs **affichés** recalculés pour ce lecteur, et
  rang désigné retraduit en rang d'origine avant stockage (`Reserve::rang_d_origine`). Même test.

Moyens et faibles de cette revue :

- *M6, sujet sans fiche personne* : sans objet, un sujet « personne » ou « agent » est toujours une fiche personne
  (`contexte_sujet`). *Sur-marquage d'un extrait d'agent* : risque noté, à mesurer sur un extrait réel d'enseignant ;
  une décision en lot serait la suite naturelle si le volume gêne.
- *F7* : limites assumées (prénom seul non cherché, élèves cités sans identifiant non cherchés, accents non normalisés).
- *Compteur `masquages_a_decider`* : il compte aussi les marquages cachés ; l'écran l'explique par l'encart
  « réservé(s) à la direction ».
- *Message 409* de la reconstruction rendu neutre (« extrait périmé »).
- *Clé de secrets* : l'assemblage exige désormais `ENT_SECRETS_KEY` hors labo (empreinte à clé M2), comme la remise.

## Non traité ici, à décider par une personne

- **Libellés génériques de la lecture de contrôle** (lot 5) : à relire avec la DPO, aucun changement de code sans elle.
- **`acc_purge` en portée technique** (lot 5) : laissé tel quel. La purge porte sur des partitions mensuelles communes
  à tous les établissements, c'est une opération de plateforme sans `etablissement_id` ; la passer en portée
  établissement voudrait une ligne par établissement et par partition. À trancher si le DPO de l'établissement doit
  lire ce registre (G6 reprend la table avec `cons_execution`).
- **M4, vue de la demande** (`epurer_pour_traiteur`, plus faible) : non modifiée.

## Valeurs prudentes retenues (D-069)

- Une ligne sans identifiant ne reçoit jamais une levée reportée ; un masquage reporté (`masque`) l'est, puisqu'il
  ne fait que retirer.
- Type de pièce inconnu du catalogue → « concerne aussi un tiers ».
- Nom d'un autre élève : `prénom nom` et `nom` seul (≥ 3 lettres) ; le sur-marquage se lève par `droits.traiter`.
- Reprise des alertes : 2 h de recouvrement, 35 jours au plus.

## Compteurs (règle 28)

RLS : **+1** (`acc_balayage`) ; registre : +1 ; `Action::TOUTES` inchangé ; routes inchangées (corps de l'ajout
manuel : `rang` facultatif ; contenu : `elements_reserves_direction`).

## Vérification locale

Voir le message de PR (exécutions sur Postgres 16 local).
