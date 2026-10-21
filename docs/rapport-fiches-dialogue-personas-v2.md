# Fiches de dialogue — audit personas et parcours

**Date :** 21 octobre 2026
**Branche :** `rust-lab` (commit local, aucun push)
**Base :** PostgreSQL locale de test `scolia_migrate` uniquement
**Périmètre :** parcours élève / parents / professeur principal / direction / secrétariat
**Repro :** `scripts/audit-fiches-dialogue-personas-v2.ts`
**Résultats bruts :** `docs/rapport-fiches-dialogue-personas-v2.json`

---

## 1. Synthèse

L’audit a joué **15 scénarios** et **55 assertions métier** sur le vrai workflow : familles, conflits, accords tacites, refus, appel, avenants, Terminale, 1re ST2S et vue élève.

**Résultat : 55 assertions OK, 0 assertion en échec, 0 manque produit confirmé.**

Les parcours « heureux » et les cas sensibles tiennent : envoi, saisie, correction jusqu’à la clôture, gel, conseil, publication, acceptation, avenant, Terminale, ST2S, refus et **appel jusqu’à la décision extérieure**. Les deux manques de visibilité de l’audit précédent sont levés :

- le staff dispose d’un **dossier famille** avant de décider (vœu, commentaire, désaccord, signatures, documents, historique des liens) ;
- l’élève dispose d’un **visu strictement en lecture seule** (`/eleve/fiches-dialogue`), sans action ni donnée de contact.

Le faux positif d’envoi est également corrigé : sans SMTP, un échec est compté comme échec, la fiche reste « à envoyer » et `lastSentAt` n’est pas falsifié.

### Correctifs appliqués

1. **Résumés lisibles côté parents** : les écrans « confirmation parentale » et « décision définitive » n’affichent plus `JSON.stringify(...)`. Ils montrent les champs, les libellés du catalogue, le commentaire, la destination, l’avis et l’avenant.
2. **Bouton « Modifier ma réponse » réparé** : après un dépôt, l’API renvoie un nouveau jeton d’édition (60 min). Le bouton fonctionne sans repasser par l’identification.
3. **Écran de récupération** : si un lien est invalide, le parent peut cliquer sur « Retrouver ma fiche » au lieu de rester bloqué.
4. **Jeton personnel par parent** : l’envoi crée un lien et un code distincts pour chaque responsable. Le dépôt est attribué à la bonne adresse ; après le dépôt, le lien de l’autre parent est révoqué et remplacé par la demande de confirmation. Le mail précise que le lien est personnel.
5. **Correction jusqu’à la clôture** : le jeton de saisie du déposant n’est plus révoqué après le dépôt. Le parent peut rectifier ses vœux depuis son e-mail initial, sans redemander un code, jusqu’à la date de clôture de l’étape.
6. **Accord tacite lisible** : le jeton du 2e parent est conservé avec 30 jours de grâce après l’échéance. Il affiche « Accord parental enregistré » et une confirmation tardive est idempotente, au lieu de « Lien invalide ou expiré ».
7. **Publication réservée à la direction** : `canPublishFichesDialogue` vérifie le rôle serveur dans la route `conseil`. Un professeur ou un CPE ne peut plus publier en tapant « Direction » dans le champ texte ; il peut seulement préparer un brouillon.
8. **Sortie d’appel** : dossier complétable dans le temps (commission, dates, informations, documents attendus/reçus), décision **maintien / réformation**, PDF `appel_decision`, clôture de la fiche et notification des parents. Un patch partiel ne perd plus les champs déjà saisis, et une date erronée peut être effacée.
9. **Dossier famille côté staff** : `getFdFicheDossier` + `GET /api/fiches-dialogue/fiches/[id]` réservés à `canConseilFichesDialogue`. L’écran campagne affiche les réponses lisibles, le désaccord, les signatures, les documents et l’historique des liens, sans exposer les jetons.
10. **Préremplissage du conseil** : à l’ouverture, le formulaire reprend le dernier vœu de la famille ; pour un avenant, il reprend la dernière proposition du conseil.
11. **Notifications de conflit et d’accord tacite** : alerte staff + information des deux parents lors d’un désaccord ; information des deux parents à l’échéance de l’accord tacite. Envoi best-effort : l’échec SMTP ne bloque pas le workflow et n’est jamais horodaté comme délivré.
12. **Envoi fiable** : `sent` n’est incrémenté que si `notify.sent === true`. Sans SMTP, le batch renvoie `sent=0`, `failed=N`, ne renseigne pas `lastSentAt` et n’active pas une campagne brouillon.
13. **Clôture de campagne** : boutons « Clôturer la campagne » puis « Archiver » dans l’écran campagne. La clôture est horodatée (`closedAt`), coupe les liens et les réponses, et la liste nominative des fiches bloquées avant le conseil est consultable.
14. **Dossier d’appel en lecture seule** : une fois `closedAt` renseigné, le formulaire est désactivé (`fieldset disabled`) et le serveur refuse toute nouvelle sauvegarde.
15. **Visu élève** : page `/eleve/fiches-dialogue`, rattachement par e-mail / `sourceKey` / INE, message clair si aucun dossier n’est lié. Aucun contact, jeton ou action n’est exposé. Le préfixe `/eleve` est aussi autorisé pour un compte **multi-rôles** (élève + personnel), pas seulement pour un profil élève exclusif.

---

## 2. Méthode et limites

- Tests exécutés sur `postgresql://scola:***@127.0.0.1:5432/scolia_migrate` via `TEST_DATABASE_URL`, avec `SCOLIA_TEST_MODE=1`.
- Migrations `0065_fiches_dialogue_pdf_fallback.sql` et `0066_fiches_dialogue_appel.sql` appliquées à cette base de test ; colonnes `fd_pdf_archive.pdf_base64` et `fd_fiche.appel` vérifiées.
- Aucun e-mail réel, aucun upload S3 réel. Le repli PDF base64 a été exercé à chaque publication et à la clôture d’appel.
- Les parcours sont testés au niveau **workflow** (fonctions métier réelles), pas en navigateur. Les routes et l’UI ont été relues statiquement pour les points d’autorisation et d’affichage.
- Les contenus exacts des e-mails ne sont pas vérifiés : l’environnement n’a pas de SMTP. La matrice e-mail ci-dessous décrit les points d’appel du code ; elle ne prouve pas une délivrabilité réelle.
- Les notifications de conflit et d’accord tacite sont **best-effort** : sans SMTP, l’événement métier reste enregistré et `conflictNotifiedAt` / `taciteNotifiedAt` reste vide.

---

## 3. Carte des acteurs

| Acteur | Ce qu’il fait aujourd’hui | Ce qu’il reçoit | Ce qui lui manque encore |
|---|---|---|---|
| **Élève** | Consulte ses fiches en lecture seule | Aucun mail, aucune signature | Un éventuel avis personnel, à arbitrer au cas par cas |
| **Parent 1** | Ouvre son lien personnel, saisit les vœux, signe, accepte ou refuse, peut corriger jusqu’à la clôture | Lien personnel + code, PDF d’avis, demande d’acceptation, PDF final, procédure d’appel, décision d’appel | Un accusé de réception après dépôt |
| **Parent 2** | Confirme ou contredit la réponse du parent 1 avec son propre lien | Lien personnel, puis mail de confirmation avec résumé et date limite ; information en cas de conflit ou d’accord tacite | Une notification de la décision finale |
| **Professeur principal** | Consulte le dossier famille, rédige l’avis, peut le laisser en brouillon | Rien automatiquement, hors alerte conflit | Une alerte quand la famille répond |
| **Direction** | Voit le dossier famille, gele, publie, résout les conflits, fait des avenants, complète et clôt le dossier d’appel, clôture/archive la campagne | Alerte conflit, liste des fiches bloquées | Un récapitulatif de clôture de campagne |
| **Secrétariat / admin** | Crée la campagne, génère, envoie, relance, consulte les archives, suit l’appel | Les réponses de l’API, avec `sent` / `failed` fiables | Recherche, filtres, renvoi ciblé |

### Ce qu’ils diraient

- **Élève :** « Je peux consulter ma fiche sans pouvoir la modifier : c’est le bon niveau. »
- **Parent 1 :** « J’ai validé, je peux encore corriger jusqu’à la clôture. »
- **Parent 2 :** « J’ai reçu mon propre lien : je confirme bien la réponse de l’autre parent. »
- **Professeur principal :** « Je vois enfin la demande de la famille avant de donner mon avis. »
- **Direction :** « Je vois les fiches bloquées, je clôture la campagne et je garde la main sur la publication. »
- **Secrétariat :** « L’écran distingue un envoi réel d’un échec SMTP. »

---

## 4. Scénarios joués

| # | Scénario | Ce qui est vérifié | Résultat |
|---|---|---|---|
| S1 | Collège nominal, 1 parent | Envoi → saisie → gel → avis PP → publication → choix → décision finale → acceptation | **OK** (11 assertions) — sans SMTP, l’échec est compté et la fiche reste à envoyer |
| S2 | Changement d’avis avant gel | Dépôt, lien initial, correction, nouveau code | **OK** — le lien initial reste ouvert jusqu’à la clôture ; la correction de secours par code fonctionne |
| S3 | Deux parents, accord | Dépôt P1, mail P2, confirmation ; dépôt P2 en premier, confirmation P1 | **OK** (5 assertions) |
| S4 | Deux parents, conflit | Contradiction, alertes, gel bloqué, résolution `maintien_famille` / `avis_conseil`, regelage, dossier staff | **OK** (5 assertions) |
| S5 | Accord tacite | Échéance dépassée, application automatique, information des parents, clic tardif du P2 | **OK** — l’accord tacite s’applique ; le lien P2 affiche un message dédié |
| S6 | Redoublement demandé, refusé, appel | Vœu `maintien`, avis défavorable, refus, dossier d’appel complété, documents tardifs, décision, clôture, PDF, lecture seule | **OK** (8 assertions) |
| S7 | Avenant après acceptation | Rectification, nouvelle acceptation, archives | **OK** (3 assertions) |
| S8 | Non-répondant | Gel avec 1 prête + 1 sans réponse, dégel, réponse tardive, regelage | **OK** (2 assertions) |
| S9 | Campagne clôturée | `closedAt`, ancien lien, soumission, envoi | **OK** — tout est refusé proprement et la clôture est horodatée |
| S10 | Envois | SMTP absent, fiche sans e-mail parent | **OK** — `sent=0`, `failed=1`, statut et `lastSentAt` inchangés |
| S11 | Terminale | Mode `conseil_dabord` normalisé, acceptation finale | **OK** (2 assertions) |
| S12 | 1re ST2S | Catalogue ST2S, vœu `tle_st2s` | **OK** (2 assertions) |
| S13 | Signatures | Publication direction seule, PP qui tente de publier | **OK** (2 assertions) — la garde serveur réserve la publication à la direction |
| S14 | Élève | Aucun rôle décideur, visu lecture seule, isolation entre élèves | **OK** (3 assertions) |
| S15 | Admin | Dossier famille lisible sans secrets, jeton par parent | **OK** (2 assertions) |

### Détail des parcours sensibles

#### S1 — Collège nominal

Le parcours complet fonctionne : 5 archives PDF sont produites (saisie, conseil, choix, décision finale, document final), la fiche est `acceptee` avec `closedAt`, et le repli base64 est utilisé quand S3 est indisponible.

Sans SMTP, le batch renvoie désormais `sent=0` / `failed=1`, la fiche reste `a_envoyer` et `lastSentAt` reste `null`. Le jeton personnel est tout de même créé pour permettre un renvoi après rétablissement du SMTP.

#### S2 — Changement d’avis

Le premier dépôt passe la fiche en `saisie_recue`, mais **le jeton de saisie du déposant reste valable** jusqu’à la clôture de l’étape. Le parent qui reclique sur son e-mail initial peut donc rectifier ses vœux ; le bouton « Envoyer aux familles (manquants) » reste inutile pour une fiche `saisie_recue`, ce qui est cohérent. La correction de secours par identification + code fonctionne toujours, et le jeton d’édition renvoyé après dépôt répare « Modifier ma réponse » dans la même session.

#### S3 — Deux parents

Les deux responsables reçoivent chacun leur lien et leur code. Si le parent 1 dépose, la confirmation part vers le parent 2 ; si le parent 2 dépose en premier, la confirmation part vers le parent 1. Le dépôt est attribué à l’adresse du jeton utilisé, et non à la première adresse de la fiche.

#### S4 — Conflit parental

Les deux fiches passent en `en_conflit`. Une alerte est tentée vers le PP/direction et les deux parents ; sans SMTP, l’échec est silencieux côté acteur mais l’horodatage `conflictNotifiedAt` n’est pas renseigné à tort. Le dossier staff expose la réponse déposée et le motif du désaccord. Le gel les bloque avec le motif « Conflit parents à résoudre avant l’avancement ». La direction peut résoudre en `maintien_famille` ou `avis_conseil` ; les deux fiches repassent en `saisie_recue` et avancent au regelage.

#### S5 — Accord tacite

À l’échéance, `applyFdTacitParentAccordIfDue` applique l’accord et tente d’informer les deux parents. Le jeton du 2e parent reste lisible 30 jours après l’échéance ; un clic tardif affiche « Accord parental enregistré » et reste idempotent. Sans SMTP, l’accord est acquis et `taciteNotifiedAt` reste vide.

#### S6 — Redoublement refusé et appel

La famille demande `maintien`, le conseil publie `defavorable` avec `destinationProposee: "5e"`, la famille refuse et déclenche l’appel. La fiche passe `en_appel`, l’étape courante devient `appel`, le document final est archivé, et l’e-mail d’appel + l’alerte staff sont prévus.

La direction ouvre ensuite le **dossier d’appel** depuis l’écran campagne. Elle peut l’enregistrer sans le clôturer, puis ajouter plus tard des informations et des documents (attendus ou reçus). La clôture exige une décision (`maintien` ou `reformation`) et un motif ; elle archive un PDF `appel_decision`, passe la fiche en `cloturee`, révoque les jetons et envoie le PDF aux parents. Si le SMTP est indisponible, la fiche est quand même clôturée et l’échec est signalé. Après clôture, le formulaire est désactivé et le serveur refuse toute sauvegarde.

#### S9 — Campagne clôturée

`updateFdCampagne({ statut: "cloturee" })` renseigne `closedAt`. Les anciens liens ne permettent plus de répondre, les envois sont refusés et les relances ne partent plus. L’UI propose désormais « Clôturer la campagne », puis « Archiver ».

#### S10 — Envois

Sans SMTP, l’envoi unitaire renvoie `sent=false` et conserve le statut ainsi que `lastSentAt`. Le batch compte un échec par fiche et n’active pas une campagne brouillon. Le lien personnel reste disponible pour un renvoi ultérieur.

#### S11/S12 — Terminale et ST2S

La Terminale force bien `famille_dabord` et contient une étape `acceptation_famille` ; la 1re ST2S propose `tle_st2s` et enregistre le vœu. Les deux parcours vont jusqu’à `acceptee`.

#### S14 — Élève

Aucun rôle élève n’existe dans les réponses ou les jetons : l’élève ne décide pas et ne signe pas. Le seul écran est `/eleve/fiches-dialogue`, en lecture seule. Le rattachement se fait par e-mail (insensible à la casse), `sourceKey` ou INE ; une adresse sans correspondance ne voit aucune fiche et reçoit un message invitant à contacter la vie scolaire.

#### S15 — Dossier famille staff

`GET /api/fiches-dialogue/fiches/[id]` renvoie le vœu, le commentaire, les sections lisibles, les signatures, les documents et l’historique des liens. Les jetons et codes ne sortent jamais. L’écran campagne ouvre ce dossier depuis chaque ligne et préremplit le conseil avec le dernier vœu (ou la dernière proposition en avenant).

---

## 5. Points restants (non bloquants)

### P1 — Finitions de parcours

- **Signature PP** : le workflow exige la signature direction, pas celle du PP, alors que le libellé annonce « signatures PP puis direction ». À trancher : signature obligatoire ou dispense tracée.
- **Acceptation finale** : un seul lien est encore envoyé aux deux parents. Si deux signatures sont exigées un jour, appliquer le modèle « jeton personnel par parent ».
- **Accusé de réception** : le parent ne reçoit rien après son dépôt. Un récapitulatif immédiat rassurerait.
- **Relance** : chaque relance révoque le jeton précédent. Réutiliser le jeton valide ou prévenir du remplacement.
- **Clôture de campagne** : pas de récapitulatif e-mail ni d’export des fiches non traitées.

### P2 — Lisibilité et administration

| Irritant | Détail | Recommandation |
|---|---|---|
| Double mail de décision | Le PDF de décision et la demande d’acceptation partent en deux mails | Joindre le PDF à la demande d’acceptation, ou fusionner |
| Statuts techniques | `decision_envoyee`, `en_attente_accord_parent2`… | Ajouter une phrase « prochaine action » sous le statut |
| Erreurs de génération masquées | Le wizard ne fait que `console.warn` | Afficher `created / skipped / errors` avant de quitter |
| Libellé « Relancer les non-réponses » | Le batch inclut aussi les accords parent2 | Renommer « Relancer les attentes » |
| Recherche / filtres | Le tableau campagne reste long | Filtre statut + recherche nom/classe |

---

## 6. Matrice des e-mails

| Déclencheur | Destinataire | Contenu | Moment | Constat |
|---|---|---|---|---|
| Envoi initial | Parent 1 + Parent 2 | Lien personnel, code, deadline | À l’envoi | **OK** : un lien et un code par parent ; échec SMTP tracé |
| Relance | Parent 1 + Parent 2 | Nouveau lien | Manuel | Révoque le lien précédent |
| Accord parent 2 | Parent 2 | Résumé + lien + deadline | Après dépôt P1 | Un seul autre parent est notifié |
| Accord tacite | Parent 1 + Parent 2 | Information d’accord tacite | À l’échéance | **Ajouté** (best-effort) |
| Conflit | Staff + Parent 1 + Parent 2 | Alerte staff / information des responsables | Après contradiction | **Ajouté** (best-effort) |
| Dépôt famille | Personne (staff) | — | Après dépôt | Le staff consulte le dossier famille |
| Brouillon PP | Personne (direction) | — | Après brouillon | La direction consulte le tableau |
| Publication conseil | Parents | PDF + étape suivante | Automatique | Pas de copie staff |
| Décision finale | Parents | PDF + demande d’acceptation | Automatique | Deux mails distincts ; un seul lien d’acceptation pour les deux parents |
| Acceptation | Parents | PDF final | Automatique | Pas de notification staff |
| Refus | Parents + staff | PDF + procédure d’appel / alerte | Automatique | Le staff est alerté |
| Appel | Parents | Procédure | Au refus | — |
| Clôture d’appel | Parents | PDF de décision d’appel | À la clôture | **Ajouté** ; pas de copie staff |
| Avenant | Parents | PDF + motif | Automatique | Pas de copie staff |
| Clôture campagne | Personne | — | À la clôture | Aucun récapitulatif |

---

## 7. Plan restant

### Lot 1 — Décision et confiance

1. **Signature PP** : l’exiger pour publier, ou tracer une dispense explicite.
2. **Accusé de réception** après dépôt famille.
3. **Récapitulatif de clôture de campagne** (fiches non traitées, export).

### Lot 2 — Famille et suivi

4. **Acceptation finale par parent** si le double accord devient requis.
5. **Relances** : réutiliser le jeton valide ou expliquer le remplacement.
6. **Suivi admin** : recherche, filtre statut, détail des `skipped`.

### Lot 3 — Lisibilité

7. **Mails** : reprendre les résumés lisibles déjà utilisés dans les écrans parents.
8. **Wizard** : afficher les erreurs de génération.
9. **Statuts** : ajouter une phrase « prochaine action ».

---

## 8. Vérifications

- Audit personas : **55 assertions OK / 0 KO / 0 manque confirmé**.
- Régressions workflow : **11/11 OK**, dont :
  - faux positif SMTP (`sent=0`, `failed=1`, `lastSentAt=null`, campagne non activée) ;
  - dossier famille staff et vue élève en lecture seule, avec isolation entre élèves ;
  - appel (sauvegarde partielle, documents tardifs, décision et motif obligatoires, clôture en `maintien` et en `reformation`, PDF, révocation des jetons).
- Typecheck : aucune erreur dans les fichiers fiches de dialogue ; seules 4 erreurs préexistantes `TS5097` dans des tests sans rapport.
- Migrations `0065_fiches_dialogue_pdf_fallback.sql` et `0066_fiches_dialogue_appel.sql` appliquées à `scolia_migrate` ; entrées ajoutées au journal Drizzle.
- Aucun e-mail réel, aucun S3 réel, aucune base de production touchée.
- **Validation navigateur (instance locale)** : ouverture d’une campagne, consultation du dossier famille, clôture de campagne, page élève en lecture seule. La page élève a révélé puis corrigé un blocage proxy pour les comptes multi-rôles (`admin` + `eleve`). Les données de démonstration et le rôle temporaire ont été supprimés après le test.
- **Reste à vérifier manuellement** : rendu responsive et parcours d’appel en lecture seule dans l’UI (les gardes serveur sont couvertes par les tests).

### Commandes

```powershell
$env:TEST_DATABASE_URL="postgresql://scola:scola_dev_pwd@127.0.0.1:5432/scolia_migrate"
$env:SCOLIA_TEST_MODE="1"
npx tsx --require ./scripts/stub-server-only.cjs scripts/audit-fiches-dialogue-personas-v2.ts
```

Le script npm `test:fiches-dialogue:personas` utilise la syntaxe POSIX `VAR=… commande` ; sous PowerShell, préférer les deux lignes `$env:` ci-dessus.

Équivalent POSIX :

```bash
TEST_DATABASE_URL=postgresql://scola:scola_dev_pwd@127.0.0.1:5432/scolia_migrate \
SCOLIA_TEST_MODE=1 \
npx tsx --require ./scripts/stub-server-only.cjs scripts/audit-fiches-dialogue-personas-v2.ts
```

```powershell
$env:TEST_DATABASE_URL="postgresql://scola:scola_dev_pwd@127.0.0.1:5432/scolia_migrate"
$env:SCOLIA_TEST_MODE="1"
npx tsx --test --require ./scripts/stub-server-only.cjs app/lib/fiches-dialogue-workflow.integration.test.ts
```

---

## 9. Conclusion

Le socle est solide : transitions, gel/dégel, conflits, avenants, Terminale, ST2S, repli PDF, correction jusqu’à la clôture, **appel avec décision extérieure**, **dossier famille avant décision**, **visu élève en lecture seule** et **envoi fiable** fonctionnent.

Les blocages de l’audit précédent sont levés. Les points restants sont des finitions de parcours et de lisibilité, sans remise en cause de la machine à états. La prochaine étape est une validation navigateur des nouveaux écrans, puis un commit ciblé sur le module, sans toucher à `main`.
