# Socle 21, lot V7 « famille et élève » : livraison

Date : 2026-11-28. Branche `claude/project-thread-9zu5zq`, partie d'`origin/ENT` 06333a59. Conception :
`21_EVALUATIONS.md` §10 (publication aux familles), §17.6 (EVA-159 à EVA-168), §17.15 « Élèves et familles (lot V7) »
(EVA-269 à EVA-274), CE-59, R-V7, R-V8, R-V12, R-V14, R-V26, R-V46, S-113, S-130 ; tableau des lots (« V7 famille et
élève : relevé avec explication, statistiques, rattrapages, projet d'évaluation, demande de révision ; avis du socle 8 »).

**Niveau** : implémenté et testé en local (PostgreSQL 16, base neuve de test) ; contre-revue, CI et versement à faire ;
non validé par Florian ; pas prêt pilote.

## 1. Ce que le lot apporte

Jusqu'ici, la famille et l'élève lisaient l'ancien relevé (`/evaluations/eleves/{id}/releve`, moyennes et statistiques
de matière) et les résultats externes publiés (V5). Le moteur V2 savait déjà calculer une moyenne « publiées seulement »
avec son explication (`eva_v1_moyennes_famille_for_actor`), sans aucune route. La table des demandes de révision existait
depuis 466 (V1), sans service. V7 ouvre ces lectures et ce geste, sur le nouveau modèle.

| Action | Qui | Route | Ce qui est garanti |
|---|---|---|---|
| EVA-159, EVA-160 relevé et moyennes provisoires expliquées | élève (son compte) ; parent au lien « voir les notes » | `GET /eva/v7/eleves/{id}/releve?periode_id=` | évaluations **publiées** seulement (R-V7), même moteur que le bulletin ; une phrase par évaluation (« comptée », « absent, non compté »…) puis la conclusion ; la moyenne est dite **provisoire** avec la date d'arrêt tant que la classe de l'élève n'a pas arrêté ses notes ; commentaire du professeur publié avec la note ; ni note privée, ni motif, ni auteur |
| EVA-161 statistiques de l'évaluation | idem | dans le relevé | `Statistiques::pour_famille` seulement : jamais sous le seuil (5 par défaut, plancher 3), jamais si l'établissement les masque (S-113), médiane seulement si elle est choisie |
| §10.1 retrait de publication | idem | dans le relevé (`retirees`) | une évaluation retirée après sa première publication est dite « résultats retirés pour vérification » ; ni sa note, ni le motif du retrait |
| EVA-269 dates prévues | idem | dans le relevé (`calendrier`) | début, fin, date d'arrêt (effective si la classe est arrêtée), date prévue du conseil (référentiel des périodes) |
| EVA-162 évaluations annoncées, rattrapages à faire | idem | `GET /eva/v7/eleves/{id}/a-venir` | annoncées, ouvertes, non publiées, dans les 60 jours, pour un cours dont l'élève est dans le public à la date (et dans la restriction s'il y en a une) ; rattrapages et remplacements convoqués non échus : date, heure, lieu, modalité ; jamais de note |
| EVA-163 dispenses | idem | dans `a-venir` | dispenses en vigueur ou à venir : matière (ou « tous les enseignements ») et nature, en mots ; jamais le motif ni la pièce |
| EVA-164 projet d'évaluation et coefficients | idem | `GET /eva/v7/eleves/{id}/projet?periode_id=` | règles de l'établissement en phrases (échelle, pondération, absence jamais zéro, non rendu, bonus, sous-matières, générale, annuelle, arrondis, statistiques) ; coefficients des matières de la classe et des groupes de l'élève ; un coefficient manquant est dit (`null`), jamais inventé |
| EVA-270 simuler sa moyenne | **élève seulement** (S-130) | `POST /eva/v7/eleves/{id}/simulation` | « si j'ai 14 au prochain devoir » : même moteur, mêmes règles, sur les seules notes publiées ; moyenne de matière et générale avant / après ; **rien n'est écrit** (transaction annulée) ; matière non suivie refusée ; note entre 0 et le barème |
| EVA-166 demander une révision | élève ; parent au lien « voir les notes » | `POST /eva/v7/revisions` | sur un résultat d'une évaluation **publiée** et ouverte (sinon « introuvable », aucun oracle) ; message de 3 à 1 000 caractères ; une demande ouverte par résultat et par demandeur ; 20 demandes ouvertes au plus par demandeur ; jusqu'au conseil de la période de l'élève (ensuite : « adressez-vous à la direction ») ; ce n'est pas un recours (R-24) |
| Mes demandes, retirer | leur auteur | `GET /eva/v7/eleves/{id}/revisions`, `POST /eva/v7/revisions/{id}/retirer` | chacun ne voit que les siennes : jamais celles de l'autre parent ni de l'élève (R-V46) ; retrait tant qu'elle est ouverte |
| Demandes à traiter, répondre | professeur du cours (titulaire, co-enseignant, suppléant en lecture) ; direction (`notes.corriger`) | `GET /eva/v7/revisions[?etat=toutes]`, `POST /eva/v7/revisions/{id}/repondre` | le personnel voit l'élève et le **rôle** du demandeur (parent ou élève), pas son identité ; une réponse, une fois ; la réponse **ne change pas la note** (le professeur corrige par la grille, la direction après l'arrêt : CE-59) |
| Avis du socle 8 | — | consommateur `d6.evaluations` (`reactions.rs`) | demande → les professeurs du cours ; réponse → le seul demandeur, s'il a encore le droit de lire les notes ; l'avis relit la demande au traitement (une demande déjà close n'appelle personne) ; ni note, ni message, ni réponse dans l'avis |
| EVA-272 glossaire | tout compte connecté | `GET /eva/v7/glossaire` | statuts, mentions, statistiques, en mots simples (statique, `ent_types::evaluations_v7::glossaire`) |
| R-V26 journal des consultations | — | chaque relevé lu | une ligne `eva_consultation` (`releve`, lecteur, rôle) ; lue par la direction seulement |

**Le personnel n'utilise pas ces lectures** : `releve`, `a-venir`, `projet`, `revisions` de l'élève et la demande de
révision lui répondent 403. Il lit par les grilles (V3) et la vue de l'élève (V5) ; aucune vue « famille » ne sert de
raccourci à un droit.

## 2. Migration 703

`703_eva_v7_famille_eleve` (plage d'extension 701–719 du socle 21 ; 701 est à V14, 702 à V6, non versées). Aucune table
(compteur RLS inchangé) ; `eva_revision_demande` (466) reçoit :
- `demandeur_role` (`parent` | `eleve`, figé à la demande ; `NULL` pour une ligne antérieure) ;
- `repondue_par` et `retiree_le` ;
- l'index unique partiel « une demande ouverte par (résultat, demandeur) » et deux index de lecture ;
- le garde `eva_revision_demande_garde` : ni suppression, ni réécriture du message, de la cible, de l'auteur ou de la
  date ; une seule sortie de l'état « ouverte » (réponse avec auteur et date, ou retrait avec date et sans réponse).

466 est versée et ne change pas. Fonction avec `SET search_path = pg_catalog, public, pg_temp`, relations `public.`.
Fin du tableau de `migrate.rs` (règle 38). `MIGRATION_TAG` inchangé. Registre des données : `eva_revision_demande` reste
« tout exportable » (les nouvelles colonnes sont communiquées au demandeur).

**Chaîne avec V14 (701, PR 175) et V6 (702, PR 176)** : les trois migrations sont indépendantes ; à la fusion de chaîne,
l'ordre du tableau est 701, 702, 703.

## 3. Décisions prises par défaut (D-069)

1. **Fin des demandes de révision au conseil** (état `conseil_tenu` de la classe de l'élève à la date de l'évaluation) :
   après, la correction est un geste de la direction (V8), pas une conversation.
2. **Rôle du demandeur, pas son identité**, pour le personnel (minimisation) : le professeur répond, l'avis va au bon
   destinataire. Le nom du demandeur reste lisible par la direction en base si besoin.
3. **Plafond de 20 demandes ouvertes par demandeur** et une par résultat, sous verrou consultatif par demandeur.
4. **Personnel exclu des lectures famille** (403) plutôt qu'une « vue comme la famille » : à décider avec les écrans.
5. **Statistiques par évaluation** seulement (moyenne de classe, extrêmes, médiane choisie) ; la statistique de matière
   reste dans l'ancien relevé jusqu'à la bascule des écrans (V12).
6. **Horizon des évaluations annoncées : 60 jours.**
7. **Simulation** : la note imaginée compte comme une note ordinaire de la matière (coefficient 1 et sur 20 par défaut,
   modifiables) ; elle masque une moyenne externe comme le ferait une vraie note (R-V38).
8. **Avis du professeur** : tous les professeurs des services du cours ; un suppléant n'est pas prévenu (il lit la
   liste). À revoir avec le socle 8 (préférences d'avis, EVA-167).

## 4. Vérifications locales

PostgreSQL 16, base neuve `ent_v7_test`, Rust 1.99.0.
- `ent-db`, 3 scénarios V7 sur base : relevé (publié seulement, explication, statistiques à 5 élèves, masquées par le
  réglage, retrait, provisoire puis arrêté, conseil tenu, droits, journal) ; révisions (refus, unicité, R-V46,
  personnel, réponse unique, note inchangée, retrait, événements neutres, avis, garde de la base) ; à venir, projet,
  simulation (aucune écriture).
- `ent-types` : 4 tests (messages, bornes de la simulation, dates en français, glossaire).
- `ent-api`, séparation : personnel 403 sur les lectures famille, autre foyer et parent sans « notes » 403, simulation
  refusée au parent, routes du personnel refusées à la famille, retrait d'une demande d'un autre « introuvable »,
  glossaire lisible de tous.
- Voisins : gardes statiques (isolation SQL, jour UTC, écritures) ; RLS et compteur, registre des données, migrations,
  `search_path`, réactions ; évaluations V0 à V6 existantes (le moteur `moyennes_tx` délègue désormais à
  `moyennes_tx_avec`, sans changement de comportement).
- clippy 1.99.0 : les 3 commandes de la CI ; `cargo metadata --locked` dans `ent-rust/web`.

## 5. Reste

- Écrans Leptos (relevé, à venir, projet, simulateur, demandes) et applis (V9, V12).
- EVA-165 (bulletins au coffre) : socles 23 et 7 ; EVA-167 (choix des avis) : socle 8 ; EVA-168 (élève majeur) : socle 4 ;
  EVA-271 (copie corrigée) : socle 20 ; EVA-273 (export pour le droit d'accès) : socle 27 ; EVA-274 (plusieurs
  établissements côte à côte) : applis.
- Bascule de l'ancien relevé famille vers `/eva/v7/…/releve` (V12).
- Préexistant, signalé : l'extrait RGPD de l'élève exporte toutes les demandes de révision portant sur ses résultats,
  y compris celles de l'autre parent (R-V46) ; à trancher au socle 27.
