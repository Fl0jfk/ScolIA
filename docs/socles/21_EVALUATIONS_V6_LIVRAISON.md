# Socle 21, lot V6 « appréciations » : livraison

Date : 2026-11-28. Branche `claude/project-thread-tbh550`, partie d'`origin/ENT` 06333a59. Conception :
`21_EVALUATIONS.md` §12 (types, règles, compléments personas), §17.10 (EVA-194 à EVA-206), tableau des lots (« V6
appréciations : types, versions, brouillons, banque de phrases, contrôles, saisie en série, fenêtre et clôture »).

**Niveau** : implémenté et testé en local (PostgreSQL 16) ; contre-revue, CI et versement à faire ; non validé par
Florian ; pas prêt pilote.

## 1. Ce que le lot apporte

Les tables existaient depuis V1 (465 : `eva_appreciation`, son historique en ajout seul, la banque de phrases), mais
aucun service ne les écrivait. V6 apporte le service, les routes et les garanties qui manquaient.

| Action | Qui (défaut) | Route | Ce qui est garanti |
|---|---|---|---|
| EVA-194 appréciation de cours | professeur en saisie sur le cours (titulaire, co-enseignant, suppléant), pour un élève du cours dans la période | `POST /eva/v6/appreciations` (`type` = `cours`) | une seule appréciation vivante par (période, élève, type, cours) ; chaque réécriture est une version ; l'éditeur de chaque version est gardé |
| EVA-201 générale | professeur principal de la classe de l'élève, ou `bulletins.publier` | idem (`generale`) | 600 caractères au plus |
| EVA-203 vie scolaire | CPE, ou `bulletins.publier` | idem (`vie_scolaire`) | — |
| Appréciation de dispositif (§12.3) | `bulletins.publier` | idem (`dispositif`) | — |
| Appréciation intermédiaire (S-112) | professeur du cours | idem (`intermediaire`) | période ouverte seulement ; jamais montrée à la famille (la vue `ent_note_appreciation_v` ne lit que cours et générale) |
| EVA-205 « pas d'appréciation (professeur absent) » | direction | idem, texte libre | correction motivée si la fenêtre est fermée |
| Brouillon, validation | l'auteur | `valider: false` puis `true` | un brouillon n'est lu que par son auteur ; une validée ne redevient pas brouillon (base) |
| Jeter un brouillon | son auteur | `POST /eva/v6/appreciations/{id}/jeter` | rien ne se supprime : état `jete` ; la clé se réécrit ensuite |
| Correction après la fenêtre | direction (`notes.corriger`) | `POST /eva/v6/appreciations` avec `motif_correction` | motif de 10 à 500 caractères, inscrit dans `eva_appreciation_correction` (ajout seul) avec l'état de la période ; jamais un brouillon ; impossible une fois la période close |
| EVA-195 saisie en série | professeur du cours (lecture : `notes.lire`) | `GET /eva/v6/cours/{id}/serie?periode_id=` | mêmes élèves que la grille V3 (élèves protégés retirés), moyenne du cours à côté, appréciation validée ou brouillon de l'acteur, avertissements par ligne ; un brouillon d'un autre est signalé, jamais montré |
| EVA-200 contrôles de cohérence | l'auteur | réponse de chaque saisie, et dans la série | non bloquants (§2) |
| Historique des versions | l'auteur, le PP, la direction | `GET /eva/v6/appreciations/{id}/historique` | les versions « brouillon » d'un autre éditeur n'apparaissent jamais ; motif d'une correction affiché |
| EVA-204 relire les appréciations d'un élève | PP de sa classe, `bulletins.publier`, `notes.corriger` | `GET /eva/v6/eleves/{id}/appreciations?periode_id=` | validées seulement (plus les brouillons de l'acteur), dans l'ordre du bulletin |
| EVA-204 remarque à l'auteur | idem | `POST /eva/v6/appreciations/{id}/remarques` | vise la version relue (conflit si elle a changé), adressée à son éditeur, jamais à la famille ; ne se modifie pas |
| Mes remarques, les traiter | destinataire | `GET /eva/v6/remarques`, `POST /eva/v6/remarques/{id}/traiter` | traitée une fois, par le destinataire seulement |
| EVA-196 banque de phrases | chaque membre du personnel, pour lui | `GET/POST /eva/v6/banque`, `POST /eva/v6/banque/{id}/retirer` | personnelle et propre à l'établissement (R-V35) ; 500 phrases au plus ; un doublon rend la phrase existante |
| Mots à éviter (S-117) | lecture : personnel ; réglage : `bulletins.publier` ou `notes.corriger` | `GET/POST /eva/v6/mots-interdits`, `POST /eva/v6/mots-interdits/{id}/retirer` | liste courte (100 au plus), casse ignorée |

Toutes les routes sont réservées au personnel : la famille et l'élève reçoivent 403 (test de séparation). Ils continuent
de lire les appréciations publiées par les lectures existantes (vue `ent_note_appreciation_v`, appréciations closes
pour la classe de l'élève).

### 1.1 Fenêtre (état de la période de la classe de l'élève, R-V29)

| Type | L'auteur écrit tant que | Ensuite |
|---|---|---|
| intermédiaire | `ouverte` | refus (conflit) |
| cours | `ouverte` ou `notes_arretees` | direction seule, avec motif, jusqu'à `conseil_tenu` inclus |
| générale, vie scolaire, dispositif | jusqu'à `appreciations_closes` inclus | direction seule, avec motif |
| tous | — | `close` : plus rien ici (corrections après publication, V8) |

L'état est lu au jour de l'établissement ramené dans la période, sinon à la fin de la période. Un élève sans classe
pour la période ne reçoit aucune appréciation (refus explicite).

## 2. Contrôles de cohérence

Déterministes, sans IA, non bloquants (`ent_types::evaluations_v6::controler`, compilable en WebAssembly pour l'écran) :

| Code | Déclencheur |
|---|---|
| `identique` | le même texte (casse, accents et ponctuation ignorés) pour plus de 5 élèves du même cours, de la même période et du même type ; seuls les textes validés et les brouillons de l'acteur comptent |
| `autre_prenom` | prénom (d'usage) d'un autre élève du cours ou de la classe, mot entier, 3 lettres au moins |
| `eloge_sous_moyenne` | « excellent », « très bien », « félicitations »… avec une moyenne du cours sous 8 |
| `sante` | terme de santé ou de diagnostic (« dyslexique », « TDAH »…) : « information de santé, à retirer » (R-V18) |
| `disciplinaire` | « exclusion », « sanction », « conseil de discipline »… (CE-93, R-V32) |
| `mot_interdit` | un mot de la liste de l'établissement |

Un texte trop long est **refusé** avec la limite (400 caractères, 600 pour la générale), jamais tronqué.

## 3. Migration 702

`702_eva_v6_appreciations.sql` (plage 701–719 du socle 21, fin du tableau de `migrate.rs`) : trois tables avec RLS.

- `eva_appreciation_mot_interdit` : mot unique par établissement (casse ignorée).
- `eva_appreciation_remarque` : remarque sur une version, destinataire, auteur ≠ destinataire ; déclencheur : ni
  suppression, ni modification, sauf le passage unique à « traitée » ; pas de vidage.
- `eva_appreciation_correction` : version, état de la période, motif (10 à 500 caractères), auteur ; ajout seul.

Registre des données : les trois tables au domaine `evaluations_pedagogie` ; remarques et corrections personnelles,
**hors export** (internes à l'équipe, comme l'historique des versions). Compteur RLS +3.

## 4. Décisions prises par défaut (D-069)

1. **Droits** : le catalogue n'a pas d'actions `appreciations.*` (§14.1). Plutôt que d'en créer (catalogue, rôles,
   tests de parité), le lot réutilise les droits existants : saisie sur le cours (`ent_prof_sur_cours`), professeur
   principal (`ent_service_enseignement.professeur_principal`), rôle CPE, `bulletins.publier`, `notes.corriger`.
   Les actions dédiées restent à créer si un établissement veut confier une appréciation à un rôle libre.
2. **Appréciation de dispositif** : aucun « coordonnateur ULIS, SEGPA ou UPE2A » n'existe dans le modèle ; elle est
   réservée à `bulletins.publier` en attendant.
3. **Vie scolaire** : tout CPE de l'établissement, sans rattachement aux classes (aucun n'existe).
4. **Fenêtres** : celles du §12.1 (cours jusqu'à la clôture des appréciations ; générale et vie scolaire jusqu'au
   conseil, « souvent rédigée pendant le conseil ») ; dispositif comme la générale ; intermédiaire pendant la période
   ouverte. Correction de la direction avec un motif d'au moins 10 caractères (comme la réouverture de période).
5. **Limites** : 400 caractères (cours et autres), 600 (générale), faute de modèle de bulletin réglé.
6. **Brouillon d'un autre** sur la même clé : la saisie est refusée en conflit (« il doit le valider ou le jeter ») ;
   personne, pas même la direction, ne lit ni ne jette le brouillon d'un autre.
7. **Marque « rédigée avec aide »** (`aide_ia`) : jamais modifiée par ce lot (posée par V11) ; elle reste sur les
   versions suivantes.
8. **Ancienne route** `POST /evaluations/appreciations` (écrit `ent_note_appreciation`, table de reprise) : laissée en
   place, l'écran Leptos actuel l'appelle. Sa fermeture (410) et la bascule de l'écran sur `/eva/v6/*` vont avec les
   écrans du lot (M3, « bascule des écrivains »).
9. **Aucun événement** n'est publié : aucun consommateur ne les attend encore (les rappels de clôture sont au lot V8).

## 5. Tests

- `ent-types` : 7 tests des contrôles (normalisation, seuils, mots entiers, prénom de l'élève lui-même ignoré, limites).
- `ent-db`, unitaires : fenêtres par type, refus d'un texte trop long sans troncature.
- `ent-db`, sur base (`tests_evaluations_v6.rs`) :
  - `saisie_versions_brouillons_fenetres_et_corrections` : brouillon privé, droits (autre professeur, élève hors du
    cours, famille, cours inconnu sans oracle), refus (type, cours absent, 401 caractères), conflits de version,
    validation, réécriture identique sans nouvelle version, historique filtré pour la direction, intermédiaire, fenêtre
    après l'arrêt des notes puis après la clôture, correction motivée journalisée, **vue famille** (la version validée
    apparaît après la clôture, l'intermédiaire jamais), générale du PP, vie scolaire de la CPE, relecture du PP, conseil
    tenu ;
  - `serie_controles_banque_mots_interdits_et_remarques` : mots interdits (droit, doublon), contrôles à la saisie,
    série du professeur et de la direction (brouillon d'un autre signalé, non montré, clé refusée), remarques (version,
    droit, destinataire, traitée une fois, non modifiable en base), banque (personnelle, doublon, retrait), dispositif,
    brouillon jeté puis clé réécrite.
- `ent-api`, séparation : les 14 routes répondent 403 à la famille et à l'élève.
- Voisins : RLS (compteur), registre des données, migrations, suites d'évaluations V0 à V5.

## 6. Reste

- **V11** : propositions de l'IA (EVA-197, 198, 202), qui naîtront en brouillon `aide_ia`.
- **Écrans Leptos** de la série et de la relecture, puis bascule de l'ancienne route ; **applis** (dictée vocale, V9).
- **Prénom d'usage** (R-V36) : contrôle « ancien prénom » à brancher quand l'état civil versionné sera lisible ici.
- **V8** : rappels J-7 / J-2 de clôture des appréciations, par classe (EVA-239) ; corrections après publication.
