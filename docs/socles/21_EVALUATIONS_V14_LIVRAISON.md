# Socle 21, lot V14 « cohabitation de l'an 1 » : livraison

Date : 2026-11-28. Branche `claude/project-thread-llhtj5`, partie d'`origin/ENT` 06333a59. Conception : `21_EVALUATIONS.md`
§16, §17.15 (EVA-233 à EVA-237), CE-89, tableau des lots (« V14, à livrer avant le premier bulletin du pilote »).

## 1. Ce que le lot apporte

L'an 1 du lycée pilote (D-032), les notes et les bulletins restent dans Charlemagne. Jusqu'ici, rien ne permettait à
ScolIA de distribuer ces bulletins. Le lot couvre le parcours complet du secrétariat et de la direction.

| Action | Qui (défaut) | Route | Ce qui est garanti |
|---|---|---|---|
| EVA-234 ouvrir un dépôt pour une période | secrétariat, direction (`imports.appliquer` ou `bulletins.publier`) | `POST /eva/v14/lots` | période commencée ; logiciel en liste fermée |
| EVA-234 déposer un bulletin PDF | idem | `POST /eva/v14/lots/{id}/fichiers` | PDF reconnu par sa signature (politique de fichiers commune) ; même fichier jamais déposé deux fois pour une période ; clé d'objet produite par le serveur |
| EVA-234 rapprochement automatique | — | au dépôt | INE (annoncé ou lu dans le nom du fichier), sinon nom + prénom + date de naissance ; **un seul candidat** scolarisé pendant la période, sinon file « à rapprocher » |
| EVA-234 voir la file et l'aperçu | secrétariat, direction | `GET /eva/v14/lots?periode_id=`, `GET /eva/v14/lots/{id}`, `GET /eva/v14/bulletins/{id}/apercu` | personnel habilité seulement |
| EVA-234 rapprocher à la main, écarter | secrétariat, direction | `POST /eva/v14/bulletins/{id}/rapprocher`, `/ecarter` | élève scolarisé pendant la période ; écart motivé ; jamais sur un bulletin distribué |
| EVA-246 contrôle avant distribution | direction (`bulletins.publier`) | `GET /eva/v14/lots/{id}/controle` | rien n'est écrit ; compte des destinataires, remplacements, fichiers restés à rapprocher, élèves **sans destinataire en ligne** (à remettre sur papier) |
| EVA-235 distribuer un dépôt | direction | `POST /eva/v14/lots/{id}/distribuer` | seuls les bulletins rapprochés partent ; document au coffre de l'élève (tiroir famille, `source = 'bulletin'`), lu sous le droit « voir les notes » comme un bulletin ScolIA ; destinataires relus à l'instant et figés ; avis |
| retirer un bulletin distribué | direction | `POST /eva/v14/bulletins/{id}/retirer` | motif obligatoire ; le document sort du coffre des familles (`restreint`) |
| lecture famille et élève | parent (lien « voir les notes »), élève | `GET /eva/v14/eleves/{id}/bulletins`, `GET /eva/v14/bulletins/{id}/pdf` | liste vide ou 404 sans droit : aucun oracle d'existence |
| EVA-233 exporter l'assiduité d'une classe | secrétariat, direction (`exports.officiels`) | `GET /eva/v14/assiduite?periode_id=&classe_id=[&format=csv]` | bornée à la période et à aujourd'hui ; effectif de la classe de rattachement (V5, D15) ; demi-journées du décompte versionné (AS4) ; CSV échappé contre l'injection de formule ; chaque export journalisé |
| EVA-237 bandeau « où sont les notes » | direction | `GET`, `PUT /eva/v14/cohabitation` | lien `https://` seulement ; lisible de tout membre |

## 2. Modèle (migration 701, plage 701–719 du socle 21)

Six tables, toutes avec RLS (`ent_rls_poser`) : `eva_tiers_lot`, `eva_tiers_bulletin`, `eva_tiers_destinataire`,
`eva_tiers_journal`, `eva_tiers_export`, `eva_cohabitation`.

Garanties posées **en base** :
- un bulletin « à rapprocher » n'a pas d'élève ; un bulletin distribué a un élève, une voie de rapprochement, une date,
  un auteur et un document (CE-89 : un bulletin non rapproché ne peut pas être distribué, même par une requête directe) ;
- un seul bulletin distribué par élève et par période (index unique partiel) ; le même fichier (empreinte) n'est déposé
  qu'une fois par période, sauf s'il a été écarté ;
- fichier, lot, période et date de dépôt figés ; aucune suppression ; un bulletin remplacé, retiré ou écarté ne change
  plus ; un bulletin distribué ne revient jamais en arrière ;
- journal, destinataires et exports en ajout seul.

Registre des tables (`registre_donnees.rs`) : domaine `evaluations_pedagogie` ; `eva_tiers_bulletin` porte un objet du
magasin (`Fichiers::Objet`). Compte RLS de `tests_rls.rs` : + 6.

## 3. Décisions prises par défaut (D-069, à relire quand Florian le souhaite)

1. **Pas de nouvelle action de droit.** Dépôt et rapprochement : `imports.appliquer` ou `bulletins.publier` ;
   distribution et retrait : `bulletins.publier` (direction par défaut, un rôle paramétré peut l'accorder au
   secrétariat) ; export : `exports.officiels`. La conception nommait `echanges.importer` et `echanges.exporter`, qui
   n'existent pas encore (socle 9) : à remplacer quand ils existeront.
2. **Destinataires** : le droit `recevoir_bulletins` de la conception (R-V47) n'existe pas encore sur le lien. Par
   prudence, la règle est celle des bulletins produits par ScolIA : `peut_voir_notes` sur un lien actif, plus le compte
   de l'élève. S'y ajoute l'exclusion d'un lien sous interdiction de communication (`ent_eleve_lien_communicant`). Le
   parent sans exercice de l'autorité parentale qui garde « voir les notes » reçoit donc le bulletin (R-V43).
3. **Avis** : envoyés aux destinataires figés qui le restent au moment du traitement (lien sollicitable avec « voir les
   notes », ou compte de l'élève). Le texte de l'avis ne contient aucune note.
4. **Rapprochement** : jamais sur un critère ambigu. Un INE annoncé qui contredit celui de l'élève empêche
   l'appariement par l'identité. Un deuxième fichier pour un élève dont un bulletin attend déjà la distribution reste
   « à rapprocher », et le rapprochement manuel est refusé tant que le premier attend.
5. **Format de l'INE lu dans le nom du fichier** : 9 chiffres et 2 lettres, ou 10 chiffres et 1 lettre ; un seul segment
   plausible, sinon rien. Le nommage réel des fichiers Charlemagne reste à confirmer sur l'exemple F-07 **[NV]**.
6. **Format de l'export d'assiduité** : CSV `;`, UTF-8 avec BOM, colonnes INE, nom, prénom, date de naissance, classe,
   bornes, demi-journées, justifiées, non justifiées, retards. Le format d'import de Charlemagne, s'il existe, reste à lire
   sur F-07 **[NV]**. Retards : lignes d'appel à l'état « retard » sur les jours de la période.
7. **Période** : celle du référentiel (`ent_periode`), comme le moteur V1 à V5. Dépôt et distribution refusés avant le
   début de la période ; pas d'autre condition (c'est le logiciel tiers qui arrête ses notes).

## 4. Ce qui reste (hors de ce lot)

- **EVA-236** : moyennes et appréciations de période importées du logiciel tiers en lecture (résultats externes de
  période). Le format de l'export Charlemagne n'est pas connu **[H]** ; à faire avec l'exemple F-07.
- **EVA-248** : lot d'impression des bulletins sans destinataire en ligne. Le contrôle les liste déjà.
- Découpage d'un PDF de classe en bulletins d'élèves (la conception l'évoque) : non fait, un fichier par élève.
- Écrans Leptos du secrétariat et de la famille, et écran des applis : à faire, les routes sont prêtes.

## 5. Tests

- `ent-db/src/tests_evaluations_v14.rs` (base locale, feature `labo`) : rapprochement par INE et par identité, INE
  contradictoire, fichier non rapproché jamais distribué, parent privé des notes exclu (liste, PDF, coffre), contrôle sans
  écriture, rapprochement manuel puis distribution, écart, journal en ajout seul ; élève destinataire et avis aux seuls
  destinataires ; nouvelle version qui remplace, retrait motivé, garde de la base ; droits et refus (famille, professeur,
  CPE, secrétariat qui ne distribue pas, PDF seulement, doublons) ; bandeau (https seulement, direction) ; export borné et
  journalisé.
- Tests unitaires : INE lu dans le nom du fichier, CSV échappé.
- `ent-api/tests/separation.rs` : les 13 routes `/eva/v14/*` refusent la famille et l'élève là où il le faut.

## 6. Livraison

Voir la PR brouillon vers `ENT` (contre-revue à faire avant versement, migration 701 à inscrire dans
`migrations_versees.txt` au versement).
