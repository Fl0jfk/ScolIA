# Socle 23 — Bulletins, conseils de classe et passage

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian) ; recommandations appliquées par défaut (D-031), listées au §26. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production. Le module Next.js « fiches de dialogue » (M10, en usage réel) a été **lu seulement**, de façon ciblée (états et gabarits), sans rien exécuter ni modifier (CLAUDE.md).
**Branche :** `ent-socle-22-23-conception`, partie de `origin/ent-mc0hoe` (`7467f2d`).
**Passe personas :** `docs/socles/PERSONAS_23.md` ; ajouts faits **ici**, marqués **« ajout personas »** (§28) ; en cas de contradiction, le §28 l'emporte ; rien n'est supprimé.
**Cohérence stricte avec le socle 21** (`21_EVALUATIONS.md` §4, §6.2, §6.3, §9, §10, §12, §14, §16 « cohabitation », §17.15 EVA-233 à 280, §18 R-V1 à R-V47, §29) **et D-032** : au lycée pilote, **notes et bulletins restent dans Charlemagne l'an 1** ; ScolIA y distribue les bulletins de Charlemagne (lot V14 du 21) ; le LSL officiel attend la démarche éditeur. Les actions du 21 qui relèvent du 23 (EVA-235, 244 à 258, 269, 275, 276) sont **reprises sous leur identifiant**, sans doublon.
**À lire avec :** `CLAUDE.md` ; `DECISIONS.md` D-017, D-021 à D-036, D-037 à D-046 (E1–E14) ; `SOCLES.md` §23 ; `MATRICE_CHARLEMAGNE.md` NOT-11 à NOT-18, T-11 ; `TRANSITION_T0_INVENTAIRE.md` (M10, C06) ; `rapport-fiches-dialogue-personas-v2.md` ; `22_COMPETENCES.md` ; `04_ADMINISTRATIF.md` §8.6 (droits du lien) ; `05_STRUCTURE.md` (montée, STR-70) ; `07_DOCUMENTS.md` ; `08_NOTIFICATIONS.md` ; `09_INTEROP.md` (R-X24, FX-34) ; `IA_COEUR.md` §5.
**Conventions** : [F], [V], [V*], [NV], [H], [R] comme au 22.
**Numérotation (préfixes vérifiés libres le 2026-11-11) :** actions **BUL-100 à BUL-426** ; règles **R-B1 à R-B22** ; réglementaire **R-B01 à R-B17** ; cas limites **CB-01 à CB-46** ; défauts Rust **BD1 à BD14** ; existant Next.js **NXF-01 à NXF-16**, défauts Next.js **NXFD-1 à NXFD-12** ; questions **Q-BU1 à Q-BU13** ; réglages **S23-01 à S23-08** ; points pour Florian **F23-1 à F23-7** ; lots **B0 à B15** ; migrations **2460–2479** (plage imposée).

---

## 0. En bref

1. **Le bulletin versionné et immuable de l'existant est bon ; le reste fuit ou casse** : un conseil **ne se verrouille jamais** (`tenu` inatteignable), les décisions sont **écrasées sans trace**, le bulletin **imprime des décisions provisoires** d'un conseil seulement planifié, la famille lit les appréciations **en direct** au lieu du bulletin publié, supprimer un vœu **efface en cascade** l'avis du conseil et la réponse de la famille, la montée **ignore** les décisions, les applis familles **ne voient plus** aucune décision. **Lot B0** de correctifs (14 défauts BD1–BD14, trois urgents) avant tout (§2).
2. **Le réglementaire tranche [V, §5]** : le conseil (R. 421-50/51) **propose**, le chef d'établissement **décide** orientation et redoublement (D. 331-31 à 38, D. 331-62) ; désaccord → **entretien**, décision **motivée**, **3 jours ouvrables** pour faire appel, commission, **maintien de droit** un an ; l'**avis de l'élève mineur est recueilli** ; spécialités **choisies par la famille** ; privé sous contrat : procédure miroir avec **commission propre** ; **un délai de recours non mentionné n'est pas opposable** (CJA R. 421-5) ; délégués des parents tenus à la **confidentialité** (D. 111-11) ; l'avertissement est une **sanction** (R. 511-13) étrangère au conseil ; LSL : avis « très favorable / favorable / doit faire ses preuves ».
3. **Le conseil en un clic** : série planifiée, participants calculés (professeurs de groupe compris, chacun ne voyant que ses élèves, D-022), dossier préparé depuis l'instantané du 21 et les synthèses du 22, « ce qui a changé », propositions du PP en brouillon, **séance sur tablette** avec vue projetée **minimisée** et geste « délégués sortis », **verrou** à la tenue, **PV** et synthèse anonyme pour les délégués des parents (§7, §8).
4. **La fiche de dialogue de Next.js est reprise, pas réinventée** (D-035) : seize actions existantes (NXF-01 à 16) gardées, réécrites en Rust, **branchées sur les vrais liens** (droits, parent sans exercice, interdiction), sans jetons ni JSON ni base64 ; douze défauts ou manques corrigés (rattachement par e-mail, avis de l'élève, délais non calculés, accusé…) ; niveau **production** pour la campagne 2027-2028 (§2.3, §9).
5. **Un circuit tracé** : demandes par titulaire, confirmation, accord tacite seulement si l'information est tracée, conflit résolu par la direction, avis du conseil, entretien, décision motivée, notification **avec voies et délais** (bloquant sinon), acceptation, avenant, appel et commission, maintien, redoublement avec **dialogue** et **dispositif d'accompagnement**, puis **décision définitive** lue par la montée (§9).
6. **Bulletins** : 4 trames prêtes (D-023), contrôle avant publication, publication programmée, **un exemplaire nominatif par destinataire calculé à la publication**, coffre, courrier par adresse, remise en main propre tracée, non ouverts relancés, version 2 à tous, registre, duplicata (§10).
7. **An 1 du pilote** : bulletins de Charlemagne distribués par ScolIA (V14 du 21), conseils **en doublon** facultatifs sur moyennes importées, fiche de dialogue en **usage réel** dans ScolIA, décisions reportées dans Charlemagne (§12).
8. **L'IA** relit, résume, rédige un brouillon de PV, explique une décision écrite ; **jamais** une mention, un avis ou une décision (§14).
9. **152 actions** (BUL-100 à BUL-426), **46 cas limites**, **22 règles**, **16 lots** (B0 à B15), **17 migrations** planifiées dans 2460–2479 (3 en réserve) (§17, §23).
10. **Pour Florian** : démarche éditeur ; export des archives Next.js ; report actuel des décisions dans Charlemagne ; recommandés ; date de bascule de la fiche de dialogue ; juriste (acceptation réputée, jours ouvrables) ; DPO (§26.2).
11. **[ajout personas]** §28 : 19 personas, 88 manques ; trois bloquants tranchés (D-031) — **notification et délai d'appel par titulaire** (rien ne court sur un simple dépôt au coffre), **tout titulaire peut faire appel** (définitif à la fin de tous les délais), **actes engageants authentifiés** (passkey, le lien nominatif ne sert qu'à lire) ; actions BUL-430 à 466, règles R-B23 à R-B35, migrations dans la réserve 2474–2476.

---
---

## 1. Thèse : le conseil examine, la direction décide, le bulletin atteste

Le conseil de classe est le seul moment de l'année où **toute l'équipe** regarde **chaque élève**. Il ne décide presque rien seul : il **examine** (résultats, assiduité, travail), il **propose** (orientation, passage), il **émet des avis** (mentions, avis pour l'examen) ; la **décision** revient au chef d'établissement (orientation, redoublement [V R-B07, R-B09]) ; la **famille** demande, répond, fait appel. Le bulletin, lui, est un **acte** : il atteste ce qui a été examiné, à une date, pour des destinataires précis. Une erreur de bulletin envoie un document faux à un parent privé de droits ; une erreur de procédure d'orientation rend une décision **contestable devant le juge**.

Douze principes [R] :

1. **Une seule entrée** : le conseil et le bulletin lisent l'**instantané d'arrêt** du 21 (moyennes, statuts, appréciations versionnées) et les **synthèses** du 22 ; rien n'est recalculé (21 §6.2).
2. **Préparer en un clic, tenir en séance, verrouiller à la fin** : dossier prêt automatiquement ; séance pilotée sur tablette ; verrou à la tenue ; réouverture motivée ★.
3. **Le conseil propose, la direction décide** : avis et propositions du conseil ; décisions d'orientation et de redoublement du chef d'établissement, **motivées** en cas de désaccord, notifiées avec leurs **voies et délais de recours** [V R-B07, R-B13].
4. **Tout est historisé** : chaque avis, mention, proposition, décision est une version (auteur, moment, motif) ; corrige l'écrasement actuel (BD1).
5. **Mentions = données de l'établissement, jamais une sanction** : félicitations, compliments, encouragements, mises en garde ; **jamais « avertissement »** (R. 511-13 [V R-B10]).
6. **La séance est minimisée** : la vue projetée devant les délégués ne montre ni santé, ni aménagements, ni discipline détaillée, ni situation familiale (21 §14.4, §14.6 point 7) ; les délégués des parents sont tenus à la **confidentialité** (D. 111-11 [V R-B03]).
7. **La procédure d'orientation est un circuit tracé** : demande de la famille (vœux provisoires puis définitifs), proposition du conseil, dialogue, décision, réponse, appel, commission, maintien ; les délais sont **calculés** en jours ouvrables de l'établissement (socle 5).
8. **La fiche de dialogue qui marche aujourd'hui (Next.js, M10) est reprise, pas réinventée** (D-035) : ses actions métier sont gardées, réécrites en Rust, branchées sur le cœur (élève, liens, droits, documents, notifications) ; ses défauts restants sont corrigés.
9. **Le bulletin suit 4 trames prêtes** (D-023) : collège (bilan périodique), lycée général et technologique, lycée professionnel, sans note ; l'établissement choisit et habille, il ne met pas en page.
10. **Chaque destinataire reçoit son exemplaire**, calculé **à la publication** (R-V47 du 21) selon le droit `recevoir_bulletins` de **son** lien ; jamais l'adresse d'un autre destinataire ; aucune rétention pour impayé (R-V33).
11. **Rien de ce qui est publié ne change** : correction = version 2, envoyée à tous les destinataires de la v1, ancienne version au registre.
12. **L'IA prépare, ne décide pas** : aucune mention, avis ou décision proposés par un modèle (R-V40 du 21) ; elle relit, résume, signale.

**Ce qui fera la différence [H]** : le **conseil préparé en un clic** (A7) avec « ce qui a changé depuis le dernier conseil » ; la **séance sur tablette** qui suit l'élève projeté, propose la mention selon les règles de l'établissement et note la décision en un geste ; le **procès-verbal** produit à la fin ; la **fiche de dialogue** de Next.js, déjà éprouvée par l'établissement, branchée enfin sur les vrais liens familiaux (parents séparés, accord du second parent, conflit) ; la **procédure d'appel** suivie jusqu'à la décision de la commission ; la **montée de niveau** qui lit enfin les décisions (aujourd'hui elle les ignore, BD8).

---

## 2. Évaluation franche de l'existant

### 2.1 Rust : ce qu'il y a [F]
Audit en lecture seule par un sous-agent sur `ent-mc0hoe` (`7467f2d`), 2026-11-11 ; chemins depuis `ent-rust/`.

| Brique | Où | Ce qu'elle fait | Verdict |
|---|---|---|---|
| Conseil de classe | migrations `071`, `111`, `181` (`ent_conseil_classe` : `classe` en texte + `classe_id`, `periode_id`, `president_user_id`, `statut` planifie/tenu/annule, `periode_libelle` libre) | un conseil par classe et période | **corriger** : états, verrou, participants |
| Dossier par élève | `ent_conseil_classe_dossier` (071, 181, 460) : appréciation, `mention` (6 valeurs, CHECK), `decision_code` (aucune / passage / redoublement / orientation / reserve), `avis_orientation` **texte libre**, `decision_texte`, `visible_famille`, `decided_at/by` ; UNIQUE (conseil, élève) | une ligne par élève | **remplacer** : avis, mentions multiples, propositions et décisions distinctes, versions |
| Conseil (voie EVA) | `crates/ent-db/src/evaluations_conseil.rs:315,406,620` ; qui décide : `eva_peut_decider` (`:88-100`) = `bulletins.publier` (admin, direction) **ou** PP de la classe | création et décisions | **corriger** (verrou, historique, droits) |
| Conseil (voie historique) | `crates/ent-db/src/service.rs:13709-13960` ; routes `crates/ent-api/src/routes.rs:504-516` ; `decider` fermée en 410 (V0, `routes.rs:6008`) | `…/tenir` est la **seule** voie qui pose `tenu` | **remplacer** |
| Mentions | `evaluations_conseil.rs:30-37` : félicitations, compliments, encouragements, mise_en_garde_travail, mise_en_garde_conduite, aucune (V0 a retiré « avertissement », D11) | liste en dur | **remplacer** par une liste de l'établissement |
| Avis d'orientation | `ent_orientation_avis` (181) : FK vers `ent_voeu_inscription` **`ON DELETE CASCADE`** ; avis favorable/réservé/défavorable ; `reponse_famille` accord/désaccord ; `evaluations_conseil.rs:718,811,860` (refusé si bulletin publié ; réponse unique après publication) | avis par vœu | **remplacer** par le circuit D. 331 (§9) |
| Bulletin versionné | migration `182` (`ent_bulletin` : `contenu JSONB` figé, `empreinte`, `storage_key`, `version`, statut genere/publie/remplace, **déclencheur d'immuabilité** `182:34-60`, index « une seule version publiée ») ; `crates/ent-db/src/evaluations_bulletin.rs:380` (génération), `:502` (publication), `:730`, `:744` | période fermée requise ; motif obligatoire pour une v2 ; v1 passée « restreinte » ; PDF et document au tiroir famille ; outbox | **garder** le modèle ; **remplacer** le PDF (`crates/domain-eleve/src/bulletin_pdf.rs` : texte ASCII, accents perdus, 21 §2.1) par le moteur du socle 7 |
| Bulletin historique | `routes.rs:8222` | fermée en 410 (D5 du 21) | retirer à terme |
| Exports LSL | `evaluations_bulletin.rs:765` ; `crates/domain-eleve/src/evaluation.rs:420-600` (XML « non officiel ») ; CSV historique `service.rs:14964`, route `routes.rs:7320` ; trace `ent_eva_export` (`conforme_verifie` toujours faux) | droit `exports.officiels` | **refaire** au moteur du socle 9 (B11) |
| Parcoursup, Notanet | `crates/domain-eleve/src/parcoursup_export.rs`, `notanet_export.rs` (non relus) | CSV maison | socles 37 et 38 |
| Montée de niveau | `service.rs:10965,11041,11055,11196` ; `crates/domain-eleve/src/continuum.rs:460` ; droit `role_can_manage_continuum` (admin, direction, administratif) | calcule `niveau_suivant` **mécaniquement**, `passage_juridique = false`, `redoublement_possible = true` ; **ne lit jamais `decision_code`** (grep) | **remplacer** par la lecture des décisions (§9.8, socle 5 STR-70) |
| Redoublement | `ent_eleve_inscription.redoublement BOOLEAN`, `decision_passage TEXT` (284) **jamais alimentés** ; `ent_eleve.redoublant` (015) orphelin ; booléen saisi à la main (`crates/domain-eleve/src/scolarite.rs:896-928` [sous-agent]) | — | **remplacer** par la décision (§9) |
| Droits | `crates/ent-auth/src/actions.rs` : `conseils.gerer` (l. 125, 512) **donné à tout professeur** (l. 1075), à la direction, au CPE, à l'administratif (l. 1225) ; `bulletins.publier` (l. 200, 547) ; `exports.officiels` (l. 191, 543) ; **aucune action `orientation.*`** | — | **refaire** le catalogue (§15) |
| Écrans Leptos | `web/crates/ent-web-app/src/modules/evaluations/` : `conseil.rs`, `bulletins.rs`, `famille.rs` | conseil, bulletins, orientation ; **pas de bouton « tenir le conseil »** | **garder**, compléter |
| UI JavaScript | `crates/ent-api/src/ui.rs:43113-43380` (conseil) | appelle `/conseils-classe/dossiers/{id}/decider`, **fermée en 410** : bouton cassé | figée (F-001) ; message clair (B0) |
| Applis mobiles | `ent-rust/apps/` : `GET /eleves/{id}/appreciations` (quatre applis), `GET /eleves/{id}/conseils` (familles iOS et Android) | lisent `visible_famille` de la voie historique ; la voie EVA écrit `visible_famille = false` : **les nouvelles décisions n'apparaissent jamais** | **corriger** (B0 : lecture depuis le bulletin publié) |
| Tests | `crates/ent-db/src/tests_evaluations.rs` (13), `tests_evaluations_v0.rs` (22 : conseils soumis au droit du lien l. 177, mentions l. 371…) ; `web/crates/ent-web-server/tests/evaluations.rs` | grille → bulletin → export ; isolation | **compléter** : verrou, historique, montée, recours |
| RLS | `460_eva_v0_correctifs.sql` : **permissive** sur `ent_conseil_classe`, `ent_conseil_classe_dossier`, `ent_orientation_avis`, `ent_bulletin`, `ent_eva_export` ; aucune FK composée ; `eleve_id` sans FK sur `ent_note_appreciation` | — | stricte au lot B15 |

### 2.2 Rust : défauts trouvés (à corriger avant toute extension)
D1–D5, D8–D12 et D14 du socle 21 sont corrigés par V0 (460, routes en 410) ; **D6, D7, D13 ne le sont pas**. Défauts propres au 23 :

| # | Défaut | Conséquence | Où [F] | Gravité | Correction |
|---|---|---|---|---|---|
| BD1 | **Décision du conseil écrasée sans trace** (`ON CONFLICT … DO UPDATE`), `decided_by` remplacé ; **aucun verrou** : un conseil `tenu` ou un bulletin publié n'empêche pas la modification | une mention ou une décision change après coup sans que personne ne le sache | `evaluations_conseil.rs:628, 657-662` | **urgent** | B0 : journal en ajout seul ; refus après `tenu` sauf réouverture motivée |
| BD2 | **Le bulletin embarque des décisions provisoires** : la génération prend le dernier conseil non annulé, **même `planifie`** | une proposition de mention ou de passage imprimée comme une décision | `evaluations_bulletin.rs:287-296` | **urgent** | B0 : génération refusée si le conseil n'est pas `tenu` (ou sans rubrique « conseil » si l'établissement publie avant, CB-31) |
| BD3 | **La famille lit appréciations et avis en direct**, pas le contenu figé du bulletin publié : après une réouverture, une appréciation modifiée devient visible alors que le bulletin n'a pas changé ; le filtre `periodes_publiees_famille` accepte aussi tout document `BULLETIN_<code>` « standard » | deux vérités côté famille | `service.rs:6482-6500, 6225-6246` | important | B0 : la famille lit la **version publiée** (contenu figé) |
| BD4 | **`tenu` inatteignable depuis la voie EVA** ; aucun bouton Leptos | aucun conseil n'est jamais verrouillé | §2.1 | important | B0 : commande « tenir » + bouton |
| BD5 | **`ON DELETE CASCADE` du vœu vers l'avis du conseil** | supprimer un vœu efface l'avis et la réponse de la famille : perte de preuve d'une procédure contestable | migration `181` | **urgent** | B0 : `ON DELETE RESTRICT` ; un vœu se retire (état), ne se supprime pas |
| BD6 | **`conseils.gerer` donné à tout professeur** | droit trop large (atténué par la portée de classe et D-022) | `actions.rs:1075` | important | B0 : décider réservé à la présidence ; professeur = lecture et avis |
| BD7 | **Décision sans paramètre** : « redoublement » sans motif ni dialogue ; « orientation » sans destination ; `avis_orientation` texte libre | décision inexploitable par la montée et l'orientation, non motivée | `evaluations_conseil.rs:30-38` | important | B8 (circuit D. 331) |
| BD8 | **La montée ignore les décisions** (`decision_code`, `redoublement`) | un élève dont le redoublement est décidé monte quand même | `service.rs:10965, 11055` | important | B10 (port du 23 lu par le socle 5) |
| BD9 | **Mobiles aveugles et UI JS cassée** : les nouvelles décisions n'apparaissent jamais dans les applis ; `/decider` renvoie 410 | familles sans information ; personnel bloqué | §2.1 | important | B0 (mobiles), F-001 (UI JS figée avec message) |
| BD10 | **Aucune procédure d'appel**, aucun recours, aucune commission, aucun délai | non conforme à D. 331-34 et suivants [V R-B07] | — | important | B9 |
| BD11 | **Mentions en dur**, une seule par élève | l'établissement ne peut pas nommer les siennes | `evaluations_conseil.rs:30-37` | mineur | B4 |
| BD12 | **Colonnes orphelines** (`decision_passage`, `redoublement`, `redoublant`) | trois vérités possibles | 015, 284 | mineur | B10 (alimentées par la décision, puis retirées) |
| BD13 | **RLS permissive seulement**, pas de FK composées | isolation par filtres applicatifs | 460 | important | B15 |
| BD14 | **PDF du bulletin illisible** (ASCII, accents perdus) | « Nguy?n » sur un acte officiel | `bulletin_pdf.rs:99-116` (21 §2.1) | important | B5 (moteur du socle 7) |

### 2.3 Next.js : la fiche de dialogue (module M10), en usage réel [F, lecture seule et ciblée]
`TRANSITION_T0_INVENTAIRE.md` l. 73 et 114 : « Fiches de dialogue (M10) — 20 fichiers, ~7 000 lignes, 11 routes, **partiel à fiable** [H] (3 versions, audit personas v2) » ; tables `fd_campagne`, `fd_etape`, `fd_fiche`, `fd_reponse`, `fd_signature`, `fd_token`, `fd_pdf_archive` ; PDF en S3 et **en base64** ; « campagne en cours : à conserver ; campagnes closes : archive (PDF signés versés au dossier) ; `fd_token` : abandonné ». Rapport `docs/rapport-fiches-dialogue-personas-v2.md` (21/10/2026 : 15 scénarios, 55 assertions, aucune en échec). Lecture ciblée de `app/lib/fiches-dialogue-workflow.ts` et `app/lib/fiches-dialogue-templates.ts` (états et gabarits seulement, rien d'exécuté ni de modifié).

**Ce que fait Next.js — actions existantes, toutes reprises** (D-035 : on garde les actions, on réécrit) :
| Id | Action Next.js | Reprise |
|---|---|---|
| NXF-01 | Campagne par niveau (6ᵉ → terminale), gabarit par niveau, assistant de création, mode **« famille d'abord »** ou **« conseil d'abord »** (terminale normalisée « famille d'abord » + acceptation) | BUL-300 à 303 |
| NXF-02 | **Catalogue de destinations** par niveau (niveau suivant interne, **maintien**, « ailleurs », « autre parcours » ; 2ᵈᵉ → 1ʳᵉ générale, 1ʳᵉ ST2S, autre série ; 1ʳᵉ → terminale) avec champs conditionnels (spécialités…) ; options cochées par défaut selon le niveau | `orn_destination`, BUL-304 |
| NXF-03 | Étapes datées : saisie famille, conseil (avis), choix définitifs, décision finale du conseil, acceptation famille, appel | `orn_etape` |
| NXF-04 | Génération des fiches, envoi par **lien et code personnels par responsable**, relance, envoi fiable (échec compté, jamais horodaté comme délivré) | BUL-305 à 308 (liens remplacés par le coffre et le lien nominatif du socle 7) |
| NXF-05 | Saisie des vœux par un responsable, **correction jusqu'à la clôture** de l'étape | BUL-320, 321 |
| NXF-06 | **Confirmation ou contradiction du second responsable** ; **accord tacite** à l'échéance avec information des deux ; **conflit** → alerte, gel bloqué, résolution par la direction (`maintien_famille` / `avis_conseil`) | BUL-322 à 326 |
| NXF-07 | **Gel / dégel** de la campagne avant le conseil ; liste nominative des fiches bloquées | BUL-309, 310 |
| NXF-08 | **Dossier famille** côté personnel (vœu, commentaire, désaccord, signatures, documents, historique des liens) | BUL-311 |
| NXF-09 | Avis du conseil préparé en **brouillon par le PP**, prérempli depuis le dernier vœu ; **publication réservée à la direction** (garde serveur) | BUL-330 à 333 |
| NXF-10 | Choix définitifs, décision finale, **acceptation ou refus** de la famille | BUL-340 à 345 |
| NXF-11 | **Avenant** après acceptation (rectification, nouvelle acceptation, archive) | BUL-346 |
| NXF-12 | **Appel** : dossier complétable (commission, dates, pièces attendues et reçues), décision **maintien / réformation**, PDF, clôture, lecture seule | BUL-350 à 356 |
| NXF-13 | Archives PDF à chaque étape (saisie, conseil, choix, décision, document final, décision d'appel) | socle 7 (documents émis, immuables) |
| NXF-14 | **Vue élève en lecture seule** (rattachement par e-mail, `sourceKey`, INE) | BUL-360 (rattachement par l'identité ScolIA, jamais par e-mail) |
| NXF-15 | Clôture puis archivage de la campagne (liens coupés) | BUL-312, 313 |
| NXF-16 | Signatures (direction ; PP annoncé mais non exigé) | BUL-334 (signature du PP : décision §9.6) |

**Défauts et manques restants de Next.js (à ne pas reproduire)** :
| # | Constat | Où | Dans ScolIA |
|---|---|---|---|
| NXFD-1 | **Rattachement de l'élève par e-mail** (insensible à la casse), `sourceKey` ou INE | rapport §1 point 15 | identité ScolIA (socle 2), jamais l'e-mail |
| NXFD-2 | Responsables pris depuis des **adresses** importées une fois (pas les liens vivants, pas les droits `recevoir_bulletins`, pas l'interdiction de communication) | T0 ; D-035 point 2 | liens du socle 4 **à la date** |
| NXFD-3 | **Données de réponse en JSON** (`fd_reponse`, colonnes JSONB) et **PDF en base64** en base | T0 l. 73, 114 | tables normalisées (D-036) ; objets immuables (E5) |
| NXFD-4 | **Jetons** d'accès par lien envoyé par e-mail (`fd_token`) | T0 | coffre authentifié ; lien nominatif à usage limité du socle 7 pour qui n'a pas de compte (D-027 C, G) |
| NXFD-5 | **Signature du PP** annoncée mais non exigée | rapport §5 P1 | décidé au §9.6 |
| NXFD-6 | Pas d'**accusé de réception** après dépôt ; pas de **notification au second parent** de la décision finale ; un seul lien d'acceptation pour les deux ; pas de copie au personnel | rapport §5, §6 | §9 (accusé, chaque responsable informé, acceptation par responsable si exigée) |
| NXFD-7 | Relance qui **révoque** le lien précédent | rapport §5 | plus de jeton (coffre) |
| NXFD-8 | **Statuts techniques** affichés (`decision_envoyee`…) | rapport §5 P2 | phrase « prochaine action » |
| NXFD-9 | **Avis de l'élève mineur** non recueilli (« l'élève ne décide pas et ne signe pas ») | rapport §4 S14 | D. 331-32 : « l'avis de l'élève mineur est recueilli » [V R-B07] → recueil tracé (BUL-361) |
| NXFD-10 | Délai d'appel et voies de recours **non calculés** par le logiciel | rapport | calcul en jours ouvrables, mention obligatoire (R-B13) |
| NXFD-11 | Pas de lien avec la **décision de redoublement** de D. 331-62 (dialogue, dispositif d'accompagnement) | — | §9.5 |
| NXFD-12 | Pas de lien avec la **montée** de niveau ni avec la constitution des classes | D-035 point 2 | B10 |

### 2.4 Ce qu'il ne faut pas perdre (D-035, point 5)
Le jeton **personnel par responsable** (attribution de la réponse à la bonne personne) → devient l'**authentification** ; l'**accord tacite** et la **résolution de conflit** par la direction ; la **correction jusqu'à la clôture** ; l'**avenant** ; le **dossier d'appel complétable dans le temps** et sa lecture seule après clôture ; l'**envoi fiable** (un échec n'est jamais compté comme délivré) ; la **publication réservée à la direction** ; la **vue élève en lecture seule** ; le **préremplissage** de l'avis du conseil depuis le vœu ; les **gabarits par niveau** (y compris 1ʳᵉ ST2S, propre au lycée pilote) ; les **archives PDF** à chaque étape.

### 2.5 Charlemagne et Pronote [H, `MATRICE_CHARLEMAGNE.md` §3.7]
Préparation automatique (NOT-12, « X(F) » : fait par l'add-on de Florian), tenue en séance sur tablette, mentions, avis (NOT-13), décisions de passage, redoublement, orientation (NOT-14), bulletin figé publié (NOT-15), LSL et Parcoursup (NOT-16), **fiches de dialogue** (NOT-17, **trou** du marché comblé par Next.js, T-11), **recours** (NOT-18, carte). Pronote 2026 affiche une mini-fiche d'aménagements en mode conseil [V, lu pour le 21] : **à ne pas reproduire** (21 §14.6 point 7).

### 2.6 Verdict
Le **bulletin versionné et immuable** (182) est une bonne base ; tout le reste du domaine est à reconstruire : un conseil qui ne se verrouille jamais, des décisions écrasées, une orientation réduite à « favorable / réservé », une montée qui ignore les décisions, des applis qui ne voient plus rien. La **fiche de dialogue** de Next.js est le module de référence du domaine : elle fixe le niveau d'exigence (production pour la campagne 2027-2028, D-035 point 2) et ses actions sont reprises.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait ici | Ce qui lui fait perdre du temps aujourd'hui [H] | Ce qu'il redoute | Ce qui le ravirait | Fréquence |
|---|---|---|---|---|---|
| **Chef d'établissement** (ou adjoint, président délégué) | préside, arrête, décide (orientation, redoublement), signe, vise | préparer 30 conseils ; relire 900 bulletins ; décisions saisies deux fois (conseil, logiciel) | une décision contestée pour vice de procédure ; un bulletin erroné envoyé | séance pilotée, décision en un geste, PV automatique, contrôle avant publication, **délais d'appel calculés** | 3 fois par an × chaque classe |
| **Professeur principal** | prépare, synthétise, rédige l'appréciation générale, présente chaque élève, mène le dialogue d'orientation | courir après les appréciations ; dossiers sur papier ; tableaux Excel des vœux | arriver au conseil sans savoir ce que veut la famille | dossier prêt, vœux de la famille à côté, propositions préremplies, « ce qui a changé » | chaque période |
| **Professeur** | donne son avis sur ses élèves ; absent parfois | assister à des conseils pour 3 élèves (groupe) | parler d'un élève sans voir ses autres matières | **note au président** s'il est absent ; fenêtre du conseil (21 §14.3) | chaque période |
| **CPE** | présente la vie scolaire, l'assiduité ; appréciation de vie scolaire | ressortir les absences, les retards | qu'une sanction apparaisse au bulletin | assiduité bornée calculée (R-V31) ; pas de sanction (R-V32) | chaque période |
| **Secrétariat de scolarité** | planifie les conseils, convoque, imprime et envoie les bulletins, gère les campagnes de fiches de dialogue, archive | **mise sous pli** de 900 bulletins, double envoi aux parents séparés, réclamations ; fiches papier perdues ; relances | envoyer un bulletin au parent interdit ; perdre une fiche signée | **lot d'impression par adresse**, distribution par les droits, campagne de fiches suivie, registre annuel | toute l'année |
| **Délégués des élèves** | représentent la classe, prennent la parole | — | être exposés à des informations confidentielles sur leurs camarades | vue séance minimisée ; sortie tracée pour les cas individuels | 3 fois par an |
| **Délégués des parents** | représentent les parents, font un compte rendu | notes prises à la main | trahir la confidentialité (D. 111-11) | **synthèse anonyme** validée par le président pour leur compte rendu (EVA-257 du 21) | 3 fois par an |
| **PsyEN, infirmière, assistante sociale, médecin** | participent quand un cas individuel le justifie [V R-B02] | — | être exposés hors de leur cas | invitation au seul point de l'élève | ponctuel |
| **Famille** (dont parents séparés, parent sans exercice, tiers digne de confiance) | lit le bulletin, formule les vœux, répond, fait appel | bulletin papier perdu, double envoi ; vœux sur papier ; délai d'appel incompris | découvrir une décision trop tard pour faire appel | bulletin au coffre, avis clair, **vœux en ligne**, délais écrits, entretien proposé en un clic | 3 fois par an ; paliers |
| **Élève mineur** | lit son bulletin, donne son avis sur son orientation | — | être absent de son orientation | **avis recueilli** (D. 331-32), lecture seule de sa fiche | paliers |
| **Élève majeur** | formule lui-même ses demandes (« les parents ou l'élève majeur », D. 331-32 [V]) | — | que ses parents décident encore | circuit à son nom ; parents informés selon son choix (socle 4) | paliers |
| **Commission d'appel** (public : DASEN ; privé : commission propre [V R-B08]) | statue | dossiers incomplets | — | dossier d'appel complet, exporté proprement | juin |
| **Établissement d'accueil, Affelnet, Parcoursup** | reçoivent décisions, bulletins | — | — | exports via les socles 9 et 37 | paliers |
| **DPO, auditeur** | conservation, accès | — | PV et dossiers gardés sans limite | durées (§15.4), journal | sur demande |

**Ce qui fait perdre le plus de temps [H]** : (1) les bulletins papier (impression, mise sous pli, double envoi, réclamations) ; (2) la course aux appréciations avant le conseil (21) ; (3) les décisions saisies deux fois et les vœux sur papier ; (4) les corrections après conseil (tout réimprimer) ; (5) les appels mal outillés (délais, pièces, commission).

---

## 4. Frontières

### 4.1 Tableau
| Sujet | 21 | 22 | **23** | Autres |
|---|---|---|---|---|
| Instantané d'arrêt (moyennes, statuts, appréciations versionnées) | **possède** | fournit ses synthèses | lit ; **ne recalcule rien** | — |
| État d'évaluation de la période par classe | **possède** (`eva_periode_etat`) | lit | fait passer à `conseil_tenu` (événement `conseils.tenu`) et `close` (`bulletins.publies`) | 5 refuse la clôture d'année tant que tout n'est pas clos |
| Appréciations de cours, générale, de vie scolaire, de dispositif | **possède** | — | lit, imprime ; **possède l'appréciation du conseil** (séparée seulement si S-121 du 21 est changé) et **l'appréciation annuelle** (LSL) | — |
| Conseil (planification, participants, séance, PV), avis, mentions | — | — | **possède** | 6 (participants), 46 (délégués élus), 19 (convocations, rendez-vous) |
| Bilan de fin de cycle | — | **possède** | l'**arrête** en séance par commande du 22 | — |
| Procédure d'orientation et de passage (demandes, propositions, décisions, réponses, appel, maintien), fiche de dialogue | — | — | **possède** | **37** possède le parcours Avenir, les entretiens d'orientation, Affelnet, Parcoursup, la fiche Avenir ; 5 lit les décisions pour la montée ; 35 reçoit le dispositif d'accompagnement d'un redoublement |
| Bulletins (contenu, versions, exemplaires, publication), registre, duplicata | fournit l'instantané | fournit les bilans | **possède** | **7** rend le PDF, émet, range au coffre, conserve, lot d'impression ; 8 avise |
| LSL (appréciations annuelles, avis pour l'examen, engagements, compétences) | fournit les moyennes | fournit les compétences | **produit** | 9 transporte (bloqué D-2) ; 46 fournit les engagements |
| Sanctions disciplinaires | — | — | **jamais** (R-V32) | **13** |
| Bulletins déposés depuis Charlemagne (an 1) | V14 (EVA-233 à 237) | — | les **registre** comme bulletins d'origine tierce (§12) | 9 (journal à reporter) |

### 4.2 Ports exposés par le socle 23 [R]
- `conseils::calendrier(tx, classe | annee) -> [ConseilPrevu]` — 21 (fenêtre), 20, 19, 8, élève et famille (date prévue, EVA-269).
- `conseils::fenetre(tx, classe, periode) -> Option<(du, au)>` — 21 (§14.3), 22.
- `conseils::etat(tx, classe, periode) -> EtatConseil` — 21, 5.
- `passage::decision(tx, eleve, annee) -> Option<DecisionDefinitive>` (destination, nature, date, statut d'appel) — **5** (montée, STR-70), 3 (réinscription), 37, 2 (compteur de redoublements).
- `passage::en_attente(tx, annee) -> [Eleve]` — 5 (« élèves sans décision listés »).
- `bulletins::publie(tx, eleve, periode) -> Option<BulletinPublie>` — 2 (chronologie), 7, 9, 37.
- `bulletins::destinataires(tx, eleve, date) -> [Destinataire]` — 7 (exemplaires), 8.
- `lsl::declaration(tx, perimetre) -> DeclarationLsl` — 9.

### 4.3 Ports consommés [R]
- **21** : `instantane`, `etat_periode`, `etat_saisie`, `moyennes_annuelles`, appréciations validées, `fenetre` (lecture de la fenêtre qu'il calcule depuis la date du conseil).
- **22** : `syntheses`, `bilan_cycle`, `arreter_bilans`, `attestations`, compétences du LSL.
- **12** : assiduité de la période (bornée, R-V31) ; **13** : faits de discipline **autorisés** pour le dossier de **préparation** (jamais au bulletin, jamais en vue séance détaillée).
- **4** : liens et droits à la date (`recevoir_bulletins`, `signer_documents`, interdiction), autorité parentale (exercice, retrait), majorité ; **2** : identité, prénom d'usage, compteur de redoublements.
- **5** : classe, niveau, MEF, offre de formation (destinations internes), calendrier et **jours ouvrables** ; **6** : PP, intervenants, présidence possible ; **46** : délégués élus ; **35** : dispositif (libellé), accompagnement d'un redoublement ; **37** : vœux et parcours (lecture) ; **7** : émission, coffre, lien nominatif, lot d'impression, signature ; **8** : avis ; **9** : moteur d'exports.

---
## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

Même méthode et mêmes réserves que `22_COMPETENCES.md` §5 (Légifrance lu par un outil qui résume : le sens est fiable, la lettre à reconfirmer ; eduscol et education.gouv.fr/bo en 403). Les lignes R-07 à R-35 du `21_EVALUATIONS.md` §5 restent valables ; celles qui fondent le 23 sont reprises et **mises à jour** ici.

| # | Règle | Source (consultée le 2026-11-11) | Statut | Conséquence pour ScolIA |
|---|---|---|---|---|
| R-B01 | **Conseil de classe** (public) : présidé par le **chef d'établissement ou son représentant** ; professeurs de la classe, **2 délégués des parents**, **2 délégués des élèves**, CPE, conseiller d'orientation (PsyEN) ; pas de parents pour les formations post-bac | C. éduc. R. 421-50 — https://www.legifrance.gouv.fr/codes/id/LEGISCTA000021754093 | [V] | participants calculés (socles 6, 46) ; **présidence déléguée** possible à un représentant (EVA-256 du 21 ; R-34 du 21 levée) |
| R-B02 | **Médecin, assistant social, infirmier** quand le conseil examine un **cas individuel** | R. 421-50 (même URL) | [V] | invitation **au point d'un élève**, jamais à toute la séance (BUL-127) |
| R-B03 | **Délégués des parents** : « obligation de **confidentialité** » sur les informations personnelles ; calendrier compatible avec leur présence ; **mêmes documents** que les autres membres | C. éduc. D. 111-11, D. 111-12, D. 111-13 — https://www.legifrance.gouv.fr/codes/id/LEGISCTA000006182459 | [V] | rappel de confidentialité à l'ouverture de la séance (lève le [NV] de R-34 du 21) ; « mêmes documents » = la **vue séance** projetée, pas un accès à leur compte ; planification avec un horaire compatible (avertissement si avant 17 h, S23-03) |
| R-B04 | Le conseil se réunit **au moins trois fois par an** ; les lycées professionnels peuvent se limiter à deux | C. éduc. R. 421-51 — URL de R-B01 | [V] | contrôle de planification (avertissement sous 3, ou 2 en LP) |
| R-B05 | **Information des parents** : « tenus régulièrement informés de l'évolution des acquis » (livret au collège, **bulletin et livret au lycée**), à la fréquence fixée par le conseil d'administration ; l'établissement prend les mesures pour que les parents **reçoivent réellement** ces documents ; **toute réponse négative** à une demande des parents est **motivée** | C. éduc. D. 111-3, D. 111-4 — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006182457/ | [V] | accusé de première ouverture, liste « non ouverts » et courrier de secours (socles 7, 8) ; réponse motivée aux contestations (BUL-262) ; **aucun texte national sur le contenu obligatoire du bulletin trimestriel** [NV] → trames D-023 |
| R-B06 | **Parents séparés** : informations aux deux adresses ; le tiers à qui l'enfant est confié a les mêmes droits | Circulaire 2006-137 (21 R-09, R-27 : [V*] par relais) ; texte du BO illisible le 2026-11-11 | [V*] | exemplaire nominatif par lien (inchangé) |
| R-B07 | **Orientation (public)** : demande de la famille (D. 331-31) ; **propositions du conseil de classe**, « l'avis de l'élève mineur est recueilli » (D. 331-32) ; si conforme, **décision** du chef d'établissement (D. 331-33) ; si **désaccord**, **entretien** avec la famille, puis décision **motivée par écrit** (connaissances, capacités, intérêts), **3 jours ouvrables** pour accepter ou faire **appel** (D. 331-34) ; **commission d'appel** présidée par le DASEN, décisions **définitives**, maintien possible (D. 331-35) ; **maintien de droit** une seule année dans la classe d'origine (D. 331-37) ; enseignements optionnels et **spécialités choisis par les parents**, éclairés par l'avis du conseil (D. 331-38) ; « les parents ou l'élève majeur » formulent les demandes (D. 331-32) | C. éduc. D. 331-23 à D. 331-61 — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006166846/ | [V] ; l'expression « dernier mot » n'est pas dans le texte | circuit §9 : demande, proposition, entretien, décision motivée, délai calculé, appel, commission, maintien ; choix des spécialités **par la famille** (le conseil donne un avis) |
| R-B08 | **Privé sous contrat** : procédure miroir (D. 331-46 à D. 331-61, champ L. 442-5 et L. 442-12) : conseil de classe **présidé par le chef d'établissement** (D. 331-52, D. 331-54) ; décision du chef d'établissement, motivation écrite, recours en **3 jours ouvrables** (D. 331-56) ; **commission d'appel** propre (D. 331-57) : au moins deux tiers de chefs d'établissement, professeurs ou parents, aucun membre pour son propre enfant ou élève, décisions définitives, composition communiquée au DASEN | URL de R-B07 (page datée du 09/11/2025) ; D. 331-61 modifié au 15/02/2026 **[à revérifier]** ; composition du conseil de classe dans le privé **[NV]** | [V] / [NV] | un seul circuit, paramétré par le **statut** de l'établissement (public / privé sous contrat) : autorité d'appel, composition de la commission (contrôle « aucun membre pour son enfant ou son élève », BUL-354) |
| R-B09 | **Redoublement** : accompagnement dès la difficulté ; décidé par le **chef d'établissement** « après un dialogue » avec l'élève et ses représentants et **avis du conseil de classe** ; notifié ; **appel** (D. 331-34, -35, -56, -57) ; **dispositif d'accompagnement** spécifique (PPRE, D. 332-6) ; **un seul redoublement au collège**, un second après interruption de scolarité avec accord du DASEN ; appel possible aussi **quand une demande de redoublement est rejetée** (D. 331-63) ; décision opposable au public et au privé sous contrat (D. 331-64) | C. éduc. D. 331-62 à D. 331-64 (décret 2024-228, en vigueur le 18/03/2024) — https://www.legifrance.gouv.fr/codes/id/LEGISCTA000029781614 | [V] | décision « redoublement » de la direction, **jamais** du conseil (R-B7) ; dialogue tracé ; dispositif d'accompagnement exigé (socle 35) ; compteur de redoublements au collège (socle 2) ; appel ouvert dans les deux sens |
| R-B10 | **Sanctions** (modifié par le décret 2026-637, en vigueur le 19/07/2026) : avertissement, blâme, mesure de responsabilisation, exclusions… prononcées par le chef d'établissement (R. 511-14) ou le conseil de discipline ; **aucune mention du conseil de classe** | C. éduc. R. 511-13 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000054446411 | [V] | le conseil ne prononce **aucune sanction** ; « mise en garde » sans valeur disciplinaire ; un avertissement vient du socle 13 |
| R-B11 | **Félicitations, compliments, encouragements, mises en garde** : **aucun texte national** | — | [NV] (rien trouvé, deux recherches) | **données de l'établissement** (D-022), préréglage de 5 (21 Q-EV10) |
| R-B12 | **Livret scolaire du lycée** : moyennes annuelles, **appréciations**, compétences sur 4 degrés, **engagements et responsabilités**, compétences numériques (Pix), **avis de l'équipe pédagogique « très favorable », « favorable », « doit faire ses preuves »**, visa du chef d'établissement ; droits d'accès auprès du chef d'établissement ; modifié jusqu'au 06/05/2026 | Arrêté du 04/03/2020 — https://www.legifrance.gouv.fr/loda/id/JORFTEXT000041814643 ; BO n° 18 du 30/04/2020 ; modification 2026 — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000054245567 | [V] (2020) ; annexes consolidées **[NV]** | trois libellés **fixes** (21 R-32) ; appréciation annuelle par enseignement ; engagements depuis le socle 46 ; visa (BUL-404) ; diffusion à l'élève **[NV]** → recommandation : visible de l'élève avec son dernier bulletin (BUL-406) |
| R-B13 | **Voies et délais de recours** : les délais ne sont **opposables** que s'ils sont **mentionnés** avec les voies de recours dans la notification ; sans mention, recours dans un « délai raisonnable » (en règle générale un an, CE avis n° 474865 du 12/07/2023 [V partiel]) ; motivation des décisions défavorables (CRPA L. 211-2) — l'orientation contraire à la demande est déjà motivée par D. 331-34 | CJA R. 421-5 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006449946 ; CRPA L. 211-2 — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000031367505 | [V] | **toute notification** de décision d'orientation ou de redoublement porte, par la trame (D-023), la **motivation**, la **voie d'appel** (commission, délai de 3 jours ouvrables, date limite calculée) **et** la voie contentieuse ; sans ces mentions, le document **ne part pas** (contrôle bloquant, R-B13) |
| R-B14 | **Archives** : « dossiers scolaires » (bulletins trimestriels, **fiches d'orientation**) DUA **10 ou 50 ans** puis tri ; livret classé au dossier ; commissions d'orientation et d'appel : 2 ans (échelon académique) ; conseils de discipline : 10 ans ; **procès-verbaux des conseils de classe : aucune ligne trouvée** | Instruction DPACI/RES/2005/003, BO n° 24 du 16/06/2005 — copie : https://www.ih2ef.gouv.fr/sites/default/files/2020-07/fiche-dossier-scolaire-instruction-de-tri-et-conservation-des-archives-694.pdf | [V*] ; PV **[NV]** | bulletins et fiches au dossier scolaire (50 ans par défaut, 21 R-35) ; PV : **5 ans** proposés, à valider avec le DPO et l'archiviste (Q-BU12) |
| R-B15 | **Parent sans exercice** de l'autorité parentale : droit de surveiller l'éducation, **informé des choix importants** | C. civ. 373-2-1, al. 5 (21 R-25) | [V] | bulletins et **décisions d'orientation** reçus par défaut (R-V43 du 21) ; il **ne formule pas** les vœux (D. 331-31 : la famille au sens des titulaires de l'autorité) [H juridique, Q-BU7] |
| R-B16 | **Règlement IA** : évaluer les acquis et **déterminer le niveau d'éducation approprié** (orientation) = haut risque ; applicable le 2 décembre 2027 | annexe III point 3 (copie) — https://artificialintelligenceact.eu/annex/3/ ; règlement 2026/1744 — https://eur-lex.europa.eu/eli/reg/2026/1744/oj | [V*] / [V] | l'IA ne propose **ni mention, ni avis, ni orientation, ni passage** (R-V40 du 21) |
| R-B17 | **Fiche Avenir** (Parcoursup) : appréciation du PP, avis du chef d'établissement sur chaque vœu, appréciations disciplinaires remontées par le lycée | 21 R-31 | [V] | socle 37 ; le 23 fournit les appréciations annuelles |

**Non vérifié et interdit de coder dessus** : composition du conseil de classe dans le privé sous contrat ; contenu obligatoire du bulletin trimestriel ; diffusion du LSL à l'élève ; durée de conservation des PV de conseils ; version en vigueur de D. 331-61 ; annexes consolidées du LSL ; XSD du LSL (D-2).

---

## 6. Concepts et vocabulaire [R]

| Terme | Définition | Remplace [F] |
|---|---|---|
| **Série de conseils** | ensemble des conseils d'une période pour un niveau ou l'établissement, avec un calendrier | — |
| **Conseil** | une réunion pour une **classe** et une **période** ; états §7.1 | `ent_conseil_classe` |
| **Participant** | personne convoquée avec un rôle (président, PP, professeur, CPE, PsyEN, délégué des parents, délégué des élèves, invité pour un cas, secrétaire de séance) et sa présence | — |
| **Dossier de conseil** | lecture préparée de l'instantané du 21, des synthèses du 22, de l'assiduité (12), de la fiche de dialogue, des propositions du PP | `ent_conseil_classe_dossier` (en partie) |
| **Examen d'un élève** | ce que le conseil dit d'un élève : appréciation du conseil (= générale par défaut, S-121), **mentions** (0 à n), **avis pour l'examen** (LSL), **propositions** d'orientation ou de passage ; versionné | `mention`, `decision_code`, `avis_orientation` |
| **Mention** | distinction ou mise en garde, **donnée de l'établissement** | 6 valeurs en dur |
| **Procédure de passage** | pour (élève, année) : demande(s), proposition(s), décision, réponse, appel, résultat définitif | `ent_orientation_avis` |
| **Palier** | fin de 3ᵉ, fin de 2ᵈᵉ (et choix de spécialités en fin de 2ᵈᵉ et de 1ʳᵉ) : la procédure D. 331 complète s'applique ; **hors palier** : passage de droit, seul le **redoublement** suit D. 331-62 | — |
| **Destination** | voie, série, formation ou niveau proposé ou décidé (catalogue de l'établissement, préréglage national) ; interne ou externe | `destinationProposee` (Next.js) |
| **Fiche de dialogue** | support de la procédure : la campagne, les étapes datées, les réponses de chaque responsable, l'avis de l'élève, les propositions, la décision, l'acceptation ou l'appel, les documents à chaque étape | `fd_*` (Next.js) |
| **Décision définitive** | ce que la montée lit : décision acceptée, ou décision de la commission d'appel, ou maintien | — |
| **Bulletin** | acte par (élève, période), version, empreinte, origine (`scolia` ou `depose_tiers`) | `ent_bulletin` |
| **Exemplaire** | ce que reçoit **un** destinataire (lien) : canal, document, première ouverture | — |
| **Lot de publication** | ensemble des bulletins d'une classe ou d'un niveau publiés ensemble, avec son contrôle | — |
| **Appréciation annuelle**, **avis pour l'examen** | contenus du LSL (1ʳᵉ, terminale, voie pro) | — |

---

## 7. Le conseil de classe

### 7.1 États [R]
```
planifie ──(J-7 : dossier prêt, instantané vN)──▶ prepare ──(ouverture de séance)──▶ en_seance ──(clôture)──▶ tenu ─(bulletins publiés)─▶ clos
   │                                              ▲                                                     │
   └── annule (motif)                             └──────── reouvert ★ (motif, durée, direction) ◀───────┘
```
- **`prepare`** : l'instantané d'arrêt du 21 est **cité par sa version** (R-V11) ; un rattrapage ou une correction ultérieure crée une nouvelle version et le dossier signale « instantané mis à jour le … » (21 §9.3).
- **`en_seance`** : écriture réservée au président et au secrétaire de séance ; les professeurs déposent des avis jusqu'à l'ouverture.
- **`tenu`** : **verrou** (corrige BD1, BD4) ; émet `conseils.tenu` qui fait passer `eva_periode_etat` à `conseil_tenu` ; les bilans de fin de cycle arrêtés sont figés côté 22 ; le PV est produit.
- **`reouvert`** ★ : direction, motif obligatoire, durée (défaut 48 h), liste des élèves concernés ; chaque changement est une **correction** versionnée ; si un bulletin est déjà publié, la réouverture prépare une version 2 (§10.6).
- **`clos`** : après publication de tous les bulletins de la classe.

### 7.2 Planifier [R]
- Une **série** par période et par niveau (« conseils du 1ᵉʳ trimestre, 3ᵉ ») ; un conseil par classe : date, heure, salle (socle 10), président, secrétaire de séance.
- **Participants calculés** à la date : PP et professeurs des cours de la classe (socle 6, **y compris les professeurs de groupes**, qui ne verront que leurs élèves, D-022), CPE de référence, PsyEN, délégués (socle 46 ; à défaut, désignation par la vie scolaire) ; invités pour un cas (R-B02).
- **Contrôles** : un professeur convoqué à deux conseils simultanés (avertissement) ; horaire incompatible avec la présence des parents délégués (avertissement, R-B03) ; conseil avant la date d'arrêt de la classe (refus) ; moins de 3 conseils dans l'année (2 en LP) (avertissement, R-B04).
- **Convocations** par le socle 8 (avec l'agenda du socle 19) ; les délégués des parents sans compte reçoivent un lien nominatif (socle 7).
- **Fenêtre du conseil** (21 §14.3) : du jour de l'arrêt à J+7 du conseil ; calculée par le 23 et lue par le 21 et le 22.

### 7.3 Préparer : le dossier en un clic [R, A7]
Pour la classe : effectif, moyennes de classe par enseignement (21), dispersion, état de saisie et appréciations manquantes (21), synthèses manquantes (22), assiduité agrégée (12), faits de vie scolaire **agrégés** (13, nombre seulement), **ce qui a changé depuis le dernier conseil** (déterministe, EVA-276), élèves **à discuter** (alertes déterministes du 21 §13.3, marquages des professeurs, demandes de la famille).
Pour chaque élève : moyennes et appréciations versionnées, synthèses (22), assiduité bornée (R-V31), **fiche de dialogue** (vœux, avis de l'élève, désaccord éventuel des responsables), **propositions préparées par le PP** (mention, appréciation du conseil, proposition d'orientation, avis pour l'examen) en **brouillon**, **notes au président** des professeurs absents (M-592 du 21), historique des mentions et des décisions des conseils précédents, bilan de fin de cycle à arrêter (22).
**Exclus du dossier projeté** (vue séance, §7.5) : santé, aménagements, PAP, PPS, dispositifs, discipline détaillée, situation familiale, alertes, notes au président.

### 7.4 Tenir la séance (tablette) [R]
1. **Ouverture** : appel des présents (émargement, BUL-135) ; rappel de confidentialité aux délégués (R-B03) ; choix de la vue projetée.
2. **Tour de classe** : synthèse de la classe par le PP ; interventions des délégués (texte libre pour le PV).
3. **Élève par élève** : l'élève projeté ; le président voit la proposition du PP, les mentions **proposées par les règles de l'établissement** (21 EVA-208 : seuils de moyenne et d'assiduité fixés par l'établissement ; **proposition seulement**, R-V20) ; il arrête en un geste ou modifie ; le secrétaire de séance prend les notes du PV.
4. **Cas individuel** : bouton « **délégués sortis** » (tracé, CE-87 du 21) ; vue complète pour la direction ; invité (infirmière, AS) appelé pour ce seul élève.
5. **Fin de cycle** (6ᵉ, 3ᵉ, dernier conseil) : arrêt des bilans du 22 (CMP-165), un par un ou en lot.
6. **Clôture** : contrôle « élèves non examinés » ; passage à `tenu` ; PV généré (brouillon, relu par le président, puis versé au socle 7) ; **synthèse anonyme** pour les délégués des parents (EVA-257).

### 7.5 Ce que montre la vue séance [R, 21 §14.4 et §14.6]
Nom, prénom d'usage, moyennes, appréciations validées, synthèses, assiduité agrégée (demi-journées), proposition de mention et de décision. **Jamais** : santé, aménagements, PAP, PPS, PPRE, ULIS, UPE2A (même en pictogramme), discipline détaillée, situation familiale, alertes, fiche de dialogue détaillée (seule la **demande de la famille** est projetée à son point : « demande : 2ᵈᵉ générale »), notes au président.

### 7.6 Procès-verbal [R]
Généré depuis la séance : date, présidence (et délégation), présents, excusés, absents, synthèse de la classe, interventions des délégués, **liste des décisions** (mentions, propositions) **sans** les échanges sur les cas individuels hors présence des délégués ; relu et validé par le président ; versé au socle 7 (document émis, immuable) ; consultable par la direction, le PP et les membres présents ; durée de conservation : Q-BU12.

---

## 8. Ce que dit le conseil d'un élève

### 8.1 Appréciation du conseil [R]
Par défaut, **un seul texte** : l'appréciation générale du 21, rédigée par le PP avant ou pendant la séance, **arrêtée** par le président (S-121 du 21) ; l'établissement peut séparer « appréciation générale » (PP) et « appréciation du conseil » (président) : dans ce cas, celle-ci est une table du 23. Longueur et contrôles du 21 §12.2 (santé apparente, prénom d'un autre élève, sanction, CE-93).

### 8.2 Mentions [R]
- **Liste de l'établissement** (`cns_mention_type`) : code, libellé, nature (`distinction` / `mise_en_garde`), ordre, **imprimée ou non**, **règle de proposition** facultative (moyenne générale ≥, aucune moyenne < , assiduité) ; préréglage : félicitations, compliments, encouragements, mise en garde (travail), mise en garde (comportement) [R-B11 : aucun texte].
- **Plusieurs mentions** possibles (une distinction **et** une mise en garde de comportement, par exemple) ; contrôle d'incohérence (deux distinctions) non bloquant.
- **Jamais « avertissement »** ni « blâme » (R-B10) : la création d'un type portant ces mots est **refusée** avec un message qui renvoie au socle 13.
- **Diplôme de mention** imprimable (trames prêtes, D-023), facultatif.

### 8.3 Avis pour l'examen (LSL) [R, R-B12]
En 1ʳᵉ (le cas échéant) et en terminale (voies GT et pro) : « très favorable », « favorable », « doit faire ses preuves » — **trois libellés fixes** ; arrêté au dernier conseil de l'année ; **visé** par le chef d'établissement ; explication à l'élève et demande d'explication tracée (M-637 du 21).

### 8.4 Propositions [R]
Le conseil **propose** : en palier, une **destination** (et un avis par vœu de la famille : favorable, réservé, défavorable — repris de l'existant 181 et de Next.js) ; hors palier, **passage** (par défaut) ou **proposition de redoublement** (exceptionnelle, motif pédagogique, R-B09) ; en fin de 2ᵈᵉ et de 1ʳᵉ, un **avis sur les spécialités** demandées (D. 331-38 : choix des parents) ; éventuellement un **dispositif** (remise à niveau, D. 331-32 ; accompagnement personnalisé ; PPRE). Chaque proposition est une version (auteur, moment) ; elle n'est **jamais** une décision (R-B7).

---

## 9. Orientation et passage : le circuit tracé

### 9.1 Le calendrier type [H, à caler par l'établissement]
| Moment | Palier 3ᵉ / 2ᵈᵉ (D. 331-31 à 38) | Hors palier |
|---|---|---|
| Décembre – février | **vœux provisoires** de la famille (fiche de dialogue, étape 1) ; avis de l'élève recueilli | — |
| Conseil du 2ᵉ trimestre | **avis provisoire** du conseil sur chaque vœu | signalement précoce des difficultés (accompagnement, R-B09) |
| Avril – mai | **vœux définitifs** (étape 2), confirmation ou contradiction du second responsable, accord tacite | dialogue si un redoublement est envisagé |
| Conseil du 3ᵉ trimestre | **proposition** du conseil | avis du conseil sur un redoublement envisagé |
| Juin (J à J+2) | **décision** du chef d'établissement ; si désaccord : **entretien**, décision **motivée** ; notification avec **voies et délais** (R-B13) | décision de redoublement motivée, notifiée, voies et délais ; sinon passage de droit |
| J + 3 jours ouvrables | acceptation, ou **appel** ; à défaut, **acceptation réputée** (à confirmer en droit, Q-BU5) | idem |
| Fin juin – début juillet | **commission d'appel** ; décision définitive ; **maintien** possible (D. 331-37) | idem |

### 9.2 États d'une procédure [R]
```
ouverte ─▶ demande_provisoire ─▶ avis_provisoire ─▶ demande_definitive ─▶ proposition ─┬─(conforme)──────────────▶ decision ─▶ acceptee ─▶ definitive
                                                                                     └─(désaccord)─▶ entretien ─▶ decision_motivee ─┬─▶ acceptee ─▶ definitive
                                                                                                                                     ├─▶ appel ─▶ commission ─▶ definitive (maintien de la décision | réformation)
                                                                                                                                     └─▶ maintien_demande (D. 331-37) ─▶ definitive
```
Mode « **conseil d'abord** » (gabarit Next.js) : la proposition précède la demande définitive ; mode « **famille d'abord** » : l'ordre ci-dessus (terminale normalisée « famille d'abord », NXF-01). **Avenant** possible après acceptation (rectification, nouvelle acceptation, NXF-11).

### 9.3 Les responsables et l'élève [R]
- La **demande** est formulée par **les titulaires de l'autorité parentale** (ou l'**élève majeur**, D. 331-32) : chaque titulaire a **sa propre réponse** (reprise du jeton personnel de Next.js, devenu l'authentification) ; le premier qui dépose fait foi jusqu'à la **confirmation** de l'autre ; **contradiction** → état `conflit`, alerte au PP et à la direction, information des deux responsables (sans révéler la réponse de l'un à l'autre au-delà de ce que la fiche montre aux deux, R-V46), **gel** bloqué ; **résolution** par la direction (`maintien_famille` = la demande du déposant est retenue, ou `avis_conseil` = le conseil statue sans demande commune), motif tracé.
- **Accord tacite** du second titulaire à l'échéance de l'étape, s'il a été **réellement informé** (canal tracé, accusé) ; sinon pas d'accord tacite (correction du risque NXFD-2).
- **Parent sans exercice** : informé de la demande et de la décision (R-B15), **ne dépose pas** ; **tiers digne de confiance** ou service gardien : comme le titulaire, selon la décision judiciaire (socle 4).
- **Interdiction de communication** : exclu de tout.
- **Élève mineur** : son **avis est recueilli** (D. 331-32 [V R-B07]) : écran de l'élève « ce que je souhaite » (texte et vœux), visible du PP et de la famille, projeté au conseil avec la demande (corrige NXFD-9).
- **Élève majeur** : il est le demandeur ; ses parents sont informés selon son choix (socle 4).

### 9.4 Décision du chef d'établissement [R]
- **Conforme** à la demande : décision en lot possible (« toutes les décisions conformes de la 3ᵉ B »), notification.
- **Non conforme** : **entretien proposé** (créneaux du socle 19) ; puis décision **motivée par écrit** (champ obligatoire : connaissances, capacités, intérêts [V R-B07]) ; notification portant **la voie d'appel, le délai de 3 jours ouvrables et la date limite calculée** (jours ouvrables de l'établissement, socle 5), **et** la voie contentieuse (R-B13) ; contrôle bloquant si l'une de ces mentions manque.
- **Spécialités** (fin de 2ᵈᵉ, fin de 1ʳᵉ) : la famille choisit (D. 331-38) ; la décision du chef d'établissement porte sur la **voie** ; les spécialités sont inscrites comme **choix** avec l'avis du conseil, puis transmises au socle 5 (offre, enseignements suivis) et au socle 10 (barrettes, D-033 A).

### 9.5 Redoublement (D. 331-62 à 64) [R]
- **Proposé** par le conseil ou **demandé** par la famille ; **dialogue** avec l'élève et ses représentants **tracé** (date, participants, compte rendu court) **avant** la décision ; **avis du conseil** requis.
- **Décision** du chef d'établissement, motivée, notifiée avec voies et délais ; **dispositif d'accompagnement** obligatoire (socle 35 : PPRE ou autre), créé ou demandé par la décision (tâche au socle 35).
- **Collège** : compteur de redoublements de l'élève (socle 2) ; un second redoublement exige une **interruption de scolarité** et l'**accord du DASEN** (pièce) : contrôle bloquant.
- **Appel** ouvert aussi quand une **demande** de redoublement est **rejetée** (D. 331-63).

### 9.6 Signatures, documents, notifications [R]
Chaque étape produit un **document** (socle 7) : fiche de vœux, avis provisoire, proposition, décision (motivée ou non), acceptation, dossier d'appel, décision de la commission ; immuable, au coffre de chaque titulaire, **au dossier scolaire** de l'élève (R-B14). **Signature du PP** : non exigée (recommandation : le visa de la direction suffit, le PP est cité comme auteur de la proposition ; ferme NXFD-5, S23-06). Signature des responsables : **acceptation** par chaque titulaire si l'établissement l'exige (défaut : un titulaire suffit, l'autre informé ; Q-BU6). **Accusé de réception** à chaque dépôt (ferme NXFD-6) ; chaque titulaire et le parent sans exercice reçoivent la décision.

### 9.7 Appel et commission [R]
- **Ouverture** par un titulaire (ou l'élève majeur) dans le délai ; hors délai : refus motivé, sauf notification irrégulière (R-B13 : délai non opposable).
- **Dossier d'appel** complétable dans le temps (reprise de NXF-12) : pièces de la famille, bulletins, avis, proposition, décision motivée, compte rendu d'entretien ; pièces attendues / reçues ; **export** du dossier vers la commission (socle 7, lien nominatif).
- **Public** : commission présidée par le DASEN [V R-B07] ; l'établissement **saisit** la décision reçue. **Privé sous contrat** : commission propre (composition saisie ; contrôle « aucun membre pour son enfant ou son élève » [V R-B08]).
- **Décision de la commission** : définitive ; **maintien** de la décision ou **réformation** (destination) ; si l'appel échoue en 3ᵉ ou 2ᵈᵉ, **maintien de droit** dans la classe d'origine possible à la demande de la famille (D. 331-37).
- Lecture seule après clôture (reprise de NXF-12).

### 9.8 Ce que lit la montée (socle 5) [R]
Le port `passage::decision(eleve, annee)` renvoie la **décision définitive** (ou `en_attente`) ; la commande de montée du socle 5 (STR-70) **refuse** un élève sans décision définitive en palier, applique le **redoublement** (même niveau, compteur), le **maintien**, le **départ** (destination externe → socle 3 : sortie préparée) ; hors palier, le passage de droit s'applique sans saisie. Les colonnes orphelines (`decision_passage`, `redoublement`, `redoublant`) sont alimentées depuis la décision puis retirées (BD12).

### 9.9 Doublon 2027-2028 (D-034, D-035) [R]
La fiche de dialogue est une fonction de **l'add-on** (Next.js), en **usage réel** : en 2027-2028, ScolIA la **remplace** (régime « usage réel », D-035 point 1), avec un niveau **production** pour la campagne de l'année (D-035 point 2). Les **décisions** d'orientation prises dans ScolIA sont **reportées dans Charlemagne** par le secrétariat (journal à reporter du socle 9, R-X24) tant que Charlemagne fait foi (point pour Florian F23-3 : comment l'établissement reporte-t-il aujourd'hui les décisions de Next.js vers Charlemagne ?). Next.js reste en service tant que le module Rust n'a pas fait ses preuves ; **une campagne commencée dans un outil se termine dans cet outil** (T0 : « finir dans Next.js »).

---
## 10. Bulletins

### 10.1 Quatre trames prêtes (D-023) [R]
| Trame | Pour | Contenu (depuis l'instantané et les ports) |
|---|---|---|
| **Collège — bilan périodique** | 6ᵉ–3ᵉ | par enseignement : éléments du programme travaillés (22), moyenne ou positionnement (21/22, R-C04 du 22), appréciation (progrès, difficultés) ; projets (EPI, AP, parcours dont parcours éducatif de santé, 22) ; **accompagnements** (libellés du 35, exemplaire famille seulement, R-V37) ; vie scolaire : assiduité bornée, ponctualité, demi-journées justifiées et non justifiées (R-V31), appréciation de la CPE ; appréciation générale ; mentions ; en fin d'année de cycle : **bilan de fin de cycle** (22) ; attestations obtenues (22) |
| **Lycée général et technologique** | 2ᵈᵉ–terminale | enseignements (tronc, spécialités, options), moyenne de l'élève, **moyenne du groupe** (R-V30), min/max sous seuil, appréciation, nombre d'évaluations non rendues (R-30 du 21) ; vie scolaire ; appréciation générale et mentions ; en 1ʳᵉ et terminale (3ᵉ trimestre) : rappel de l'avis pour l'examen une fois visé |
| **Lycée professionnel** | CAP, bac pro | enseignements professionnels et généraux, PFMP de la période (socle 39 : dates, lieu, appréciation du tuteur validée), compétences du référentiel si l'établissement le choisit (22), assiduité |
| **Sans note** | classes en positionnements (6ᵉ–4ᵉ, ULIS) | positionnements de synthèse en mots (22), éléments travaillés, appréciations |
Chaque trame : identité de l'établissement (logo, couleurs, signature du chef, coordonnées), **prénom d'usage** (R-V36), statut scolaire ou apprenti, glossaire en ligne (M-640), **aucune sanction** (R-V32), **aucune mention financière** (R-V33), **aucune adresse d'un autre destinataire**, PDF/A et PDF/UA par le moteur du socle 7 (Typst ; corrige BD14). Longueurs maximales affichées à la saisie (jamais de troncature silencieuse).

### 10.2 Générer [R]
- Par **lot** (classe × période) après `tenu` ; refus si le conseil n'est pas tenu (BD2), sauf choix explicite de la direction de publier **sans rubrique du conseil** (CB-31).
- Contenu = instantané du 21 (version citée) + synthèses du 22 + examen du conseil (version arrêtée) + assiduité (12) ; **empreinte** ; état `genere`.
- **Aperçu** en filigrane pour le PP et la direction.

### 10.3 Contrôler avant publication (EVA-246 du 21) [R]
**Bloquants** : destinataire interdit de communication présent ; bulletin sans moyenne ni statut « en attente » pour un enseignement suivi ; destinataire sans aucun canal (ni compte, ni adresse postale) ; élève sorti avant la date d'arrêt ; notification de décision sans voies et délais (R-B13) ; bilan de fin de cycle non visé alors que la trame l'imprime (CC-28 du 22). **Avertissements** : appréciation vide, texte proche de la limite, prénom d'un autre élève, information de santé apparente (R-V18), mention incohérente, deux bulletins d'origines différentes pour la même période (CB-35). Le contrôle produit un **rapport** (par élève, par gravité) ; la publication exige zéro bloquant.

### 10.4 Publier et distribuer [R]
- **Publication** immédiate ou **programmée** par classe ou niveau (« remis en main propre le 12, en ligne le 14 à 18 h », EVA-247) ; heure locale de l'établissement.
- **Destinataires calculés à la publication** (R-V47) : chaque lien `recevoir_bulletins` actif à cette date (dont le parent sans exercice par défaut, R-V43 ; le tiers ou le service gardien selon la décision, R-26 du 21), l'**élève** (s'il a un compte ; majeur : toujours), **jamais** un lien interdit ou retiré.
- **Un exemplaire nominatif par destinataire** (socle 7) : coffre ; **courrier** si aucun compte (lot d'impression par adresse, EVA-248 : jamais deux parents séparés dans une enveloppe, fratries d'un même destinataire regroupées) ; **remise en main propre** tracée (secrétariat ou PP : « remis le … à … »).
- **Avis** `evaluations.bulletin_disponible` (socle 8, enveloppe neutre, U1) ; liste « **non ouverts** à J+7 » pour relance ou courrier (D. 111-3 : les parents doivent **réellement** les recevoir [V R-B05]).
- À la publication : `bulletins.publies` → `eva_periode_etat` passe à `close` quand toute la classe l'est ; appréciations et synthèses deviennent visibles **depuis la version publiée** (corrige BD3).

### 10.5 Ce que la famille et l'élève lisent [R]
Le **PDF** de leur exemplaire et une **vue en ligne** de la même version (lecture de la copie figée, jamais des tables vivantes, BD3) ; glossaire ; date prévue du prochain conseil (EVA-269) ; **demande d'explication** tracée (BUL-262) ; le parent ne voit **jamais** l'activité de l'autre parent (R-V46).

### 10.6 Corriger [R, 21 §9.3]
Après publication : correction ★ par la direction, motif obligatoire ; **version 2** générée, contrôlée, publiée **à tous les destinataires de la v1** (et aux nouveaux destinataires de la date) avec la mention « bulletin rectifié le … » ; la v1 est retirée du coffre, gardée au registre ; renvoi papier de la seule v2 (EVA-249) ; écart signalé si le LSL ou Parcoursup sont déjà partis (CX-45 du 9).

### 10.7 Registre, duplicata, archives, transfert [R, EVA-251 à 255 du 21]
**Registre annuel** par classe (exemplaire établissement, PDF/A), DUA 50 ans par défaut (10 si registre matricule complet, R-B14) ; **duplicata** (mention, identité du demandeur, droits relus, R-V39) ; recherche dans les archives (journalisée) ; **dossier de transfert** (dernier bulletin publié, bilans, attestations, jamais les accompagnements, R-V37) ; **ancien élève** : coffre en lecture un an (Q-EV22 du 21).

---

## 11. Livret scolaire du lycée (LSL)

- **Contenu propre au 23** : **appréciation annuelle** par enseignement (rédigée par le professeur en fin d'année de 1ʳᵉ et de terminale, distincte des appréciations de période, 21 §12.1), **avis pour l'examen** (§8.3), **engagements et responsabilités** (socle 46 : délégué, CVL, éco-délégué… ; ajout manuel par la CPE), **visa** du chef d'établissement.
- **Contenu fourni** : moyennes annuelles (21), compétences et CECRL, Pix (22).
- **Production** par le moteur du socle 9 : **bloquée par D-2** (format officiel) ; d'ici là, **aide à la saisie** (document par classe, ordre des écrans du LSL [H]) et **pré-contrôle** (avis manquant, appréciation annuelle vide, moyenne annuelle manquante, R-V45 du 21).
- **Pilote (D-032)** : le LSL de l'an 1 reste produit par Charlemagne ; ScolIA tient néanmoins les **engagements** (usage réel des fonctions vie scolaire) et peut fournir un export de travail pour leur report.
- **Élève** : voit son avis pour l'examen une fois visé, avec son explication (M-637) [NV R-B12 sur la diffusion].

---

## 12. Cohabitation de l'an 1 et année de doublon (D-032, D-034, D-035)

| Fonction | 2027-2028 au lycée pilote | Ce que fait ScolIA |
|---|---|---|
| Notes, moyennes, appréciations, bulletins | **Charlemagne** (officiel, D-032) | **V14 du 21** : export de l'assiduité, dépôt des bulletins PDF de Charlemagne, rapprochement, **distribution par ScolIA** selon les droits des liens (EVA-233 à 236) ; le 23 enregistre ces bulletins comme bulletins d'**origine `depose_tiers`** (même registre, mêmes exemplaires, même coffre, même duplicata) |
| Conseil de classe | Charlemagne pour la saisie des décisions [H] | **Mode doublon** facultatif : dossier et séance ScolIA construits sur les **moyennes importées** (EVA-236, libellées « importées de Charlemagne le … ») et l'assiduité ScolIA ; mentions et propositions **notées en parallèle**, jamais publiées ; sert à éprouver la séance (D-034) ; recommandation : un ou deux conseils pilotes par trimestre (Q-BU10) |
| Fiche de dialogue, orientation | **Next.js** jusqu'à la preuve, puis **ScolIA** (usage réel, D-035) | §9 ; décisions reportées dans Charlemagne (journal à reporter, R-X24) |
| Montée, constitution des classes N+1 | Charlemagne officiel ; ScolIA en parallèle | lit les décisions de la fiche de dialogue ScolIA (§9.8) |
| LSL, Parcoursup | Charlemagne (éditeur validé) | engagements tenus dans ScolIA ; aide au report |
**Règles** : un bulletin déposé n'est **jamais** distribué sans rapprochement certain (CE-89 du 21) ; la diffusion par le portail de Charlemagne est désactivée pour les bulletins (CE-90 du 21) ; ScolIA **ne génère** aucun bulletin pour ce lycée l'an 1.

---

## 13. Cas limites (catalogue)

### 13.1 Conseil
| # | Cas | Règle [R] |
|---|---|---|
| CB-01 | Président empêché le jour même | délégation changeable le jour même (EVA-256, CE-86 du 21) ; tracée au PV |
| CB-02 | Professeur absent au conseil | **note au président** par élève avant la séance ; mentionnée « absent excusé » au PV |
| CB-03 | Professeur de groupe (spécialité, LV2) dans six classes | il voit, pour chaque conseil, **ses seuls élèves** de la classe (D-022), pendant la fenêtre de **cette** classe (21 §14.6 point 8) |
| CB-04 | Délégué des élèves dont le cas est examiné | sortie de la vue séance tracée pour son point (« délégués sortis ») |
| CB-05 | Délégué des parents parent d'un élève de la classe | idem pour le point de son enfant ; rappel de confidentialité (R-B03) |
| CB-06 | Conseil reporté | nouvelle date ; fenêtre recalculée ; convocations renvoyées ; publication programmée décalée si elle dépend du conseil |
| CB-07 | Conseil tenu à distance (visio, socle 33) | même séance ; présence « à distance » ; vue séance partagée sans téléchargement |
| CB-08 | Élève arrivé après l'arrêt | examiné avec les seules données disponibles ; mention « arrivé le … » ; aucune mention proposée par règle |
| CB-09 | Élève parti avant le conseil | non examiné ; son bulletin de la période est produit depuis l'état figé au départ (R-V42) si l'établissement le choisit |
| CB-10 | Changement de classe entre l'arrêt et le conseil | examiné par la classe de rattachement **à la date d'arrêt** (R-V28) |
| CB-11 | Instantané reversionné après la préparation (rattrapage) | dossier mis à jour, bandeau « mis à jour le … » ; si `tenu`, réouverture ★ nécessaire pour changer une décision |
| CB-12 | Égalité de vues sur une mention | le président tranche ; aucune règle de vote codée |
| CB-13 | Mention proposée par la règle alors qu'une sanction est en cours (socle 13) | proposition affichée, **sans** révéler la sanction en vue séance ; le président décide (R-V20) |
| CB-14 | Panne réseau pendant la séance | saisie locale sur la tablette (hors ligne, version), synchronisée ; refus si une autre saisie a eu lieu entre-temps (R-V27) |
| CB-15 | Classe à double PP | les deux préparent ; un seul « présente » |
| CB-16 | Conseil d'une classe de dispositif (ULIS, UPE2A) | l'élève est examiné dans sa **classe de référence** ; le coordonnateur est invité |

### 13.2 Décisions, orientation, passage
| # | Cas | Règle [R] |
|---|---|---|
| CB-17 | Responsables en désaccord sur les vœux | état `conflit`, résolution motivée par la direction (§9.3) ; jamais la réponse de l'un montrée à l'autre au-delà de la fiche commune |
| CB-18 | Second responsable injoignable (aucun canal tracé) | **pas d'accord tacite** ; la direction constate et poursuit avec le déposant (motif) |
| CB-19 | Parent sans exercice qui veut déposer des vœux | refus poli ; il est informé (R-B15) |
| CB-20 | Élève majeur en cours d'année (18 ans en mars) | le demandeur devient l'élève à la date ; les demandes déjà faites par les parents restent, l'élève peut les modifier jusqu'à la clôture de l'étape |
| CB-21 | Famille sans réponse à l'étape des vœux définitifs | relances ; le conseil propose sans demande ; décision « conforme » impossible sans demande : entretien proposé |
| CB-22 | Famille qui ne répond pas à la décision dans les 3 jours ouvrables | acceptation réputée **[NV, Q-BU5]** ; recommandation : la notification le dit en toutes lettres |
| CB-23 | Notification sans voies et délais (erreur) | contrôle bloquant (R-B13) ; si elle est partie quand même (reprise d'archive), le délai n'est pas opposable : appel accepté hors délai |
| CB-24 | Jour férié ou vacances dans le délai d'appel | jours ouvrables de l'établissement (socle 5), samedi selon le calendrier de l'établissement [NV : définition des jours ouvrables à confirmer, Q-BU5] |
| CB-25 | Appel puis désistement | état `appel_retire` ; la décision initiale devient définitive |
| CB-26 | Commission d'appel qui réforme vers une formation pleine | décision définitive enregistrée ; la place relève de l'affectation (socle 37 / Affelnet) |
| CB-27 | Demande de redoublement de la famille rejetée | appel possible (D. 331-63 [V R-B09]) |
| CB-28 | Second redoublement au collège | refus sans pièce d'interruption de scolarité et accord du DASEN (R-B09) |
| CB-29 | Élève de 3ᵉ qui part dans le privé hors contrat ou à l'étranger | destination externe « autre » ; décision du palier quand même si demandée |
| CB-30 | Changement d'avis de la famille après acceptation | **avenant** (NXF-11), nouvelle acceptation, archive |
| CB-31 | Bulletin à publier avant la tenue du conseil (vacances) | choix explicite de la direction : bulletin **sans** rubrique du conseil, puis version 2 après le conseil ; jamais de proposition imprimée comme décision (BD2) |
| CB-32 | Spécialité abandonnée en fin de 1ʳᵉ que la famille veut garder | choix des parents (D. 331-38) éclairé par l'avis ; contrainte d'offre (socle 5) traitée à part |
| CB-33 | Élève en double scolarité (CNED partiel) | examiné par l'établissement de rattachement ; moyennes externes (R-V38) |
| CB-34 | Décision prise, puis élève qui déménage | décision définitive transmise au dossier de transfert ; la montée N+1 se fait ailleurs |

### 13.3 Bulletins
| # | Cas | Règle [R] |
|---|---|---|
| CB-35 | Bulletin Charlemagne déposé **et** bulletin ScolIA pour la même période (erreur) | contrôle : un seul bulletin publiable par (élève, période) ; le second est refusé à la publication |
| CB-36 | Parent dont le droit est retiré entre la génération et la publication | non destinataire (R-V47) |
| CB-37 | Lien créé après la publication (nouveau tiers) | reçoit le bulletin publié si la direction le décide (geste « mettre à disposition », R-V39) |
| CB-38 | Élève majeur qui a retiré l'accès de ses parents | parents non destinataires ; mention aucune |
| CB-39 | Ordonnance de protection : établissement caché à un parent | aucun document à l'en-tête de l'établissement vers lui (CE-73 du 21) |
| CB-40 | Adresse postale inconnue et aucun compte | bloquant au contrôle ; tâche au secrétariat |
| CB-41 | Courrier revenu « n'habite pas à l'adresse » | exemplaire marqué « non distribué », tâche de mise à jour (socle 4) |
| CB-42 | Bulletin réclamé par un établissement d'accueil | jamais sans demande vérifiée (INS-75 du 3) ; dossier de transfert |
| CB-43 | Famille qui conteste une appréciation | demande d'explication tracée ; correction ★ possible ; réponse **motivée** si refus (D. 111-4 [V R-B05]) |
| CB-44 | Impayé de cantine | **aucune** incidence (R-V33) |
| CB-45 | Prénom d'usage décidé en cours d'année | les bulletins suivants le portent ; les anciens ne sont pas réémis sauf demande ★ (R-V36) |
| CB-46 | Élève décédé | aucun envoi automatique ; bulletins gelés ; la direction décide (socle 2) |

---

## 14. Intelligence artificielle (contrat IA du socle)

**Fait** [R] : (1) **relecture d'un lot de bulletins** avant publication (remarques à la direction, jamais de modification, EVA-275 du 21, après l'étude juridique) ; (2) **synthèse de la classe** pour le PP (déterministe d'abord : chiffres, évolutions ; reformulation par modèle facultative) ; (3) **« ce qui a changé depuis le dernier conseil »** (déterministe, EVA-276) ; (4) **brouillon du PV** à partir des notes du secrétaire de séance (relu par le président) ; (5) **synthèse anonyme** pour les délégués des parents (brouillon, validée) ; (6) **explication d'une décision** à la famille à partir du texte motivé (jamais d'argument nouveau) ; (7) **traduction** facultative d'un bulletin ou d'une décision pour une famille allophone, marquée « traduction automatique, seul le texte français fait foi ».
**Ne fait pas** [R, R-V40, R-B16] : aucune proposition de **mention**, d'**avis pour l'examen**, d'**orientation**, de **passage** ou de **redoublement** ; aucun classement d'élèves ; aucune lecture de la santé, des accompagnements, de la discipline détaillée, de la situation familiale ; rien en vue séance.
**Capacités** : `conseils.dossier.lire` (C2, portée D-022 et fenêtre), `bulletins.publie.lire` (C2, exemplaire du lecteur seulement), `passage.decision.lire` (C2), `BUL-150` brouillon de PV (N2), `BUL-212` relecture d'un lot (N1, aucune écriture), `BUL-262` explication à la famille (N1). **Jeu d'évaluation** : refus de « propose les félicitations pour la 2ᵈᵉ A », « faut-il faire redoubler Hugo ? », « montre le PAP de Léa pendant le conseil ».

---

## 15. Droits, minimisation, conservation

### 15.1 Catalogue d'actions (D-017) [R]
| Action | Portée | Modèles par défaut | Remplace [F] |
|---|---|---|---|
| `conseils.lire` | classe ; groupe (D-022) pendant la fenêtre | professeurs (leurs élèves), PP, CPE, direction | `conseils.gerer` (lecture) |
| `conseils.planifier` | établissement | direction, secrétariat | `conseils.gerer` |
| `conseils.preparer` (propositions en brouillon, notes au président) | classe | PP ; professeurs (avis et notes au président pour leurs élèves) | — |
| `conseils.tenir` ★ (séance, arrêter, clôturer, PV) | conseil présidé | chef d'établissement, adjoint, **président délégué** | `conseils.gerer` + `eva_peut_decider` |
| `conseils.deleguer` ★ | établissement | chef d'établissement | — |
| `conseils.rouvrir` ★ | établissement | direction | — |
| `conseils.parametrer` (mentions, règles de proposition, trames) | établissement | direction | — |
| `orientation.campagnes_gerer` | établissement | secrétariat, direction | — |
| `orientation.suivre` (dossier famille, relances) | classe | PP, direction, secrétariat ; PsyEN (élèves suivis, lecture) | — |
| `orientation.decider` ★ | établissement | chef d'établissement, adjoint | — |
| `orientation.appels_gerer` | établissement | direction, secrétariat | — |
| `orientation.repondre` | soi (titulaire, élève majeur) | familles, élève majeur | — |
| `orientation.avis_eleve` | soi | élève | — |
| `bulletins.generer` | classe | secrétariat, direction | — |
| `bulletins.publier` ★ | établissement | direction ; secrétariat si délégué | `bulletins.publier` |
| `bulletins.corriger` ★ | établissement | direction | — |
| `bulletins.lire` (registre, duplicata, archives) | établissement | secrétariat, direction | — |
| `bulletins.remettre` (remise en main propre tracée) | classe | PP, secrétariat | — |
| `lsl.saisir` (appréciation annuelle) | cours | professeur | — |
| `lsl.viser` ★ (avis pour l'examen) | établissement | chef d'établissement | — |
| `exports.officiels` (LSL) ★ | établissement | secrétariat, direction ; **jamais professeur** | inchangé |

### 15.2 Ce que voit chacun [R]
| Qui | Voit | Ne voit pas |
|---|---|---|
| Professeur | dossier du conseil pour **ses** élèves pendant la fenêtre ; vue séance quand il est présent | les autres élèves ; fiches de dialogue détaillées ; notes au président des collègues |
| PP | dossier de sa classe, fiches de dialogue de sa classe, procédures en cours | dossiers médicaux, sociaux (36) |
| CPE | dossier des élèves suivis ; fiches de dialogue en lecture | — |
| PsyEN | procédures d'orientation des élèves qu'il suit (lecture) | bulletins hors ceux-ci |
| Direction | tout | — |
| Secrétariat | planification, campagnes, bulletins, registre, appels | contenu des notes au président |
| Délégués | **vue séance projetée seulement**, rien sur leur compte (sauf la synthèse anonyme validée pour les parents) | tout dossier |
| Famille | ses exemplaires, sa fiche de dialogue, ses décisions | l'activité de l'autre parent (R-V46) ; les propositions non arrêtées ; le PV |
| Élève | ses bulletins, sa fiche (lecture), son avis, ses décisions | idem |
| AESH, accueil, AED, infirmière (hors invitation) | rien | tout |

### 15.3 Garde-fous [R]
RLS forcée sur les nouvelles tables (E13), FK composées (E2), `etablissement_id NOT NULL` (E1) ; journal des **consultations** de dossiers de conseil et de bulletins par le personnel (R-V26) ; journal de toute décision, réouverture, correction, délégation (E7) ; événements sans texte ni décision (R-V17) ; matrice de séparation : professeur d'un autre groupe, professeur hors fenêtre, président non délégué, PP d'une autre classe, délégué qui tente le dossier, parent sans `recevoir_bulletins`, parent sans exercice (lecture oui, dépôt non), parent interdit, élève majeur ayant retiré l'accès, établissement B, appli mobile, capacité IA.

### 15.4 Conservation (à valider avec le DPO et l'archiviste) [R]
| Donnée | Durée proposée | Base |
|---|---|---|
| Bulletins publiés (toutes versions), registre annuel | **dossier scolaire : 50 ans** (10 si registre matricule complet) | R-B14 [V*] ; 21 R-35 |
| Documents de la procédure d'orientation (fiches, décisions, appels) | dossier scolaire | R-B14 (« fiches d'orientation ») |
| Dossier d'appel complet (pièces) | 2 ans après la décision définitive (aligné sur les commissions, R-B14), puis seules la décision et sa motivation restent au dossier | R-B14 [V*] ; Q-BU12 |
| PV de conseil, liste des présents | **5 ans** | [NV] → Q-BU12 |
| Notes au président, avis internes, propositions non retenues | fin de l'année scolaire + 1 an | minimisation |
| Journaux de consultation | 1 an | E7 ; R-21 du 21 |

---

## 16. Notifications (socle 8)

| Nature | Urgence | Destinataires | Canal | Notes |
|---|---|---|---|---|
| `conseils.convocation` | U1 | participants | in-app, e-mail, agenda (19) | délégués sans compte : lien nominatif |
| `conseils.rappel_preparation` | U0 | PP, professeurs | in-app | J-3 : propositions, notes au président |
| `conseils.pv_disponible` | U0 | président, présents | in-app | — |
| `evaluations.bulletin_disponible` [F socle 8] | U1 | chaque exemplaire (lien `recevoir_bulletins`, élève) | in-app, e-mail **enveloppe**, push ; courrier si aucun | jamais le contenu |
| `bulletins.non_ouvert` | U0 | secrétariat | in-app | J+7 : liste à relancer |
| `bulletins.rectifie` | U1 | destinataires de la v1 | idem | « bulletin rectifié » |
| `orientation.etape_ouverte` | U1 | titulaires, élève majeur, élève (lecture) | in-app, e-mail, push | **information due** ; échéance |
| `orientation.accuse_depot` | U0 | déposant | in-app, e-mail | ferme NXFD-6 |
| `orientation.confirmation_demandee` | U1 | second titulaire | idem | échéance, effet de l'accord tacite écrit |
| `orientation.conflit` | U1 | PP, direction ; information des deux titulaires | in-app | sans le contenu de l'autre réponse |
| `evaluations.decision_conseil` [F socle 8] | U1 | titulaires, parent sans exercice, élève majeur | in-app, e-mail enveloppe ; courrier recommandé si non conforme et aucun compte [H : à confirmer, Q-BU5] | document de décision au coffre, **voies et délais** |
| `orientation.entretien_propose` | U1 | titulaires | in-app, e-mail | créneaux (19) |
| `orientation.delai_appel` | U1 | titulaires | in-app, push | J+1 et veille de l'échéance |
| `orientation.appel_recu`, `.appel_decide` | U1 | direction / titulaires | in-app, e-mail | — |
| `competences.bilan_cycle_disponible` | U1 | avec le bulletin | — | un seul avis (22 §16) |

---
## 17. Actions métier (BUL-100 et suivantes)

Acteurs : **DIR** direction (chef, adjoint), **PRES** président de séance (DIR ou délégué), **PP**, **PRF** professeur, **CPE**, **ADF** secrétariat, **SEC** secrétaire de séance, **DEL** délégué, **PAR** responsable titulaire, **PSE** parent sans exercice, **ELV** élève (mineur), **ELM** élève majeur, **SYS**. ★ = confirmation renforcée et journal. Les actions EVA-2xx du 21 qui relèvent du 23 sont **reprises** sous leur identifiant (entre parenthèses) pour ne pas les dupliquer.

### 17.1 Paramétrer et planifier
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| BUL-100 | Gérer la liste des mentions (nature, libellé, impression, règle de proposition) | DIR · `conseils.parametrer` | refus de « avertissement », « blâme » (R-B10) | `conseils.mentions_maj` |
| BUL-101 | Choisir la trame de bulletin par niveau et l'habiller (logo, couleurs, signature, textes encadrés) | DIR | 4 trames (D-023) | `bulletins.trame_choisie` |
| BUL-102 | Choisir « appréciation générale = appréciation du conseil » ou deux textes | DIR | S-121 du 21 | — |
| BUL-103 | Paramétrer le catalogue de destinations (interne : offre du socle 5 ; externe : préréglage national) | DIR, ADF | NXF-02 | `orientation.destinations_maj` |
| BUL-104 | Déclarer le statut de l'établissement pour l'orientation (public / privé sous contrat) et l'autorité d'appel | DIR | R-B07, R-B08 | — |
| BUL-105 | Créer une série de conseils (période, niveau) et générer les conseils des classes | DIR, ADF · `conseils.planifier` | après l'arrêt de chaque classe ; contrôles §7.2 | `conseils.serie_creee` |
| BUL-106 | Fixer date, heure, salle d'un conseil | ADF | conflits de participants et de salle (socle 10) | `conseils.planifie` |
| BUL-107 | Désigner le président (délégation) d'un conseil ou d'une série (EVA-256) | DIR · `conseils.deleguer` ★ | changeable le jour même | `conseils.presidence_deleguee` |
| BUL-108 | Désigner le secrétaire de séance | PRES, ADF | — | — |
| BUL-109 | Calculer et ajuster la liste des participants | SYS, ADF | §7.2 ; délégués du socle 46 ou désignés | — |
| BUL-110 | Convoquer (personnel, délégués) avec rappel | SYS, ADF | socles 8, 19 ; lien nominatif pour les délégués sans compte | `conseils.convocation` |
| BUL-111 | Reporter ou annuler un conseil | DIR, ADF | motif ; fenêtre recalculée (CB-06) | `conseils.reporte` |
| BUL-112 | Voir le calendrier des conseils (établissement, classe) | personnels ; ELV, PAR (date prévue, EVA-269) | — | — |
| BUL-113 | Afficher aux familles la date prévue du conseil et de la publication | SYS | « date prévue » | — |
| BUL-114 | Copier la série de l'an passé (dates décalées) | ADF | gain de temps | — |

### 17.2 Préparer
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| BUL-120 | Produire le dossier du conseil depuis l'instantané (EVA-207) | SYS, PP, DIR | version de l'instantané citée | `conseils.dossier_pret` |
| BUL-121 | Voir la synthèse de la classe et « ce qui a changé » (EVA-276) | PP, DIR, PRF (fenêtre) | déterministe | — |
| BUL-122 | Lister les élèves à discuter (EVA-209) et en marquer (EVA-210) | SYS, PP, PRF (ses élèves) | motivé | — |
| BUL-123 | Préparer, par élève, les propositions en brouillon : appréciation du conseil, mentions, proposition d'orientation, avis pour l'examen (EVA-211) | PP · `conseils.preparer` | prérempli depuis la demande de la famille (NXF-09) et les mentions proposées par règle (EVA-208) | — |
| BUL-124 | Déposer une **note au président** pour un élève (professeur absent) | PRF · `conseils.preparer` | jamais projetée ; supprimée après la fin de l'année + 1 an | — |
| BUL-125 | Déposer un avis par élève (professeur présent ou non) | PRF | jusqu'à l'ouverture de la séance | — |
| BUL-126 | Voir l'état de préparation (appréciations, synthèses, propositions, fiches de dialogue) | PP, DIR | relance en un geste (EVA-204 du 21) | — |
| BUL-127 | Inviter un participant pour un **cas individuel** (infirmière, AS, médecin) | PRES, PP | invitation au point de l'élève seulement (R-B02) | `conseils.invitation` |
| BUL-128 | Préparer la synthèse de classe pour le compte rendu des délégués des parents (EVA-257) | PP | brouillon, validée par le président | — |
| BUL-129 | Ouvrir le dossier d'un élève en **vue complète** (préparation) | PP, DIR | journalisé (R-V26) | — |
| BUL-130 | Proposer un rendez-vous à une famille depuis le dossier (M-600 du 21) | PP | socle 19 | — |

### 17.3 Tenir la séance
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| BUL-135 | Ouvrir la séance, émarger les présents, rappeler la confidentialité (EVA-258) | PRES, SEC · `conseils.tenir` | R-B03 | `conseils.seance_ouverte` |
| BUL-136 | Choisir la vue projetée (séance minimisée) ; basculer en vue complète hors délégués | PRES | §7.5 | — |
| BUL-137 | Marquer « délégués sortis » / « délégués revenus » pour un élève | PRES | tracé (CE-87) | `conseils.delegues_sortis` |
| BUL-138 | Naviguer élève par élève (ordre alphabétique, élèves à discuter d'abord) | PRES | — | — |
| BUL-139 | Arrêter l'appréciation du conseil d'un élève | PRES | versionné | `conseils.examen_arrete` |
| BUL-140 | Arrêter les mentions d'un élève (0 à n) | PRES | propositions par règle affichées, jamais appliquées seules (R-V20) | idem |
| BUL-141 | Arrêter la proposition d'orientation ou de passage | PRES | proposition, jamais décision (R-B7) | idem |
| BUL-142 | Arrêter l'avis sur chaque vœu (favorable, réservé, défavorable) | PRES | — | idem |
| BUL-143 | Arrêter l'avis sur les spécialités demandées | PRES | D. 331-38 | idem |
| BUL-144 | Arrêter l'avis pour l'examen (1ʳᵉ, terminale) | PRES | trois libellés fixes (R-B12) | idem |
| BUL-145 | Proposer un redoublement (avec motif pédagogique) | PRES | ouvre le dialogue (§9.5) | `orientation.redoublement_propose` |
| BUL-146 | Arrêter les bilans de fin de cycle (CMP-165, CMP-166 du 22) | PRES | R-C11 du 22 | `competences.bilan_arrete` |
| BUL-147 | Arrêter en lot les élèves sans remarque | PRES | aperçu de la liste | `conseils.examen_arrete` |
| BUL-148 | Prendre les notes du PV (interventions, synthèse) | SEC | texte libre ; jamais de cas individuel hors délégués | — |
| BUL-149 | Clôturer la séance (contrôle des non-examinés) → `tenu` | PRES ★ | verrou (BD1) | `conseils.tenu` |
| BUL-150 | Générer le brouillon du PV (avec aide IA facultative) | SYS | §7.6 ; relu par le président | `ia.brouillon_cree` |
| BUL-151 | Valider et verser le PV (EVA-258) | PRES | document émis (socle 7) | `conseils.pv_produit` |
| BUL-152 | Valider la synthèse anonyme pour les délégués des parents (EVA-257) | PRES | aucune donnée individuelle | `conseils.synthese_validee` |
| BUL-153 | Tenir la séance hors ligne sur tablette et synchroniser | PRES, SEC | CB-14 | — |
| BUL-154 | Tenir un conseil à distance (visio) | PRES | socle 33 ; vue partagée sans téléchargement | — |

### 17.4 Après la séance
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| BUL-170 | Rouvrir un conseil tenu ★ (motif, durée, élèves) | DIR · `conseils.rouvrir` | §7.1 | `conseils.rouvert` |
| BUL-171 | Corriger un examen d'élève pendant la réouverture | PRES | version ; si bulletin publié, prépare la v2 | `conseils.examen_corrige` |
| BUL-172 | Voir l'historique d'un examen (versions, auteurs, motifs) | DIR | — | — |
| BUL-173 | Voir l'effet d'une correction de note sur l'examen (« la moyenne passe de 9,96 à 10,04 ») | DIR | 21 §9.3 ; aucune décision ne change seule | — |
| BUL-174 | Imprimer les diplômes de mentions | ADF | trames prêtes | `documents.emis` |

### 17.5 Bulletins
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| BUL-200 | Générer les bulletins d'une classe | ADF, DIR · `bulletins.generer` | après `tenu` (BD2) ou sans rubrique du conseil (CB-31) | `bulletins.generes` |
| BUL-201 | Voir l'aperçu d'un bulletin (filigrane) | PP, DIR, ADF | — | — |
| BUL-202 | Lancer le contrôle avant publication (EVA-246) | ADF, DIR | §10.3 | `bulletins.controle_realise` |
| BUL-203 | Corriger les bloquants depuis le rapport (liens vers l'écran) | ADF | — | — |
| BUL-204 | Publier maintenant ou programmer par classe ou niveau (EVA-247) | DIR · `bulletins.publier` ★ | destinataires à la publication (R-V47) | `bulletins.publies` / `.publication_programmee` |
| BUL-205 | Annuler une publication programmée | DIR | avant l'heure | — |
| BUL-206 | Produire le lot d'impression par adresse (EVA-248) | ADF | socle 7 | `documents.lot_imprime` |
| BUL-207 | Tracer une remise en main propre (élève, classe) | PP, ADF · `bulletins.remettre` | date, personne | `bulletins.remis` |
| BUL-208 | Voir les exemplaires non ouverts à J+7 et relancer ou envoyer un courrier | ADF | D. 111-3 [V R-B05] | `bulletins.non_ouvert` |
| BUL-209 | Mettre à disposition d'un nouveau destinataire un bulletin déjà publié | ADF ★ | relit les droits (R-V39) ; CB-37 | `bulletins.exemplaire_ajoute` |
| BUL-210 | Corriger un bulletin publié ★ → version 2 | DIR · `bulletins.corriger` | §10.6 | `bulletins.rectifie` |
| BUL-211 | Réimprimer la seule v2 pour les destinataires papier (EVA-249) | ADF | — | `documents.lot_imprime` |
| BUL-212 | Faire relire un lot par l'IA (remarques) (EVA-275) | ADF, DIR | après l'étude juridique ; aucune écriture | `ia.remarques_bulletins` |
| BUL-213 | Produire un duplicata (EVA-251) | ADF · `bulletins.lire` | mention ; identité du demandeur | `bulletins.duplicata` |
| BUL-214 | Rechercher dans les archives (EVA-252) | ADF | journalisé | — |
| BUL-215 | Constituer le registre annuel (EVA-254) | ADF, SYS | 50 ans par défaut | `bulletins.registre_constitue` |
| BUL-216 | Joindre au dossier de transfert (EVA-255) | ADF | jamais les accompagnements (R-V37) | — |
| BUL-217 | Produire les relevés en lot pour une rencontre parents-professeurs (EVA-250) | ADF, PP | publiés seulement | `documents.lot_produit` |
| BUL-218 | Voir, au guichet, les restrictions du lien du demandeur (EVA-244) | SYS → ADF | bandeau sans motif | — |
| BUL-219 | Distribuer les bulletins déposés de Charlemagne (EVA-235) | ADF, DIR ★ | origine `depose_tiers` ; mêmes règles | `bulletins.publies` |

### 17.6 Familles et élèves (bulletins)
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| BUL-250 | Lire son exemplaire (PDF et vue en ligne de la même version) | PAR `recevoir_bulletins`, PSE (défaut), ELV, ELM | copie figée (BD3) | `bulletins.ouvert` (accusé) |
| BUL-251 | Télécharger les bulletins des années précédentes (coffre) | PAR, ELV, ELM ; ancien élève 1 an | — | — |
| BUL-252 | Lire le glossaire (EVA-272) | PAR, ELV | — | — |
| BUL-253 | Voir la date du prochain conseil (EVA-269) | PAR, ELV | — | — |
| BUL-260 | Demander une explication sur une appréciation ou une mention | PAR, ELV, ELM | adressée au PP ; tracée | `bulletins.explication_demandee` |
| BUL-261 | Demander une rectification d'une erreur matérielle (nom, enseignement) | PAR, ELM | RGPD art. 16 pour les données factuelles ; correction ★ | `bulletins.rectification_demandee` |
| BUL-262 | Répondre à une demande (motivée si refus) | PP, DIR | D. 111-4 [V R-B05] ; aide IA possible (§14) | `bulletins.demande_repondue` |
| BUL-263 | Recevoir l'avis de publication sans contenu | PAR, ELV | socle 8 | — |
| BUL-264 | Lire une traduction automatique d'un bulletin | PAR | mention « seul le français fait foi » | — |

### 17.7 Orientation : campagnes (reprise de Next.js)
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| BUL-300 | Créer une campagne de fiches de dialogue par niveau depuis un gabarit (NXF-01) | ADF, DIR · `orientation.campagnes_gerer` | mode « famille d'abord » / « conseil d'abord » | `orientation.campagne_creee` |
| BUL-301 | Fixer les étapes et leurs dates (NXF-03) | ADF | cohérentes avec les conseils (BUL-105) | — |
| BUL-302 | Choisir les destinations et champs proposés par niveau (NXF-02) | ADF, DIR | catalogue BUL-103 | — |
| BUL-303 | Copier la campagne de l'an passé | ADF | — | — |
| BUL-304 | Prévisualiser la fiche telle que la famille la verra | ADF, PP | — | — |
| BUL-305 | Générer les fiches des élèves (procédures) | SYS, ADF | élèves du niveau à la date ; rapport `créées / ignorées / erreurs` (corrige l'assistant silencieux de Next.js) | `orientation.fiches_generees` |
| BUL-306 | Ouvrir une étape (avis aux titulaires, élève majeur ; élève en lecture) | SYS, ADF | destinataires = liens à la date (NXFD-2) | `orientation.etape_ouverte` |
| BUL-307 | Relancer les attentes (dépôts et confirmations) | ADF | sans révoquer quoi que ce soit (NXFD-7) | `orientation.relance` |
| BUL-308 | Voir l'état de la campagne (filtres statut, classe, recherche ; « prochaine action ») | ADF, PP, DIR | NXFD-8 | — |
| BUL-309 | Geler la campagne avant le conseil (liste nominative des bloquées) (NXF-07) | DIR | refus si conflits non résolus | `orientation.gel` |
| BUL-310 | Dégeler | DIR | motif | `orientation.degel` |
| BUL-311 | Ouvrir le dossier famille d'une fiche (NXF-08) | PP, DIR, ADF · `orientation.suivre` | sans secrets ; journalisé | — |
| BUL-312 | Clôturer la campagne (liens coupés, récapitulatif, export des non traitées) (NXF-15) | DIR | ferme le manque « récapitulatif » de Next.js | `orientation.campagne_close` |
| BUL-313 | Archiver la campagne | DIR | documents au dossier scolaire | — |
| BUL-314 | Saisir une réponse papier pour une famille sans compte | ADF | pièce ; mode « papier » | `orientation.demande_deposee` |

### 17.8 Orientation : responsables et élève
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| BUL-320 | Déposer ses vœux (rang, destination, spécialités, commentaire) | PAR titulaire, ELM · `orientation.repondre` | NXF-05 ; accusé (NXFD-6) | `orientation.demande_deposee` |
| BUL-321 | Corriger ses vœux jusqu'à la clôture de l'étape | PAR, ELM | historique | idem |
| BUL-322 | Confirmer ou contredire la demande de l'autre titulaire | PAR | NXF-06 | `orientation.demande_confirmee` / `.conflit` |
| BUL-323 | Appliquer l'accord tacite à l'échéance (si information tracée) | SYS | §9.3 ; CB-18 | `orientation.accord_tacite` |
| BUL-324 | Résoudre un conflit (`maintien_famille` / `avis_conseil`), motif | DIR ★ | NXF-06 | `orientation.conflit_resolu` |
| BUL-325 | Informer le parent sans exercice de la demande et de la décision | SYS | R-B15 | — |
| BUL-326 | Voir la fiche (lecture) | PSE, ELV | NXF-14 | — |
| BUL-360 | Voir sa fiche en lecture seule | ELV | rattachement par l'identité ScolIA (NXFD-1) | — |
| BUL-361 | Donner son avis (« ce que je souhaite ») | ELV · `orientation.avis_eleve` | D. 331-32 [V R-B07] ; visible du PP et de la famille | `orientation.avis_eleve_recueilli` |
| BUL-362 | Devenir le demandeur à sa majorité | SYS | CB-20 | `orientation.demandeur_change` |

### 17.9 Orientation : propositions, décisions, acceptation
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| BUL-330 | Préparer l'avis provisoire ou la proposition en brouillon, prérempli depuis le vœu (NXF-09) | PP | — | — |
| BUL-331 | Arrêter l'avis provisoire (conseil du 2ᵉ trimestre) | PRES (BUL-142) | — | `orientation.avis_provisoire` |
| BUL-332 | Arrêter la proposition (conseil du 3ᵉ trimestre) | PRES (BUL-141) | — | `orientation.proposition` |
| BUL-333 | Publier l'avis ou la proposition aux familles | DIR · `orientation.decider` | publication réservée à la direction (NXF-09, garde serveur) | `orientation.avis_publie` |
| BUL-334 | Signer (direction ; PP si l'établissement l'exige) | DIR | S23-06 | — |
| BUL-340 | Décider, conforme à la demande (en lot possible) | DIR ★ | notification avec voies et délais | `orientation.decision` |
| BUL-341 | Proposer un entretien (créneaux) en cas de désaccord | DIR, PP | socle 19 | `orientation.entretien_propose` |
| BUL-342 | Tracer l'entretien (date, participants, compte rendu court) | DIR | — | `orientation.entretien_tenu` |
| BUL-343 | Décider de façon motivée (désaccord) | DIR · `orientation.decider` ★ | motivation obligatoire (R-B07) ; contrôle R-B13 | `orientation.decision` |
| BUL-344 | Accepter ou refuser la décision (par titulaire si exigé) | PAR, ELM | 3 jours ouvrables ; date limite affichée | `orientation.reponse` |
| BUL-345 | Constater l'acceptation réputée à l'échéance | SYS | [NV] Q-BU5 ; texte de la notification | `orientation.acceptee` |
| BUL-346 | Faire un avenant après acceptation (NXF-11) | DIR | motif ; nouvelle acceptation | `orientation.avenant` |
| BUL-347 | Enregistrer le choix des spécialités (famille) avec l'avis du conseil | SYS, ADF | D. 331-38 ; transmis aux socles 5 et 10 | `orientation.specialites_choisies` |

### 17.10 Appel et maintien
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| BUL-350 | Faire appel (dans le délai) | PAR, ELM | hors délai refusé sauf notification irrégulière (CB-23) | `orientation.appel_recu` |
| BUL-351 | Retirer son appel | PAR, ELM | CB-25 | `orientation.appel_retire` |
| BUL-352 | Compléter le dossier d'appel dans le temps (pièces attendues, reçues) (NXF-12) | ADF, DIR · `orientation.appels_gerer` | — | — |
| BUL-353 | Transmettre le dossier à la commission (lien nominatif, export) | ADF | socle 7 | `orientation.appel_transmis` |
| BUL-354 | Saisir la composition de la commission (privé) et contrôler les conflits d'intérêts | DIR | R-B08 | — |
| BUL-355 | Enregistrer la décision de la commission (maintien de la décision / réformation, destination) | ADF, DIR | définitive | `orientation.appel_decide` |
| BUL-356 | Clôturer l'appel (lecture seule) | DIR | NXF-12 | — |
| BUL-357 | Demander le maintien dans la classe d'origine (D. 331-37) | PAR, ELM | une seule année | `orientation.maintien_demande` |
| BUL-358 | Enregistrer le maintien | DIR | — | `orientation.definitive` |

### 17.11 Redoublement, passage, montée
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| BUL-370 | Demander un redoublement (famille) | PAR, ELM | hors palier compris | `orientation.redoublement_demande` |
| BUL-371 | Tracer le dialogue préalable avec l'élève et ses représentants | DIR, PP | obligatoire avant la décision (R-B09) | `orientation.dialogue_trace` |
| BUL-372 | Décider le redoublement (motivé) ou le rejeter | DIR ★ | avis du conseil requis ; compteur et accord du DASEN au collège (CB-28) ; voies et délais | `orientation.decision` |
| BUL-373 | Demander le dispositif d'accompagnement au socle 35 | SYS, DIR | obligatoire (R-B09) | `accompagnement.demande` |
| BUL-374 | Lister les élèves sans décision définitive (pour la montée) | DIR, ADF | port `passage::en_attente` | — |
| BUL-375 | Voir la décision définitive d'un élève | PP, DIR, ADF, PAR, ELV | — | — |
| BUL-376 | Alimenter la montée du socle 5 (STR-70) | SYS | §9.8 | `passage.decisions_pretes` |
| BUL-377 | Préparer la sortie d'un élève orienté ailleurs (socle 3) | SYS | destination externe | `orientation.depart_prevu` |
| BUL-378 | Reporter une décision dans le logiciel tiers (journal à reporter) | ADF | doublon (§9.9) | `echanges.a_reporter` |

### 17.12 LSL
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| BUL-400 | Saisir l'appréciation annuelle par enseignement | PRF · `lsl.saisir` | 1ʳᵉ, terminale, voie pro ; aide IA à la rédaction (relue, comme EVA-197 du 21) | `lsl.appreciation_saisie` |
| BUL-401 | Voir l'état du LSL de la classe (pré-contrôle) | PP, ADF, DIR | moyennes annuelles (R-V45), appréciations, avis | — |
| BUL-402 | Saisir les engagements et responsabilités | CPE, SYS (socle 46) | — | `lsl.engagement_saisi` |
| BUL-403 | Arrêter l'avis pour l'examen (BUL-144) | PRES | — | — |
| BUL-404 | Viser l'avis pour l'examen et le livret | chef · `lsl.viser` ★ | — | `lsl.vise` |
| BUL-405 | Produire l'aide à la saisie du LSL (export de travail) | ADF · `exports.officiels` | tant que D-2 n'est pas levée | `export.realise` |
| BUL-406 | Voir son avis pour l'examen et son explication | ELV, ELM | après visa [NV R-B12] | — |
| BUL-407 | Demander une explication sur l'avis (M-637) | ELV, ELM, PAR | tracée | `lsl.explication_demandee` |
| BUL-408 | Préparer la déclaration LSL officielle | ADF ★ | **après D-2** | `export.realise` |

### 17.13 Pilotage, échanges, reprise
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| BUL-420 | Voir le tableau des conseils (planifiés, préparés, tenus, bulletins publiés) | DIR, ADF | socle 47 | — |
| BUL-421 | Voir les indicateurs d'orientation (demandes, conformité, appels, issues), anonymes | DIR | aucun classement de professeurs | — |
| BUL-422 | Exporter un dossier d'orientation pour l'établissement d'accueil | ADF | demande vérifiée | — |
| BUL-423 | Exporter toutes les données d'un élève (droit d'accès) | DIR, DPO | RGPD art. 15 ; journalisé | `bulletins.acces_exporte` |
| BUL-424 | Reprendre les archives des fiches de dialogue de Next.js (PDF signés, horodatage, signataires) | SYS, ADF | « signée dans l'ancien outil » (T0 R-14) ; aucune donnée JSON reprise comme mémoire | `orientation.reprise_appliquee` |
| BUL-425 | Reprendre conseils, dossiers et bulletins de l'existant Rust (071, 181, 182) | SYS | §22 | `conseils.reprise_appliquee` |
| BUL-426 | Mode doublon : préparer et tenir un conseil sur les moyennes importées (non publié) | DIR | §12 | `conseils.doublon_tenu` |

---

## 18. Règles transverses (R-B1 et suivantes) [R]

| Id | Règle |
|---|---|
| R-B1 | Le conseil et le bulletin lisent l'instantané du 21 (version citée) et les synthèses du 22 ; aucun recalcul. |
| R-B2 | Un conseil `tenu` est verrouillé ; toute modification passe par une réouverture motivée ★, limitée dans le temps et journalisée. |
| R-B3 | Tout examen d'élève (appréciation du conseil, mentions, avis, propositions) est versionné en ajout seul. |
| R-B4 | Les mentions sont des données de l'établissement ; « avertissement » et « blâme » sont refusés (R-B10). |
| R-B5 | La vue séance n'affiche ni santé, ni aménagements, ni dispositifs, ni discipline détaillée, ni situation familiale, ni alertes, ni notes au président. |
| R-B6 | Un invité pour un cas individuel n'accède qu'au point de cet élève. |
| R-B7 | Le conseil **propose** ; le chef d'établissement **décide** orientation et redoublement ; aucune décision n'est produite par une règle ou par l'IA (R-V20, R-V40). |
| R-B8 | Une décision non conforme à la demande est précédée d'un entretien proposé et porte une motivation écrite. |
| R-B9 | Toute notification de décision d'orientation ou de redoublement porte la motivation (si requise), la voie d'appel, le délai en jours ouvrables, la date limite calculée et la voie contentieuse ; sinon elle ne part pas. |
| R-B10 | La demande est formulée par les titulaires de l'autorité parentale ou l'élève majeur ; chaque titulaire a sa réponse ; un accord tacite exige une information tracée du titulaire. |
| R-B11 | L'avis de l'élève mineur est recueilli et visible du conseil. |
| R-B12 | La montée (socle 5) ne fait passer un élève de palier qu'avec une décision définitive ; un redoublement décidé s'applique. |
| R-B13 | Un bulletin n'est généré qu'après la tenue du conseil, sauf choix explicite d'un bulletin sans rubrique du conseil. |
| R-B14 | Les destinataires d'un bulletin et d'une décision sont calculés à la publication (R-V47) ; un exemplaire nominatif par destinataire ; jamais l'adresse d'un autre. |
| R-B15 | Famille et élève lisent la **version publiée** (copie figée), jamais les tables vivantes. |
| R-B16 | Un bulletin publié ne change pas : correction = version 2 publiée à tous les destinataires de la v1. |
| R-B17 | Un seul bulletin publiable par (élève, période), quelle que soit son origine. |
| R-B18 | Un document de procédure (vœux, avis, décision, appel) est émis par le socle 7, immuable, versé au dossier scolaire. |
| R-B19 | Aucune sanction, aucune information financière, aucune rubrique « accompagnements » vers un tiers sur un document du 23 (R-V32, R-V33, R-V37). |
| R-B20 | Un parent ne voit jamais l'activité ni la réponse de l'autre parent au-delà de ce que la fiche commune montre aux deux (R-V46). |
| R-B21 | Un professeur ne voit, au conseil, que les élèves de ses cours, pendant la fenêtre de leur classe (D-022, 21 §14.3). |
| R-B22 | Un professeur n'exporte jamais (D-022). |

---

## 19. Événements (outbox, dans la transaction du fait)

**Émis** : `conseils.serie_creee`, `.planifie`, `.reporte`, `.annule`, `.convocation`, `.presidence_deleguee`, `.invitation`, `.dossier_pret`, `.seance_ouverte`, `.delegues_sortis`, `.examen_arrete`, `.examen_corrige`, `.tenu`, `.rouvert`, `.pv_produit`, `.synthese_validee`, `.mentions_maj`, `.doublon_tenu`, `.reprise_appliquee` ; `bulletins.trame_choisie`, `.generes`, `.controle_realise`, `.publication_programmee`, `.publies`, `.remis`, `.ouvert`, `.non_ouvert`, `.exemplaire_ajoute`, `.rectifie`, `.duplicata`, `.registre_constitue`, `.explication_demandee`, `.rectification_demandee`, `.demande_repondue`, `.acces_exporte` ; `orientation.campagne_creee`, `.fiches_generees`, `.etape_ouverte`, `.relance`, `.demande_deposee`, `.demande_confirmee`, `.conflit`, `.conflit_resolu`, `.accord_tacite`, `.avis_eleve_recueilli`, `.demandeur_change`, `.gel`, `.degel`, `.avis_provisoire`, `.proposition`, `.avis_publie`, `.entretien_propose`, `.entretien_tenu`, `.decision`, `.reponse`, `.acceptee`, `.avenant`, `.specialites_choisies`, `.appel_recu`, `.appel_retire`, `.appel_transmis`, `.appel_decide`, `.maintien_demande`, `.definitive`, `.redoublement_propose`, `.redoublement_demande`, `.dialogue_trace`, `.depart_prevu`, `.campagne_close`, `.reprise_appliquee` ; `passage.decisions_pretes` ; `lsl.appreciation_saisie`, `.engagement_saisi`, `.vise`, `.explication_demandee`. **Aucune** charge utile ne porte une mention, une décision, une destination ni un texte (identifiants seulement, R-V17).
**Consommés** : `evaluations.notes_arretees`, `.appreciations_closes`, `.corrigee`, `.periode_rouverte` (21) → dossier, fenêtre, v2 ; `competences.syntheses_validees`, `.bilan_vise` (22) ; `absences.*` (12) → assiduité ; `liens.*`, `eleve.majeur` (2, 4) → destinataires, demandeur ; `eleve.sorti`, `eleve.decede`, `eleve.fusionne` (2) ; `services.*`, `missions.pp` (6) → participants ; `elections.mandat_*` (46) → délégués ; `referentiel.offre_*` (5) → destinations internes ; `agenda.creneau_reserve` (19) → entretien.

---

## 20. Écrans clés

| Écran | Pour qui | Contenu | Support |
|---|---|---|---|
| **Calendrier des conseils** | ADF, DIR | série, classes, dates, présidents, conflits | web |
| **Préparation du conseil** | PP | tuiles (classe, à discuter, ce qui a changé, manques) ; liste d'élèves avec propositions | web, tablette |
| **Séance** | PRES, SEC | élève projeté (vue séance), panneau du président (propositions, mentions, décision en un geste, délégués sortis), notes du PV | **tablette**, écran projeté |
| **Dossier d'un élève (vue complète)** | PP, DIR | instantané, synthèses, fiche de dialogue, historique | web |
| **PV** | PRES | brouillon, validation | web |
| **Bulletins de la classe** | ADF, DIR | génération, aperçu, contrôle (rapport), publication programmée, lot d'impression, non ouverts | web |
| **Registre et archives** | ADF | recherche, duplicata | web |
| **Campagne d'orientation** | ADF, PP, DIR | état par fiche, « prochaine action », filtres, gel, clôture | web |
| **Décisions** | DIR | demandes, propositions, conformes en lot, désaccords (entretien, motivation), délais | web |
| **Appels** | DIR, ADF | dossiers, pièces, commission, issue | web |
| **Ma fiche d'orientation** | PAR, ELM, ELV (lecture et avis) | étapes, vœux, échéance, avis, décision, voies et délais, accusé | **mobile**, web |
| **Mes bulletins** | PAR, ELV | exemplaires, glossaire, demande d'explication | **mobile**, web |

---
## 21. Modèle de données cible (briques `conseils`, `bulletins`, `orientation`)

Mêmes règles que `22_COMPETENCES.md` §21 sur **chaque** table : UUID applicatif (E3) ; `etablissement_id NOT NULL` partout, tables filles comprises (E1) ; `UNIQUE (etablissement_id, id)` et FK **composées** (E2) ; `cree_le` / `modifie_le` (E6) ; **aucun JSON/JSONB nouveau** (E10) ; **aucun BYTEA** : documents émis par le socle 7, pièces dans `ent_objet` (E5) ; RLS forcée par `ent_rls_poser` (E13) ; classement au registre (E12) ; journal `ent_journal_modif` pour décisions, réouvertures, corrections, délégations, publications (E7) ; aucun nom d'élève dans un nom d'objet (E14).

```
-- Conseils
cns_mention_type              id, etablissement_id, code, libelle, nature ('distinction','mise_en_garde'), imprimee BOOLEAN,
                              ordre, regle_moyenne_min NUMERIC(5,2) NULL, regle_aucune_sous NUMERIC(5,2) NULL,
                              regle_absences_nj_max SMALLINT NULL, archive_le NULL, cree_le, modifie_le
                              CHECK (libelle !~* 'avertissement|bl[aâ]me')     -- R-B4
cns_serie                     id, etablissement_id, annee_id, periode_ref_id, niveau_id NULL, libelle, cree_par, cree_le
cns_conseil                   id, etablissement_id, serie_id NULL, classe_id, periode_id, debut TIMESTAMPTZ, fin TIMESTAMPTZ NULL,
                              salle_id NULL, president_personne_id, delegation_id NULL, secretaire_personne_id NULL,
                              etat ('planifie','prepare','en_seance','tenu','reouvert','clos','annule'),
                              instantane_id NULL, instantane_version INT NULL, mode_doublon BOOLEAN DEFAULT false,
                              tenu_le NULL, rouvert_jusqu_au NULL, motif_etat_code NULL, motif_etat_texte NULL, version, cree_le, modifie_le
                              UNIQUE (etablissement_id, classe_id, periode_id) WHERE etat <> 'annule'
cns_conseil_transition        ajout seul : de, vers, par, le, motif (machine d'états par déclencheur)
cns_delegation                id, etablissement_id, delegant_personne_id, delegue_personne_id, serie_id NULL, conseil_id NULL, du, au, motif, cree_le
cns_participant               id, etablissement_id, conseil_id, personne_id, role ('president','pp','professeur','cpe','psyen',
                              'delegue_parents','delegue_eleves','invite_cas','secretaire'), eleve_cas_id NULL (invite_cas),
                              presence ('convoque','present','excuse','absent','distance'), emarge_le NULL
cns_avis_professeur           id, etablissement_id, conseil_id, eleve_id, auteur_personne_id, nature ('avis','note_president'),
                              texte VARCHAR(600), cree_le                              -- interne ; purge fin d'année + 1 an
cns_marquage                  etablissement_id, conseil_id, eleve_id, auteur_personne_id, motif_code, cree_le
cns_examen                    id, etablissement_id, conseil_id, eleve_id, etat ('brouillon','arrete'), appreciation_conseil VARCHAR(800) NULL,
                              avis_examen ('tres_favorable','favorable','doit_faire_ses_preuves') NULL,
                              proposition_nature ('passage','redoublement','orientation','aucune') NULL, proposition_destination_id NULL,
                              delegues_sortis BOOLEAN DEFAULT false, version INT, arrete_par NULL, arrete_le NULL, cree_le, modifie_le
                              UNIQUE (etablissement_id, conseil_id, eleve_id)
cns_examen_mention            etablissement_id, examen_id, mention_type_id, proposee_par_regle BOOLEAN       -- 0..n mentions
cns_examen_avis_voeu          etablissement_id, examen_id, voeu_id, avis ('favorable','reserve','defavorable')
cns_examen_avis_specialite    etablissement_id, examen_id, voeu_specialite_id, avis ('favorable','reserve','defavorable')
cns_examen_historique         ajout seul : version, appréciation, avis, proposition, mentions (une ligne par mention dans
                              cns_examen_historique_mention), auteur, motif
cns_pv                        id, etablissement_id, conseil_id, texte_synthese VARCHAR(4000), interventions_delegues VARCHAR(4000),
                              etat ('brouillon','valide'), valide_par NULL, valide_le NULL, document_id NULL (socle 7), version
cns_synthese_delegues         id, etablissement_id, conseil_id, texte VARCHAR(2000), valide_par NULL, valide_le NULL

-- Bulletins (succède à ent_bulletin 182, gardé et étendu)
bul_trame_choix               etablissement_id, annee_id, niveau_id, trame ('college','lycee_gt','lycee_pro','sans_note'), version
bul_lot                       id, etablissement_id, classe_id, periode_id, conseil_id NULL, etat ('genere','controle','programme','publie'),
                              publication_prevue_le NULL, remise_main_propre_le NULL, publie_le NULL, publie_par NULL, cree_le
bul_bulletin                  -- = ent_bulletin (182) étendu : origine ('scolia','depose_tiers','reprise'), lot_id, import_ligne_id NULL (21 V14),
                              instantane_id, instantane_version, examen_version NULL, sans_rubrique_conseil BOOLEAN,
                              etat ('genere','publie','remplace','annule'), version, empreinte, document_id (socle 7),
                              contenu JSONB (exception D-036 déjà déclarée : copie figée d'un bulletin publié, jamais relue comme source)
                              UNIQUE (etablissement_id, eleve_id, periode_id) WHERE etat = 'publie'         -- R-B17
bul_controle                  id, etablissement_id, lot_id, lance_par, lance_le, nb_bloquants, nb_avertissements
bul_controle_ligne            id, etablissement_id, controle_id, eleve_id NULL, gravite ('bloquant','avertissement'), code, cree_le
bul_exemplaire                id, etablissement_id, bulletin_id, personne_id, lien_id NULL, canal ('coffre','courrier','main_propre'),
                              document_id (exemplaire nominatif, socle 7), mis_a_disposition_le, premiere_ouverture_le NULL,
                              distribue BOOLEAN, motif_non_distribue_code NULL, remis_par NULL
bul_duplicata                 id, etablissement_id, bulletin_id, demandeur_personne_id NULL, demandeur_libelle NULL, motif_code,
                              document_id, delivre_par, delivre_le
bul_registre                  id, etablissement_id, annee_id, classe_id, document_id, constitue_le
bul_demande                   id, etablissement_id, bulletin_id, demandeur_personne_id, nature ('explication','rectification'),
                              message VARCHAR(1000), reponse VARCHAR(1500) NULL, etat ('ouverte','repondue','close'), repondu_par NULL

-- Orientation et passage (remplace ent_orientation_avis 181 et le module Next.js M10)
orn_destination               id, etablissement_id, code, libelle, nature ('niveau','voie','serie','formation','maintien','autre'),
                              interne BOOLEAN, offre_id NULL (socle 5), niveau_cible_id NULL, ordre, archive_le NULL
orn_campagne                  id, etablissement_id, annee_id, niveau_id, gabarit_code, mode ('famille_dabord','conseil_dabord'),
                              etat ('brouillon','active','gelee','close','archivee'), close_le NULL, cree_par, cree_le
orn_campagne_destination      etablissement_id, campagne_id, destination_id, par_defaut BOOLEAN
orn_etape                     id, etablissement_id, campagne_id, nature ('demande_provisoire','avis_provisoire','demande_definitive',
                              'proposition','decision','acceptation','appel'), ordre, ouvre_le, ferme_le
orn_procedure                 id, etablissement_id, eleve_id, annee_id, campagne_id NULL, palier BOOLEAN,
                              etat ('ouverte','conflit','gelee','proposition','entretien','decision','acceptee','appel','definitive','close'),
                              demandeur ('titulaires','eleve_majeur'), version, cree_le, modifie_le
                              UNIQUE (etablissement_id, eleve_id, annee_id)
orn_demande                   id, etablissement_id, procedure_id, etape_id, deposant_personne_id, mode ('en_ligne','papier'),
                              commentaire VARCHAR(1500) NULL, statut ('deposee','confirmee','contredite','tacite','retenue','ecartee'),
                              depose_le, version
orn_voeu                      id, etablissement_id, demande_id, rang SMALLINT, destination_id, precision VARCHAR(200) NULL
orn_voeu_specialite           id, etablissement_id, voeu_id, matiere_id, abandon BOOLEAN DEFAULT false
orn_confirmation              id, etablissement_id, demande_id, titulaire_personne_id, reponse ('confirme','contredit','tacite'),
                              motif VARCHAR(600) NULL, information_tracee_le NULL, le
orn_conflit_resolution        id, etablissement_id, procedure_id, issue ('maintien_famille','avis_conseil'), motif, par, le
orn_avis_eleve                id, etablissement_id, procedure_id, texte VARCHAR(1500), le, version
orn_proposition               id, etablissement_id, procedure_id, examen_id (cns_examen), nature ('provisoire','definitive'),
                              destination_id NULL, nature_passage ('passage','redoublement','orientation'), publie_le NULL, publie_par NULL
orn_entretien                 id, etablissement_id, procedure_id, propose_le, tenu_le NULL, participants_texte VARCHAR(400),
                              compte_rendu VARCHAR(1500) NULL, rendez_vous_id NULL (socle 19)
orn_decision                  id, etablissement_id, procedure_id, nature ('passage','redoublement','orientation','maintien'),
                              destination_id NULL, conforme BOOLEAN, motivation VARCHAR(2000) NULL, decide_par, decide_le,
                              notifie_le NULL, date_limite_appel DATE NULL, document_id NULL, version
                              CHECK (conforme OR motivation IS NOT NULL)                             -- R-B8
orn_reponse                   id, etablissement_id, decision_id, titulaire_personne_id NULL, reponse ('accepte','refuse','reputee_acceptee'), le
orn_avenant                   id, etablissement_id, procedure_id, decision_precedente_id, motif, par, le
orn_appel                     id, etablissement_id, procedure_id, decision_id, depose_par_personne_id, depose_le, hors_delai BOOLEAN,
                              etat ('ouvert','transmis','decide','retire','clos'), commission_libelle, seance_le NULL,
                              issue ('maintien_decision','reformation') NULL, destination_id NULL, decide_le NULL, document_id NULL
orn_appel_piece               id, etablissement_id, appel_id, libelle, attendue BOOLEAN, recue_le NULL, objet_id NULL (ent_objet)
orn_commission_membre         id, etablissement_id, appel_id, personne_libelle, qualite ('chef_etablissement','professeur','parent','autre'),
                              conflit_declare BOOLEAN                                              -- R-B08 (privé)
orn_redoublement_dialogue     id, etablissement_id, procedure_id, le, participants_texte VARCHAR(400), compte_rendu VARCHAR(1500)
orn_specialite_choix          id, etablissement_id, procedure_id, matiere_id, choisie BOOLEAN, avis_conseil ('favorable','reserve','defavorable') NULL
orn_document                  etablissement_id, procedure_id, nature ('voeux','avis','proposition','decision','acceptation','appel','decision_appel'),
                              document_id (socle 7), cree_le

-- LSL
lsl_appreciation_annuelle     id, etablissement_id, eleve_id, annee_id, cours_id, matiere_id, texte VARCHAR(600), etat ('brouillon','validee'),
                              aide_ia BOOLEAN, auteur_personne_id, version, cree_le, modifie_le
lsl_appreciation_historique   ajout seul
lsl_engagement                id, etablissement_id, eleve_id, annee_id, nature_code, libelle, source ('mandat','saisie'), mandat_id NULL (46)
lsl_visa                      id, etablissement_id, eleve_id, annee_id, avis_examen, examen_id, vise_par, vise_le, version
```
**Attente posée au 21 (C2 du 22, *appliquée par défaut*)** : l'instantané d'arrêt est **normalisé** — `eva_instantane` garde version, empreinte, motif ; `eva_instantane_ligne (etablissement_id, instantane_id, eleve_id, cours_id NULL, matiere_id NULL, nature ('moyenne_cours','moyenne_matiere','moyenne_generale','statut','nb_evaluations','nb_non_rendus','appreciation','synthese_competence','assiduite'), valeur NUMERIC NULL, statut_code NULL, appreciation_id NULL, appreciation_version INT NULL, synthese_id NULL)` — au lieu de `contenu JSONB`, relu comme source par le conseil (D-036). **Exceptions D-036** : `bul_bulletin.contenu` (= `ent_bulletin.contenu`, déjà déclarée). **Files d'effets de bord** : aucune nouvelle (avis par l'outbox et la file du socle 8, liens nominatifs par le socle 7, déjà classés E9). **Next.js** : `fd_reponse` (JSON), `fd_pdf_archive.pdf_base64`, `fd_token` **ne sont pas repris** comme données vivantes : seules les **archives** (PDF signés, horodatage, signataires) sont versées au dossier (BUL-424, T0 R-14).

---

## 22. Plan de migration depuis l'existant

| Étape | Contenu | Compatibilité | Contrôle |
|---|---|---|---|
| M1 (B0) | journal en ajout seul de `ent_conseil_classe_dossier` (déclencheur) ; refus d'écriture si `tenu` ; commande « tenir » sur la voie EVA ; génération refusée si le conseil n'est pas tenu ; lecture famille depuis la version publiée ; `ON DELETE RESTRICT` sur `ent_orientation_avis` ; décider réservé à la présidence ; mobiles : `/eleves/{id}/conseils` lit les décisions **du bulletin publié** | voies historiques en lecture | tests de fuite et de verrou |
| M2 (B1, B4, B8) | tables `cns_*`, `orn_*`, `lsl_*` ; reprise : `ent_conseil_classe` → `cns_conseil` (état repris ; `planifie` → `planifie`, `tenu` → `tenu`) ; dossiers → `cns_examen` (version 1, mention reprise en `cns_examen_mention`, `decision_code` → proposition **pas** décision, avec un rapport des « décisions à confirmer ») ; `ent_orientation_avis` → `cns_examen_avis_voeu` | vues de compatibilité en lecture | comptes identiques ; rapport des décisions à confirmer |
| M3 (B5, B6) | `ent_bulletin` étendu en place (colonnes ajoutées, déclencheur d'immuabilité conservé) | aucune rupture | immuabilité testée |
| M4 (B14) | archives Next.js (PDF signés) versées par lot depuis un export fourni par Florian (anonymisé pour les essais, D-035 point 4) | — | une archive par fiche close, empreinte |
| M5 (B15) | RLS stricte ; retrait des vues et anciennes tables (071, 181) après la durée d'archive | — | aucune lecture des vues 30 jours |

---

## 23. Lots de code (équipe Sonnet)

### 23.1 Tableau des lots
| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| **B0** correctifs urgents | BD1 (journal + verrou), BD2 (génération après `tenu`), BD3 (lecture de la version publiée), BD4 (commande et bouton « tenir »), BD5 (`RESTRICT`), BD6 (décider = présidence), BD9 (mobiles lisent le bulletin publié ; UI JS : message « fonction déplacée ») | rien ; tout de suite | tests : modification refusée après `tenu` ; ancienne valeur au journal ; bulletin non générable sur un conseil `planifie` ; famille lit la v publiée après réouverture ; suppression d'un vœu refusée ; professeur non PP ne décide pas ; appli famille voit la décision publiée |
| **B1** conseils : modèle et planification | `cns_mention_type`, `cns_serie`, `cns_conseil` + transitions, délégation, participants, convocations, calendrier, fenêtre (port) | B0 ; 21 V8 (instantané) ; socles 6, 46 (délégués, à défaut saisie) | machine d'états en base ; contrôles §7.2 |
| **B2** préparation | dossier en un clic, à discuter, ce qui a changé, propositions en brouillon, notes au président, avis, invitations | B1 ; 22 C3 | A7 mesuré : dossier d'une classe de 30 en moins de 2 s ; vue séance sans aucune donnée exclue (test) |
| **B3** séance | tablette, vue projetée, délégués sortis, arrêt par élève et en lot, bilans du 22, hors ligne, clôture | B2 | parcours complet d'un conseil en recette ; R-B2, R-B5, R-B6 |
| **B4** examen, mentions, PV | versions, mentions multiples, avis pour l'examen, PV, synthèse des délégués, réouverture, corrections | B3 | historique complet ; « avertissement » refusé |
| **B5** trames et rendu | 4 trames Typst (socle 7), PDF/A et PDF/UA, prénom d'usage, glossaire | socle 7 D1 | accents justes (« Nguyễn ») ; axe-core sur la vue en ligne |
| **B6** publication et distribution | génération, contrôle, publication programmée, exemplaires, coffre, courrier, main propre, non ouverts, v2 | B4, B5 ; socles 7, 8 ; socle 4 (`recevoir_bulletins`) | R-B13 à R-B17, R-V47 en tests ; matrice famille complète |
| **B7** secrétariat (= V15 du 21) | lot d'impression, duplicata, archives, registre, dossier de transfert, guichet | B6 | EVA-244 à 255 ; R-V39 |
| **B8** orientation (reprise de Next.js) | destinations, campagnes, étapes, procédures, demandes par titulaire, confirmation, accord tacite, conflit, avis de l'élève, gel, propositions, entretien, décision motivée, délais, acceptation, avenant, spécialités, documents | B3 ; socles 4, 7, 8, 19, 5 (jours ouvrables) | **les 15 scénarios du rapport Next.js** (S1 à S15) rejoués en tests automatiques, plus NXFD-1 à NXFD-12 corrigés ; R-B7 à R-B11 |
| **B9** appel et maintien | dossier d'appel, pièces, transmission, commission (public, privé), issue, maintien, redoublement (dialogue, décision, accompagnement, compteur) | B8 ; socle 35 (demande d'accompagnement) | CB-22 à CB-28 ; R-B9 en test bloquant |
| **B10** branchement montée | port `passage::decision`, `en_attente` ; socle 5 STR-70 ; sortie vers le socle 3 ; colonnes orphelines alimentées | B8, B9 ; socle 5 | un redoublant ne monte pas ; un élève de palier sans décision bloque sa montée |
| **B11** LSL | appréciations annuelles, engagements, avis pour l'examen, visa, aide à la saisie ; déclaration officielle **après D-2** | B4 ; 22 C7 ; 46 | pré-contrôle ; aucun professeur n'exporte |
| **B12** mobile | « ma fiche d'orientation », « mes bulletins », séance sur tablette | B3, B6, B8 ; socle 11 | parité des droits |
| **B13** IA | relecture de lot, brouillon de PV, synthèses, explication d'une décision, traduction | B4, B6 ; cœur IA ; étude juridique | refus testés (§14) |
| **B14** reprises | existant Rust (071, 181, 182) ; archives des fiches Next.js | B8 ; export fourni par Florian | rapport d'écarts ; une archive par fiche |
| **B15** RLS stricte et retraits | RLS stricte, retrait des vues et des tables 071, 181 | B14 ; 21 V13 | inventaire RLS |

**Ordre recommandé (appliqué par défaut, D-031)** : **B0 tout de suite** (verrou, décisions provisoires imprimées, cascade qui efface des preuves, applis aveugles) ; puis B1 → B2 → B3 → B4 → B5 → B6 ; **B8 → B9 → B10 en priorité de l'épaississement** (février – juin 2027) : la fiche de dialogue est un module de l'add-on en usage réel, à niveau **production** pour la campagne 2027-2028 (D-035 point 2) ; B7 avec le premier bulletin produit par ScolIA ; B11, B12, B13, B14 ensuite ; B15 en dernier. **Démo de décembre (D-034, « notes et bulletins »)** : B0 + écrans Leptos existants (conseil, bulletins) avec le bouton « tenir ». **V1 (fin janvier – février 2027)** : B0 à B6 et B8 en version essentielle (sans appel complet).

### 23.2 Migrations (plage imposée **2460–2479**, registre `MIGRATIONS.md`)
| N° | Lot | Contenu |
|---|---|---|
| **2460** | B0 | `ent_conseil_classe_dossier_journal` (ajout seul) et déclencheurs (journal ; refus si `tenu`) ; `ent_orientation_avis` → `ON DELETE RESTRICT` |
| **2461** | B1 | `cns_mention_type` (préréglage), `cns_serie`, `cns_conseil`, `cns_conseil_transition`, `cns_delegation`, `cns_participant` |
| **2462** | B2 | `cns_avis_professeur`, `cns_marquage` |
| **2463** | B3–B4 | `cns_examen`, `cns_examen_mention`, `cns_examen_avis_voeu`, `cns_examen_avis_specialite`, `cns_examen_historique`, `cns_examen_historique_mention` |
| **2464** | B4 | `cns_pv`, `cns_synthese_delegues` |
| **2465** | B6 | `bul_trame_choix`, `bul_lot`, extension de `ent_bulletin` (origine, lot, instantané, examen, `sans_rubrique_conseil`, état `annule`), index unique R-B17 |
| **2466** | B6 | `bul_controle`, `bul_controle_ligne`, `bul_exemplaire` |
| **2467** | B7 | `bul_duplicata`, `bul_registre`, `bul_demande` |
| **2468** | B8 | `orn_destination`, `orn_campagne`, `orn_campagne_destination`, `orn_etape` |
| **2469** | B8 | `orn_procedure`, `orn_demande`, `orn_voeu`, `orn_voeu_specialite`, `orn_confirmation`, `orn_conflit_resolution`, `orn_avis_eleve` |
| **2470** | B8 | `orn_proposition`, `orn_entretien`, `orn_decision`, `orn_reponse`, `orn_avenant`, `orn_specialite_choix`, `orn_document` |
| **2471** | B9 | `orn_appel`, `orn_appel_piece`, `orn_commission_membre`, `orn_redoublement_dialogue` |
| **2472** | B11 | `lsl_appreciation_annuelle`, `lsl_appreciation_historique`, `lsl_engagement`, `lsl_visa` |
| **2473** | B14 | reprise 071/181 → `cns_*` ; vues de compatibilité |
| 2474–2476 | réserve | passe personas et lots suivants |
| **2477** | B15 | RLS stricte (contrôle des tables `cns_*`, `bul_*`, `orn_*`, `lsl_*`) |
| **2478–2479** | B15 | retrait des vues puis des tables 071 et 181 après la durée d'archive (destructrice : décision, sauvegarde vérifiée, note, E8) |
Migrations additives et rejouables (E8) ; ordre croissant dans `migrate.rs` ; `MIGRATION_TAG` = la plus haute.

---

## 24. Tests et recette

- **Machine d'états** du conseil et de la procédure (toutes transitions interdites testées).
- **Scénarios Next.js rejoués** (S1 à S15 du rapport du 21/10/2026) + NXFD-1 à NXFD-12.
- **Délais** : calcul des 3 jours ouvrables (vacances, fériés, samedi) ; notification sans mentions refusée (R-B9).
- **Matrice de séparation** (§15.3) générée depuis les routes (D-026), avec les capacités IA.
- **Recette par personas** (D-021) : délégué qui tente le dossier d'un camarade ; professeur de groupe qui tente l'élève d'un autre groupe ; parent sans exercice qui dépose des vœux ; parent interdit qui tente le bulletin par trois chemins ; second titulaire non informé et accord tacite ; président non délégué qui tente de tenir ; IA à qui l'on demande une mention ; établissement B.

---

## 25. Réglages signalés « usine à gaz » (D-023)

| Id | Réglage | Recommandation |
|---|---|---|
| S23-01 | Règles de proposition des mentions (seuils) | facultatives, 3 critères seulement |
| S23-02 | Deux textes « appréciation générale » et « du conseil » | un seul par défaut (S-121 du 21) |
| S23-03 | Avertissement d'horaire incompatible avec les délégués des parents | avant 17 h, désactivable |
| S23-04 | Durée de réouverture d'un conseil | 48 h par défaut |
| S23-05 | Délai des non ouverts avant relance | J+7 |
| S23-06 | Signature du PP sur les documents d'orientation | non exigée |
| S23-07 | Acceptation de la décision par chaque titulaire | un suffit, l'autre informé (Q-BU6) |
| S23-08 | Mode doublon des conseils | désactivé, ouvert par la direction |

---

## 26. Questions, décisions appliquées par défaut (D-031) et points pour Florian

### 26.1 Questions tranchées selon la recommandation (*appliqué par défaut, D-031*)
| Id | Question | Recommandation appliquée |
|---|---|---|
| Q-BU1 | Qui possède la fiche de dialogue : 23 (matrice NOT-17, 21 §6.2) ou 37 (T0 l. 207) ? | **Socle 23** (circuit de décision D. 331) ; le 37 possède parcours Avenir, entretiens, Affelnet, Parcoursup et **lit** les vœux (C3, §27) |
| Q-BU2 | Verrou à la tenue du conseil ? | **Oui**, réouverture motivée ★ 48 h (R-B2) |
| Q-BU3 | Plusieurs mentions par élève ? | **Oui** (distinction et mise en garde de comportement possibles ensemble) |
| Q-BU4 | Bulletin avant la tenue du conseil ? | **Non**, sauf choix explicite « sans rubrique du conseil » (CB-31) |
| Q-BU5 | Silence de la famille dans les 3 jours ouvrables ; définition des jours ouvrables | **Acceptation réputée**, écrite en toutes lettres dans la notification ; jours ouvrables = calendrier de l'établissement hors dimanches et fériés **[NV : à faire confirmer par un juriste]** |
| Q-BU6 | Acceptation de la décision par chaque titulaire ? | **Un titulaire suffit**, l'autre informé ; réglable (S23-07) |
| Q-BU7 | Parent sans exercice : dépose-t-il des vœux ? | **Non**, il est informé (R-B15) **[H juridique]** |
| Q-BU8 | Signature du PP ? | **Non exigée** (S23-06) |
| Q-BU9 | Avis de l'élève mineur | **Recueilli** par un écran dédié (BUL-361), visible du conseil (R-B11) |
| Q-BU10 | Conseils en mode doublon l'an 1 | **Un ou deux par trimestre**, sur moyennes importées, non publiés (§12) |
| Q-BU11 | Instantané JSON du 21 relu comme source | **Normalisé** en lignes (§21, attente au 21) |
| Q-BU12 | Conservation des PV et des dossiers d'appel | PV 5 ans ; dossier d'appel 2 ans puis décision au dossier — **à valider DPO et archiviste** |
| Q-BU13 | Envoi recommandé des décisions non conformes aux familles sans compte | **Courrier simple** + coffre par défaut ; recommandé si la direction le décide [H : coût, Q pour Florian F23-4] |

### 26.2 Ce qui engage Florian (non appliqué)
| Id | Point | Pourquoi Florian |
|---|---|---|
| F23-1 | **Démarche éditeur (F-06)** pour le LSL officiel (B11) | démarche officielle, D-032 |
| F23-2 | **Export des archives** de fiches de dialogue de Next.js (anonymisé pour les essais, réel après l'accord de l'établissement et l'avis du DPO) | données réelles (D-035 point 4) |
| F23-3 | **Comment les décisions de Next.js sont reportées aujourd'hui dans Charlemagne** (saisie manuelle ? import ?) : conditionne le journal à reporter de l'an 1 | connaissance du terrain |
| F23-4 | **Recommandés postaux** pour les décisions défavorables (coût) | coût |
| F23-5 | **Quand basculer la fiche de dialogue** de Next.js vers ScolIA (campagne 2027-2028 entière dans ScolIA, ou fin dans Next.js) ; arrêt du module Next.js | décision et action de Florian (D-035 point 3) |
| F23-6 | **Avis d'un juriste** sur l'acceptation réputée et les jours ouvrables (Q-BU5), et sur la diffusion du LSL à l'élève | juriste |
| F23-7 | **Durées de conservation** (§15.4) | DPO, archiviste |

---

## 27. Faits, hypothèses, contradictions

**Faits [F]** : §2.1 et §2.2 (audit du sous-agent, 2026-11-11 ; constats relus dans son rapport avec chemins et lignes) ; module Next.js d'après `TRANSITION_T0_INVENTAIRE.md`, `rapport-fiches-dialogue-personas-v2.md` et une lecture ciblée des états et gabarits (`app/lib/fiches-dialogue-workflow.ts`, `fiches-dialogue-templates.ts`).
**Vérifié [V]** le 2026-11-11 : R-B01 à R-B17 (§5), avec leurs réserves.
**Hypothèses [H]** : calendrier type (§9.1) ; usage de Charlemagne pour les décisions (§12) ; ordre des écrans du LSL.
**Contradictions relevées** :
- **C1** `21_EVALUATIONS.md` §6.2 place la **fiche de dialogue** au socle 23 (avec le 19) ; `MATRICE_CHARLEMAGNE.md` NOT-17 aussi (23, 19) ; `TRANSITION_T0_INVENTAIRE.md` l. 207 la range au **socle 37**. **Tranché** (Q-BU1) : 23 possède le circuit et la fiche, 37 lit ; à reporter dans T0 et dans la conception du 37.
- **C2** `21_EVALUATIONS.md` §21 : `eva_instantane.contenu JSONB` est l'entrée du conseil, donc relu comme source, contre D-036 : **tranché** par la normalisation (§21) ; à reporter dans le 21 (lot V1 ou V8).
- **C3** `21_EVALUATIONS.md` §12.1 attribue l'**appréciation du conseil** au 23 tandis que S-121 la fusionne avec la générale (21) : **cohérent** ici (un texte par défaut, 21 ; table du 23 seulement si l'établissement sépare).
- **C4** R-34 du 21 tenait la discrétion des délégués pour [NV] : **levé** par D. 111-11 [V R-B03] (« obligation de confidentialité »).
- **C5** `SOCLES.md` §23 mentionne « LSL [H] » : vérifié (R-B12) ; et « recours » : ce sont les **appels** D. 331 (R-B07 à R-B09) et la **contestation** non juridique d'une appréciation (BUL-260 à 262).
- **C6** Le rapport Next.js écrit « l'élève ne décide pas et ne signe pas » : exact pour la décision, mais D. 331-32 exige que **l'avis** de l'élève mineur soit **recueilli** (NXFD-9).
- **C7** `05_STRUCTURE.md` §(l. 986) attend « décision de passage (socle 23, nom à fixer) » : nom fixé, port `passage::decision`.

---

## 28. [ajout personas] Ce que change la passe personas (`PERSONAS_23.md`, 2026-11-11)

Dix-neuf personas, **88 manques** (M23-001 à M23-088 : 5 bloquants en trois points, 58 importants, 25 confort). **Ce paragraphe l'emporte** sur le texte qui précède en cas de contradiction ; rien n'est supprimé.

### 28.1 Bloquants tranchés (*appliqué par défaut, D-031*)
| # | Point | Décision appliquée |
|---|---|---|
| **A** | Quand une décision d'orientation ou de redoublement est-elle **notifiée** ? (`orn_decision.notifie_le` unique, acceptation réputée Q-BU5) | **Notification par titulaire** : elle vaut à la première **ouverture authentifiée** du document, à la **remise en main propre émargée**, ou à l'**accusé** d'un recommandé ; le dépôt au coffre et le courrier simple ne font **pas** courir le délai ; date limite **par titulaire** ; **aucune acceptation réputée** sans notification valable ; tâche bloquante au secrétariat à J+5 (R-B23, R-B24). Remplace la note « courrier simple par défaut » de Q-BU13 pour les décisions non conformes : sans compte, **remise en main propre** ou **recommandé**. |
| **B** | Un titulaire accepte, l'autre veut faire appel (S23-07, Q-BU6) | **Tout titulaire peut faire appel dans son délai** ; une décision n'est **définitive** qu'à l'expiration des délais de chaque titulaire notifié sans appel ; l'acceptation **expresse** d'un titulaire permet seulement la **montée provisoire** (constitution des classes) ; un appel suspend l'acceptation de l'autre, qui en est informé ; seul l'auteur retire son appel (R-B25, R-B26). |
| **C** | Actes engageants par lien nominatif sans compte (BUL-306, BUL-314) | Dépôt et correction de vœux, confirmation ou contradiction, acceptation, refus, appel, retrait d'appel, maintien : **session authentifiée** du titulaire ; **passkey** (D-009) et **réauthentification de moins de 5 minutes** pour accepter, faire appel ou retirer un appel (`ent_session.authentification_forte_le`, D-041) ; sans compte : **papier signé remis au secrétariat** (BUL-314, pièce obligatoire) ou entretien ; le lien nominatif **ne sert qu'à lire** (R-B27). |

### 28.2 Actions ajoutées
| Id | Action | Qui · droit | Règles | Événement |
|---|---|---|---|---|
| BUL-430 | Enregistrer la notification valable d'un titulaire (ouverture authentifiée, main propre émargée, accusé de recommandé) | SYS, ADF | R-B23 | `orientation.notifie` |
| BUL-431 | Voir, par décision, l'état de notification de chaque titulaire et sa date limite | DIR, ADF | R-B24 | — |
| BUL-432 | Recevoir la tâche « notification non valable à J+5 » et choisir la voie (main propre, recommandé, entretien) | ADF | S23-12 | `orientation.notification_a_refaire` |
| BUL-433 | Remettre une décision en main propre contre émargement (titulaire, jamais l'élève mineur seul) | ADF, DIR | M23-064 | `orientation.notifie` |
| BUL-434 | Constater la fin des délais de tous les titulaires et rendre la décision définitive | SYS | R-B25 | `orientation.definitive` |
| BUL-435 | Informer l'autre titulaire d'un appel formé | SYS | R-B25 | `orientation.appel_recu` |
| BUL-436 | Exiger la réauthentification avant acceptation, appel, retrait d'appel | SYS | R-B27 | — |
| BUL-437 | Lire une fiche par lien nominatif (lecture seule) | titulaire sans compte | R-B27 | — |
| BUL-438 | Rapport d'écart du mode doublon (mentions, propositions ScolIA / Charlemagne) | DIR | §12 ; sans nominatif hors direction | — |
| BUL-439 | Pré-contrôle avant décision en lot (avis de l'élève, confirmation du second titulaire, conflit) | DIR | M23-002 | — |
| BUL-440 | Déléguer la signature des décisions d'orientation (datée) | chef ★ | M23-003 | `orientation.delegation` |
| BUL-441 | Tableau des décisions non conformes, délais, appels | DIR, ADF | — | — |
| BUL-442 | Paramétrer la commission d'appel (privé : nom, séances) et convoquer la famille | DIR, ADF | R-B08 | `orientation.convocation_commission` |
| BUL-443 | Marquer une décision « reportée dans le logiciel tiers le … par … » | ADF | §9.9 | `echanges.reporte` |
| BUL-444 | Contrôler un dépôt de bulletins tiers (pages, nom extrait ≠ élève rapproché) | SYS | bloquant | `bulletins.depot_controle` |
| BUL-445 | Déclarer une campagne « en cours dans l'ancien outil » (bloque une campagne ScolIA du même niveau) | ADF | M23-011 | — |
| BUL-446 | Suspendre une séance et reprendre (une seule séance ouverte par président) | PRES | M23-016 | `conseils.seance_suspendue` |
| BUL-447 | Déposer une note au président pour un groupe d'élèves (élèves cochés) | PRF | M23-028 | — |
| BUL-448 | Déposer les remarques des délégués (sans nominatif), versées au PV | DEL | M23-037 | — |
| BUL-449 | Avis de l'élève en deux parties : transmis (famille, conseil) / message confidentiel au PP | ELV | M23-051 ; le message n'entre jamais au dossier | `orientation.avis_eleve_recueilli` |
| BUL-450 | Réexaminer un avis pour l'examen avant l'export du LSL | chef ★ | M23-056 | `lsl.reexamine` |
| BUL-451 | Afficher l'information art. 13 de la procédure d'orientation | SYS | M23-078 | — |
| BUL-452 | Procédure « titulaires injoignables » : constat motivé, entretien, courrier | DIR | CB-47 | `orientation.titulaires_injoignables` |
| BUL-453 | Enregistrer une décision de justice « décide seul de l'orientation » (via socle 4) et recalculer la procédure | ADF ★ | M23-046 | `orientation.demandeur_change` |
| BUL-454 | Exemplaire « pour information » sans formulaire d'appel (parent sans exercice, service gardien) | SYS | M23-050 | — |
| BUL-455 | Accès borné de la commission (séance + 7 jours), filigrane, journal | SYS | R-B33 | — |
| BUL-456 | Marquer une pièce d'appel « santé, sous pli » | ADF | M23-068 | — |
| BUL-457 | Montée provisoire sur acceptation expresse (constitution des classes) | SYS | R-B25 ; socle 5 | `passage.provisoire` |
| BUL-458 | Trame « rejet d'une demande de redoublement » avec voies et délais | SYS | M23-060 | — |
| BUL-459 | Contrôle de compatibilité des spécialités avec l'offre et les barrettes, affiché à la famille | SYS | M23-026 ; non bloquant | — |
| BUL-460 | Lot d'impression « attendre la fratrie » | ADF | S23-09 | — |
| BUL-461 | Marque « adresse à vérifier » et suspension des envois postaux suivants | SYS, ADF | M23-034 | — |
| BUL-462 | Mise en veille d'un geste de la vue projetée | PRES | M23-074 | — |
| BUL-463 | Proposer automatiquement « délégués sortis » quand l'élève projeté est un délégué ou l'enfant d'un délégué | SYS | R-B30 | — |
| BUL-464 | Tracer l'acceptation de l'obligation de confidentialité à l'émargement | DEL | R-B03 | `conseils.confidentialite_acceptee` |
| BUL-465 | Alerte de consultations anormales de dossiers | SYS → DIR | S23-11 | `conseils.consultation_anormale` |
| BUL-466 | Purger en fin d'année les examens du mode doublon | SYS | R-B35 | `conseils.doublon_purge` |

### 28.3 Règles ajoutées
| Id | Règle |
|---|---|
| R-B23 | Une décision d'orientation ou de redoublement est **notifiée par titulaire** : ouverture authentifiée, remise en main propre émargée ou accusé de recommandé ; ni le dépôt au coffre ni le courrier simple ne valent notification. |
| R-B24 | Le délai d'appel court **par titulaire** depuis sa notification valable ; aucune acceptation réputée sans notification valable d'au moins un titulaire et expiration de son délai. |
| R-B25 | Tout titulaire peut faire appel ; la décision n'est définitive qu'à l'expiration des délais de chaque titulaire notifié sans appel ; l'acceptation expresse d'un seul ouvre la montée provisoire. |
| R-B26 | Une décision en appel est `en_attente_appel` ; la montée provisoire l'accepte, la clôture d'année l'attend ou la signale. |
| R-B27 | Les actes engageants exigent une session authentifiée (réauthentification passkey de moins de 5 minutes pour accepter, faire appel, retirer un appel) ou un papier signé ; le lien nominatif est en lecture seule ; seul l'auteur retire son appel. |
| R-B28 | La motivation suit la trame « connaissances, capacités, intérêts » ; jamais de diagnostic, de situation familiale ni de sanction. |
| R-B29 | Un indicateur « point à voir hors délégués » n'apparaît que sur la tablette du président. |
| R-B30 | En présence des délégués, l'ordre de passage est alphabétique ; « délégués sortis » est proposé automatiquement pour un délégué ou l'enfant d'un délégué. |
| R-B31 | La divergence entre l'avis de l'élève et la demande des titulaires n'apparaît qu'en vue complète. |
| R-B32 | Un compteur de redoublements inconnu n'est jamais lu comme zéro ; la décision exige une déclaration motivée. |
| R-B33 | La commission d'appel accède au dossier jusqu'à la séance + 7 jours, téléchargement filigrané, consultations journalisées ; les pièces de santé sont transmises sous pli. |
| R-B34 | Un acte reçu après la clôture de son étape est refusé côté serveur (date de réception, version de la procédure, clé d'idempotence). |
| R-B35 | Le mode doublon est inscrit au registre des traitements, limité à l'année, ses examens purgés à la fin ; il ne produit aucun document. |
| R-B20 (complétée) | La fiche commune montre aux deux titulaires les **vœux** ; le **commentaire** libre de chacun n'est visible que du personnel. |

### 28.4 Cas limites ajoutés
| # | Cas | Règle |
|---|---|---|
| CB-47 | Aucun titulaire joignable | constat motivé, entretien proposé, courrier, décision possible sans demande ; jamais d'acceptation réputée |
| CB-48 | Un titulaire accepte, l'autre fait appel le lendemain | R-B25 |
| CB-49 | Titulaire qui ouvre la décision après l'échéance de l'autre | son propre délai court (R-B24) |
| CB-50 | Retrait d'appel par l'autre titulaire | refusé (R-B27) |
| CB-51 | Décision de justice produite en cours de procédure | BUL-453 |
| CB-52 | Enfant placé sans délégation d'autorité | la demande reste aux titulaires ; le service est informé [NV, Q-BU15] |
| CB-53 | Élève délégué examiné | R-B30 |
| CB-54 | Deux séances qui se chevauchent pour le même président | BUL-446 |
| CB-55 | Dépôt reçu après la clôture (rejeu) | R-B34 |
| CB-56 | Arrivant au compteur de redoublements inconnu | R-B32 |
| CB-57 | Membre de la commission chef de l'établissement d'origine | contrôle automatique, récusation |
| CB-58 | Pièce médicale dans le dossier d'appel | sous pli (R-B33) |
| CB-59 | Bulletin tiers découpé avec la page d'un autre élève | bloquant au dépôt (BUL-444) |
| CB-60 | Lien nominatif ouvert par un tiers (boîte partagée) | lecture seule ; aucun acte possible (R-B27) |
| CB-61 | Avis de l'élève contraire à la demande | vue complète seulement (R-B31) ; message confidentiel au PP possible |
| CB-62 | Majorité pendant le délai d'appel | l'élève devient seul à pouvoir faire appel à compter de sa majorité ; les appels déjà formés par les parents restent [H juridique, Q-BU19] |

### 28.5 Questions ajoutées (*appliqué par défaut, D-031*, sauf mention)
| Id | Question | Recommandation |
|---|---|---|
| Q-BU14 | L'ouverture authentifiée d'un document vaut-elle notification d'une décision administrative ? | **Oui** dans ScolIA (R-B23) **[NV : à confirmer par un juriste, F23-6]** |
| Q-BU15 | Enfant placé : qui demande l'orientation ? | titulaires de l'autorité, sauf délégation **[NV, juriste]** |
| Q-BU16 | Avis de l'élève visible de ses parents ? | en deux parties (BUL-449) |
| Q-BU17 | Ordre de passage en présence des délégués | alphabétique (R-B30) |
| Q-BU18 | Délégation de signature des décisions | permise, datée (BUL-440) |
| Q-BU19 | Majorité pendant le délai d'appel | CB-62 **[H juridique]** |
| Q-BU20 | Durée d'accès de la commission | séance + 7 jours (S23-10) |
| Q-BU21 | Traduction des voies et délais | textes fixes pré-traduits et relus ; jamais une traduction automatique (M23-062) |

### 28.6 Compléments de modèle (réserve **2474–2476**, sans JSON, E1–E14)
```
2474  orn_notification      id, etablissement_id, decision_id, titulaire_personne_id, mode ('ouverture_authentifiee','main_propre','recommande'),
                            notifie_le, date_limite_appel DATE, emargement_objet_id NULL, accuse_objet_id NULL, saisi_par NULL, cree_le
                            -- remplace l'usage de orn_decision.notifie_le / date_limite_appel (gardées comme « première notification », lecture seule)
      orn_reponse           + mode ('en_ligne','papier'), reauth_le NULL, piece_objet_id NULL        -- R-B27
      orn_appel             + depose_par_personne_id obligatoire ; retrait seulement par cet auteur (déclencheur)
      orn_avis_eleve        + message_pp VARCHAR(1500) NULL (jamais exposé à la famille ni à l'IA, purge fin d'année + 1 an)
      orn_decision          + etat_definitif ('provisoire','en_attente_appel','definitive'), delegation_signature_id NULL
2475  orn_delegation_signature  id, etablissement_id, delegant_personne_id, delegue_personne_id, du, au, motif, cree_le
      orn_campagne_externe      id, etablissement_id, annee_id, niveau_id, outil_libelle, declaree_par, cree_le      -- BUL-445
      cns_participant       + confidentialite_acceptee_le NULL
      cns_conseil           + etat 'suspendu'
2476  bul_depot_controle    id, etablissement_id, import_ligne_id (21 V14), code, gravite, cree_le                    -- BUL-444
      orn_commission_acces  id, etablissement_id, appel_id, personne_libelle, jusqu_au, cree_le                        -- R-B33
```
Toutes : `etablissement_id NOT NULL`, FK composées, UUID, RLS forcée, registre E12, aucune colonne JSON ni BYTEA (pièces dans `ent_objet`).

### 28.7 Lots modifiés
- **B3** : suspension de séance, ordre alphabétique, sortie automatique des délégués, veille de l'écran, confidentialité tracée.
- **B6** : contrôle de dépôt tiers (BUL-444), « attendre la fratrie », adresse à vérifier.
- **B8** : notification par titulaire, actes authentifiés, lien en lecture seule, avis de l'élève en deux parties, titulaires injoignables, décision de justice, campagne « ancien outil », information art. 13 ; **critère de fin ajouté** : scénarios P11, P16, P18 de `PERSONAS_23.md` rejoués en tests.
- **B9** : délais par titulaire, appel par tout titulaire, commission bornée, pièces de santé sous pli, compteur inconnu, rejet de demande de redoublement.
- **B10** : montée provisoire / définitive.
- **B13** : refus IA M23-075, M23-076 au jeu d'évaluation.
- **Mode doublon** (§12) : registre, purge (R-B35), rapport d'écart (BUL-438).

### 28.8 Nouveaux points pour Florian
| Id | Point | Pourquoi Florian |
|---|---|---|
| F23-8 | **Recommandés** pour les décisions non conformes aux familles sans compte (coût, prestataire) — devenu nécessaire par le bloquant A | coût, contrat |
| F23-9 | **Juriste** : notification par ouverture authentifiée (Q-BU14), enfant placé (Q-BU15), majorité pendant le délai (Q-BU19), appel d'un seul titulaire | juriste |
| F23-10 | **Commission d'appel du réseau** (privé) : qui la compose, comment l'établissement la saisit aujourd'hui | connaissance du terrain |
| F23-11 | **Information art. 13** et **registre** du mode doublon avec le DPO avant l'année 2027-2028 | DPO |
