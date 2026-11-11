# Socle 19 — rapport des lots AG0 et AG1 (code)

**Date :** 2026-11-11 — **Branche :** `ent-socle-19-code-1` (partie de `origin/ENT`, rien poussé sur `ENT`).
**Conception :** `docs/socles/19_AGENDA_RDV.md` et `PERSONAS_19.md` (sur `origin/ent-mc0hoe`, pas encore sur `ENT`).
**Niveau atteint :** *implémenté et testé* (local + CI, voir le run en bas) ; **non validé** par Florian, **pas prêt pilote**, rien déployé, aucune donnée de production touchée.

## 1. Ce qui est livré

| Lot | Contenu | Migration | Tests |
|---|---|---|---|
| **AG0** | Correctifs RD-01 à RD-11 des rencontres parents-professeurs (§27) | `2380_agenda_ag0_rencontres` | `ent-db/src/tests_agenda_ag0.rs` (12 tests, modes normal et **strict RLS**) + `rencontres_parents_professeurs_de_bout_en_bout` inchangé |
| **AG1** | Noyau : types, séries matérialisées, audience sous les droits de l'auteur, participants, réponses, coupon, journal en ajout seul, **vue agenda calculée** par sources, iCal ponctuel minimisé, écran « Mon agenda » (Leptos), 5 actions `agenda.*` | `2381_agenda_evenement` | `ent-db/src/tests_agenda_ag1.rs` (13 tests) + 3 tests d'iCal + `ent-auth` (modèles par défaut) + 6 tests du modèle d'écran |

`MIGRATION_TAG` = `2381_agenda_evenement` (la plus haute du répertoire ; le test de tag trie désormais **numériquement**, le tri lexicographique plaçait `2380_…` avant `667_…`). Aucune migration existante modifiée.

## 2. AG0 — défaut par défaut

| N° | Corrigé par | Test |
|---|---|---|
| RD-01 fil de faits | `ent_rdv_fait` en ajout seul (déclencheur), écrit dans la transaction | `rd01_annuler_puis_relire_le_fil_donne_deux_faits` |
| RD-02 créneau passé, annulation tardive | refus à moins de 12 h ; famille : plus d'annulation à H-2 ni rencontre close | `rd02_…` |
| RD-03 second parent aveugle | état `reserve_pour_cet_enfant` + avis sans nom, même foyer et autorité ; silence sous interdiction ou adresse protégée | `rd03_second_parent_…`, `rd03_interdiction_ou_adresse_protegee_…` |
| RD-04 réservation parallèle = 500 | verrou `(rencontre, enseignant, élève)` + 23505 traduit en conflit | `rd04_…` |
| RD-05 lien rompu | lien revérifié à l'annulation ; `reviser_rdv_liens_rompus` (rendez-vous « à revoir », enseignant prévenu, idempotent, lancé avec les rappels) | `rd05_…` |
| RD-06 rappels hors contexte | déjà corrigé sur `ENT` (passage par établissement via `etablissements_systeme`) ; **test ajouté**, y compris en RLS stricte ; `ligne_planning` relisait avec le pool nu (404 après écriture en RLS stricte) | `rd06_…` |
| RD-07 heure de Paris | heure formatée en SQL avec le fuseau de l'établissement | `rd07_…` (Indian/Reunion → 13:00) |
| RD-08 SMS | plus de SMS pour confirmation et rappel ; SMS seulement pour une annulation par l'établissement à moins de 24 h | `rd08_…` (fonction pure) |
| RD-09 annulation sans motif | motif public obligatoire pour l'organisateur, repris dans l'avis | `rd09_…` |
| RD-10 enseignant sans commande | `bloquer_creneau`, `marquer_presence_rdv` (honoré / non venu), fil lisible de l'enseignant | `rd10_…` |
| RD-11 clés | FK composites, `ON DELETE RESTRICT`, élève référencé | `rd11_…` |
| RD-12 à RD-16 | **non traités, volontairement** (§27) : RD-12 changement d'agrégat en AG4 ; RD-13/14 en AG5 ; RD-15 (droit unique) : les actions `agenda.*` existent, `rencontres.gerer` reste l'alias jusqu'à AG5 ; RD-16 : `ent_rencontre`, `ent_creneau_rdv` et `ent_rdv_fait` sont posées par `ent_rls_poser` (même politique que 480) | — |

Écarts assumés par rapport à la conception : `ent_rdv_fait` porte `creneau_id` (le rendez-vous d'AG0 **est** le créneau) ; les colonnes `annule_motif` « provisoires » ne sont pas créées (le motif vit dans le fil, qui est la source) ; les références vers `ent_user` restent simples (table d'identité commune aux établissements : pas de clé composite possible).

## 3. AG1 — règles testées

| Règle / piège | Test |
|---|---|
| R-AG5 : brouillon et événement « participants » introuvables hors de leur place (404), autre établissement compris ; saisie invalide refusée | `ag1_brouillon_invisible_…` |
| R-AG8 : aperçu sans écriture, confirmation par empreinte, empreinte périmée = conflit, déjà publié = conflit ; la famille d'une autre classe ne voit rien et n'est pas prévenue | `ag1_publication_a_une_classe_…` |
| R-AG8 : audience limitée aux droits de l'auteur (rôle D-017 « mes classes ») ; établissement entier et « tous les personnels » refusés à un rôle limité ; élèves hors droits comptés et exclus ; cible d'un autre établissement refusée | `ag1_audience_limitee_aux_droits_de_l_auteur` |
| Piège 7 : un parent ou un inconnu n'est pas invitable ; publication sans audience = droit de créer seulement | `ag1_invites_nominatifs_…` |
| AGD-128/129/144 : réponses, coupon (bornes, décompte réservé à l'organisateur), empêchement « oui → non » prévient l'organisateur avec le motif | `ag1_reponses_coupon_et_empechement` |
| AGD-115 à 120 : série matérialisée (jours, intervalle, bornes), modification par portée (occurrence / suivantes / toutes), détachement, avis aux invités, conflit de version, annulation avec motif et portée, plus de réponse sur un événement annulé | `ag1_serie_modification_…` |
| Piège 5 / R-AG3 : journal en ajout seul, historique réservé à l'organisateur et à `agenda.journal.lire` | `ag1_journal_en_ajout_seul_…` |
| R-AG1 : plage bornée à 42 jours, source inconnue refusée, source `sorties` **dite** non branchée, filtre par source, partie publique, agenda d'un enfant, agenda d'une classe non nominatif et sans brouillon | `ag1_vue_agenda_bornee_…` |
| R-AG5 : les rendez-vous de rencontre ne sont visibles que de leurs deux parties (l'enseignant ne reçoit pas d'identifiant d'enfant) | `ag1_vue_agenda_rendez_vous_…` |
| R-AG11 : heures murales converties avec le fuseau de l'établissement, journée entière | `ag1_heures_dans_le_fuseau_…` |
| R-AG7, piège 3 : événement restreint = avis et iCal sans titre ni description ni participants ; iCal 404 hors place | `ag1_ics_minimise_…`, tests unitaires d'iCal |
| Droits : types locaux réservés à l'administrateur | `ag1_types_locaux_…` |
| Piège 6 / D-036 : aucune lecture hors établissement ; aucune colonne JSON dans `ent_agenda_*` | `ag1_aucune_lecture_…` |

## 4. Règle 14 (D-051) — défauts NJ-xx fermés par ces deux lots

**Aucun défaut NJ-xx du registre n'est fermé par AG0 ni par AG1, et le dire ici est plus utile que d'en revendiquer.** AG0 corrige des défauts **de l'existant Rust** (RD-xx) ; AG1 est le noyau commun. Les défauts de la **réservation de salles** (NJ-62 lecture de toutes les réservations sans borne, NJ-76 annulation ou modification de la réservation d'un autre, et ND-01 à ND-12) relèvent du lot **AG3**, qui **n'est pas livré** : le module Next.js des salles n'est donc **pas encore remplaçable** (D-057).

Ce qu'AG1 pose pour qu'AG3 les ferme, avec le test déjà écrit pour la partie commune :
- lecture **bornée** (42 jours) et **minimisée selon la place du lecteur** (équivalent de NJ-62/ND-07 pour les événements : `ag1_vue_agenda_bornee_…`) ;
- propriété par **compte** et contrôle côté serveur (équivalent de NJ-76/ND-08 : seul l'organisateur modifie, annule, publie ; un autre compte reçoit 404 : `ag1_serie_modification_…`, `ag1_brouillon_invisible_…`) ;
- horaires au **fuseau de l'établissement** (ND-04), journal **en ajout seul** (ND-12), motif d'annulation **obligatoire**, avis **sans titre** pour un événement restreint, aucun HTML assemblé par l'agenda (les avis sont du texte brut ; l'échappement du rendu e-mail reste celui du socle 8, non revérifié ici) ;
- `ent_salle` reçoit `UNIQUE (etablissement_id, id)` et les événements une FK composite vers elle (ND-10 : une salle utilisée ne se supprime plus en silence).

## 5. Limites connues (honnêtes)

- **AG3 non livré** (salles) : tables `ent_agenda_lieu_regle`, `ent_agenda_reservation` (contrainte d'exclusion), parité M02, reprise : tout reste à faire. Les seuls éléments utiles déjà là : `ent_salle.reservable` (socle 5), la série matérialisée, le journal et les droits.
- Le port **EDT** de la vue est branché (`edt_semaine`) ; les ports **sorties** (42), conseils (23), examens (38), absences (15), `edt::occupe` / `plages_libres` ne le sont pas : la vue dit « pas encore branchée » pour les sorties.
- Publication à une audience : tout dans **une** transaction, avis un à un par `notifier_personne` ; au-delà de quelques milliers de destinataires il faudra la diffusion par lots du socle 8 (pas mesuré).
- L'écran n'a **pas** été essayé dans un navigateur, ni axe-core, ni lecteur d'écran (FRONTEND_LEPTOS §9.3 : à faire). Il est rendu par les mêmes composants que l'EDT de la semaine ; routes : `/web/mon-agenda` (`/web/agenda` reste « Travail à faire », cahier de textes — la conception prévoyait `/web/agenda`, collision évitée).
- La navigation est ajoutée pour tout le personnel et les familles / élèves ; un compte sans droit voit simplement moins d'éléments (le serveur décide).
- `agenda.evenements.publier` est donné à Admin, Direction, CPE ; un professeur principal l'obtient par un rôle paramétré « mes classes » (testé).
- Écart à signaler : les **élèves** (comptes élèves) ne reçoivent pas d'invitation dans AG1 (audience = responsables) ; l'EDT de l'élève apparaît dans sa vue.
- Les routes `/agenda/*` n'ont pas de test HTTP dédié ; les services sont testés, les routes ne font que traduire.

## 5 bis. D-058 (message du pilote reçu pendant le lot, non lu dans le dépôt : `ent-mc0hoe` seulement)
La réservation devra porter sur **toutes** les salles du socle 5, salles de classe comprises, une salle de classe étant réservable quand l'EDT réel la laisse libre (le cours prime ; remplaçant nommé = réservation tombée et demandeur prévenu), par les ports des socles 10 et 15. **Non fait dans AG1** : AG1 ne pose que la FK composite vers `ent_salle` et la lecture de l'EDT par `edt_semaine`. Le port « occupation / disponibilité d'une salle » (socles 10 et 15) n'existe pas sous forme de port : il reste à créer avec AG3, en adaptateur « source absente » testable si nécessaire. À signaler au pilote.

## 6. À contrôler par le pilote

`grep -i jsonb` sur 2380–2381 (aucune), `ON DELETE` des nouvelles FK (RESTRICT / CASCADE sur les tables filles de l'agenda / SET NULL sur `eleve_*` anonymisables), droits `agenda.*` dans `modele_par_defaut`, registre des tables (+10 tables), `domain-eleve::fusion` (`ent_rdv_fait` reste sur la fiche absorbée).
