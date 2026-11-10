# Messagerie — lot M0b (état d'implémentation)

Conception : `docs/socles/16_MESSAGERIE.md` §27.1 et §33.2, `PERSONAS_16.md` (sur la branche `ent-mc0hoe`). Migration **481** (repli 480–499). Niveau : **implémenté + testé en local** ; non validé par Florian ; non prêt production.

## Correctifs (tests : `ent-db/src/tests_messagerie_m0b.rs`)
| Défaut | Correctif |
|---|---|
| D21 | un fil ouvert par un élève n'est lu que de son auteur, de qui y a écrit et des titulaires de `messagerie.fils_reserves.lire` |
| D22 | fil du personnel « aux responsables » créé **par foyer** (`ent_msg_fil.foyer_lecteur_id`) ; lecture du personnel : auteur, qui a écrit, titulaires du droit ; avis limité au foyer |
| D23 | élève majeur : fil adressé à l'élève (audience `eleve`), jamais aux parents (aucun droit conservé du socle 4 n'existe encore) |
| D24 | plage calme nationale (20 h–7 h, vendredi 20 h–lundi 7 h, fuseau de l'établissement) : message du personnel invisible hors de son auteur jusqu'à `remise_prevue_le`, avis différé par la chaîne S2-8, annulable par l'auteur, dérogation de la direction (`messagerie.urgence_hors_plage`) avec motif tracé, réglage `ent_msg_reglage` (motif obligatoire pour réduire) |
| D25 | lien créé après une interdiction par libellé : inactif (vue `ent_eleve_lien_communicant`) jusqu'à décision de la direction (`ent_msg_lien_decision`) |
| D26 | action D-017 `messagerie.fils_reserves.lire` (établissement) au lieu des rôles ; défaut admin, direction, administratif, CPE |
| D28 | `messagerie.eleve` et `fils_reserves.lire` retirés aux profils infirmerie et psychologue (attribution refusée, droits effectifs filtrés) |
| MSG-329 | `GET /messages/non-traites` |
| D8 | `POST /annonces` crée un aperçu chiffré (rien n'est envoyé) ; `POST /annonces/{id}/confirmer` publie et met en file ; `POST /annonces/{id}/retirer` (motif) annule les envois en attente |
| D11 | une réponse désarchive le fil ; `POST /messages/fils/{id}/desarchiver` |
| D12 | boîte du personnel paginée par curseur (en-tête `x-curseur-suivant`), recherche sur le sujet côté serveur |
| D13 | personnel prévenu quand une famille écrit (dans l'ENT ; canaux selon préférences ; texte neutre) |
| D14 | échec d'envoi push conservé dans `ent_push_outbox` (statut `failed`) |
| D15 | accusé posé par le côté destinataire seulement, et pour un personnel seulement s'il participe au fil |
| D16 | pièce bornée à 256 Kio même si `ENT_UPLOAD_MAX_MESSAGE` est plus haut (413), 5 pièces au plus |
| D17 | sujet 200 caractères, message 20 000 (service et `CHECK ... NOT VALID`) ; fil de modèle : même chemin que l'ouverture (avis, foyers) |
| D18 | annonces par `classe_id` (affectations du jour) ; libellé sans classe du référentiel : ancien ciblage |
| D20 | test : fusion de fiches, les fils suivent l'élève conservé sans changer de lecteurs ; défusion à l'identique |
| D27 | e-mail, SMS et push neutres pour annonces et campagnes |

## Conformité PRA
E1 (`etablissement_id` sur les nouvelles tables), E5 (aucun nouveau BYTEA ; pièces historiques non migrées), E9 (aucune nouvelle file : la chaîne S2-8 existante est réutilisée), E10 (aucune donnée en JSON), E13 (RLS sur `ent_msg_reglage` et `ent_msg_lien_decision`, `tests_rls` +2).

## Réglages et décisions
- **Plage calme : active par défaut** (recommandation Q-M17, réversible : `PUT /messagerie/reglage`, droit `notifications.gerer`, motif obligatoire pour réduire). Le seed de labo la désactive pour des tests déterministes.
- Aucun réglage irréversible n'a été activé ; durées de conservation, antivirus, boîte d'infirmerie et supervision : **non traités** (hors M0b).
- Interface `ui.rs` : adaptation minimale (aperçu, confirmation du nombre de destinataires, puis publication).

## Points pour Florian
1. Fils du personnel antérieurs à M0b pour un élève à plusieurs foyers : restent lisibles des deux foyers (impossible à départager automatiquement) ; à traiter avec la direction.
2. Élève majeur : ni lui ni ses parents ne reçoivent de push pour un fil du personnel ; droits conservés des parents (socle 4) à définir.
3. Un fil du personnel à un responsable lié par plusieurs comptes sans foyer est créé une fois par responsable.
4. Le professeur ne lit plus les fils des autres (conforme à D21/D22) : à vérifier avec les usages réels avant la démo.
5. Avis différé : relu à l'envoi sur « ne pas solliciter » (plus strict qu'un avis immédiat dans un fil ouvert par la personne).
6. E-mail du carnet à tout responsable communicant : socle 8.
