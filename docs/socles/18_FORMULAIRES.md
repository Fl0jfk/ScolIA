# Socle 18 — Formulaires, autorisations et consentements

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée avant tout nouveau code ; aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un service de production. Next.js **lu seulement**, de façon ciblée.
**Branche :** `ent-socle-17-18-conception`, partie de `origin/ent-mc0hoe`.
**Conçu avec le socle 17** (`17_ANNONCES.md`) : le 17 publie et relance, le 18 recueille, prouve et **répond à la question « a-t-on le droit ? »**.
**Passe personas :** `PERSONAS_18.md` ; ses ajouts sont faits **ici**, marqués **« ajout personas »** (pour l'essentiel au §35). **En cas de contradiction, le §35 l'emporte.**
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` D-017, D-022, D-023, D-026, D-027 (socle 7 : lien nominatif pour un signataire sans compte), D-028 (« ne pas solliciter »), D-031, D-034 (doublon 2027-2028 ; module santé désactivé sans hébergeur HDS), D-035 (le socle 42 en usage réel à la rentrée 2027), D-036, D-038 ; `docs/SOCLES.md` §18 ; `04_ADMINISTRATIF.md` §8.3 (consentements administratifs), §8.6 (droits du lien : `signer_quotidien`, `signer_documents`, `repondre_formulaires`), ports `titulaires_autorite`, `droits_lien`, `destinataires`, table `ent_eleve_desaccord` ; `02_ELEVE.md` §7.5 (capacité juridique, majorité) ; `07_DOCUMENTS.md` §10 (niveaux S0, S1, S2, G, P ; règle d'accord `un` / `tous` / `aucun` ; dossier de preuve ; « le retrait d'un consentement relève du socle 18 ») ; `08_NOTIFICATIONS.md` §13, §15 ; `42_SORTIES_VOYAGES.md` §5.4, §9.3, §11.2, R-SV39, R-SV40 ; `03_INSCRIPTIONS.md` §4.4 (accord des deux parents) ; `34_SANTE.md` ; `IA_COEUR.md` ; `PRA_SAUVEGARDES.md` §18.4.

**Conventions.** **[F]** fait vérifié dans le code ; **[V]** règle lue sur une source officielle (§4) ; **[NV]** non vérifiée ; **[H]** hypothèse ; **[R]** recommandation.

**Numérotation propre au socle** (vérifiée libre dans `docs/` le 2026-11-11) :

| Élément | Numéros | Remarque |
|---|---|---|
| Défauts de l'existant Rust | **FD-01**… | |
| Défauts Next.js | **FN-01**… | Next.js intouchable |
| Actions | **FRM-100 à FRM-279** | FRM-01 à FRM-05 sont pris par `SOCLE2_DOMAINES.md` (D24) et renvoient ici |
| Règles | **R-FC1**… | |
| Cas limites | **CK-01**… | (CF- est pris) |
| Questions | **Q-FC1**… | |
| Réglages signalés | **S18-01**… | |
| Lots | **FO0 à FO13** | (F0 est pris par les finances) |
| Manques de la passe | **M18-001**… | `PERSONAS_18.md` |

**Plage de migrations imposée : 2360–2379** (consigne du 2026-11-11). Aucune réservation antérieure n'existait pour le socle 18. Registre : `MIGRATIONS.md`.

---

## 0. En bref

1. **Ce qu'est un accord.** Un **fait juridique daté**, donné par **une personne habilitée à la date**, sur **un texte précis** (version et empreinte), pour **un usage précis**, prouvé selon son enjeu (S0, S1, S2, guichet, papier — socle 7), **révocable** selon sa nature. Et un **port unique** répond, avant chaque usage, à « **a-t-on le droit, ici, maintenant, pour cette personne ?** » — photo dans une actualité, participation à une sortie, liste transmise à l'association de parents, trombinoscope, enregistrement d'un cours en visio.
2. **Quatre natures juridiques, jamais confondues** (le défaut le plus répandu dans les établissements) : **information** (prise de connaissance : règlement, charte, sortie obligatoire) ; **collecte** (fiche de renseignements, enquête due : base « mission », **pas** de consentement à demander) ; **autorisation** (acte d'autorité parentale : sortie facultative, transport, activité) ; **consentement** (droit à l'image, productions d'élèves, diffusion de coordonnées, services facultatifs : libre, spécifique, retirable). L'auteur choisit la nature ; le logiciel en déduit qui signe, comment on prouve, si l'on peut retirer, et ce qui se passe sans réponse.
3. **Qui décide pour qui** vient des socles 2 et 4, jamais d'une case : titulaires actifs ayant le droit du lien requis, **règle d'accord** par objet (`un` titulaire avec présomption, `tous`, `eleve_seul`, `eleve_et_titulaires`), **avis de l'enfant** pour son image (C. civ. 372-1), désaccord déclaré, majorité et capacité juridique (tutelle, émancipation), interdiction de communication, élève placé.
4. **Droit à l'image par usage et par canal**, comme le modèle eduscol (classe, ENT, lettre, affiche, écran du hall, site public, réseaux, presse, vidéo), pour l'**année scolaire**, avec **accord des deux titulaires par défaut** et **refus de l'enfant qui l'emporte** — choix prudent tant qu'un juriste n'a pas tranché la qualification (Q-FC1).
5. **L'existant n'est pas nul — et la carte se trompe en le disant « absent »** : `193_formulaires.sql` et `formulaires.rs` gèrent des consentements versionnés à empreinte, des autorisations et des « sondages » oui / non ; le trombinoscope masque déjà la photo d'un élève dont un parent a refusé. Mais l'audit relève **un défaut critique et sept élevés** (§2.2) : **un consentement donné sur la version 1 vaut pour la version 2** sans nouveau recueil (FD-01, confirmé par lecture) ; le parent sous interdiction de communication **lit et signe** (FD-02) ; un parent **voit la décision de l'autre** (FD-03) ; l'élève majeur n'est pas prévenu, ses parents si (FD-04) ; la majorité ignore tutelle et émancipation (FD-05) ; les « sondages » n'ont ni question ni anonymat (FD-06) ; un seul accord suffit toujours, même là où il en faut deux (FD-17). → **Lot FO0**.
6. **Next.js** n'a **aucune autorisation ni aucun consentement réellement recueilli** [F] : droit à l'image **pré-coché « OK »** à l'ajout d'un élève dans un voyage, consentement des portes ouvertes **forcé à vrai**, consentement des rendez-vous d'inscription en simple booléen sans texte versionné. À la reprise, tout est **« inconnu »**, jamais « accordé » (R-SV40).
7. **Un moteur de formulaires typé, sans JSON** (D-036) : questions, options, conditions, sections en tables ; réponses en **valeurs typées** (une ligne par réponse, une colonne par type, contrainte « une seule renseignée ») ; unité de réponse déclarée (par élève et un titulaire, par élève et chaque titulaire, par foyer, par personne, par l'élève lui-même).
8. **Sondage anonyme vraiment anonyme** : participation et réponses dans deux tables **sans lien**, insertion **par lots mélangés**, date sans heure, **seuil de 5** répondants par case avant tout affichage, texte libre désactivable ; et le libellé dit honnêtement ce qui est garanti. Un sondage **ne vaut jamais scrutin** (arrêté du 2-7-2024).
9. **Données sensibles** : question de santé, de religion, d'opinion, de handicap, de situation sociale → **interdite par défaut** ; santé : seulement par un formulaire du socle 34, et **désactivé** tant qu'il n'y a pas d'hébergeur HDS (D-034). L'IA ne lit jamais une réponse de santé.
10. **Cohérence accord ↔ usage** (« faire mieux », A15) : chaque socle **déclare** ses usages (photo publiée, liste transmise, participation) ; un retrait, un désaccord, une expiration ou une majorité **relancent la vérification** et **masquent** ou **alertent** en moins d'une minute ; contrôle nocturne de rattrapage.
11. **Le plan** :

    | Élément | Volume | Numéros |
    |---|---|---|
    | Actions | 180 | FRM-100 à FRM-279 |
    | Règles | 56 | R-FC1 à R-FC56 |
    | Cas limites | 76 | CK-01 à CK-76 |
    | Réglages signalés | 12 | S18-01 à S18-12 |
    | Lots | 14 | FO0 à FO13 |

    **Parcours minimal** : démo de décembre 2026 → **FO0, FO1** ; V1 janvier–février 2027 → **+ FO2, FO3** ; **rentrée 2027 en usage réel** (le socle 42 remplace Next.js, D-035) → **FO3 au niveau production** (autorisations de sortie, droit à l'image, soins), FO5 (cohérence), FO8 (renouvellement de rentrée).
12. **Ajout personas (`PERSONAS_18.md`, §35 ci-dessous)** : quatre bloquants tranchés — **route anonyme hors des journaux du serveur**, **aucun état global montré aux familles** (il révélerait la décision de l'autre parent), **attestation en masse à deux personnes** à la bascule, **délégataire du droit à l'image** pour un enfant confié ; 62 manques, 15 actions (FRM-280 à FRM-294), 16 règles (R-FC57 à R-FC72).
13. **Décisions appliquées par défaut (D-031)** : §33.1 (26 points). **Soumis à Florian, au DPO et à un juriste** : §33.2 — qualification de l'image (un ou deux accords), valeur probante S1 pour une autorisation, durées de conservation, AIPD, application du D111-8 et de la circulaire de 2024 dans le privé.

---

## 1. Thèse : un accord est un fait, pas une case

Dans les établissements, « l'autorisation » est un **papier** : un coupon rendu (ou pas) dans le cartable, signé par l'un des parents (lequel ?), rangé dans un classeur, recopié en « droit à l'image : OUI » dans un tableur, jamais relu, valable « pour toujours », et que personne ne consulte avant de mettre la photo sur le site. Le jour où un parent séparé proteste, on ne sait ni qui a signé, ni quoi, ni pour quel usage. Le jour où la famille retire son accord, la photo reste en ligne.

D'où cinq principes :
1. **Un accord porte sur un texte et un usage**, pas sur un thème : « image — lettre d'information — année 2026-2027 — version 2 », pas « droit à l'image : oui ».
2. **Le logiciel calcule qui doit décider**, à la date, depuis le dossier (socles 2 et 4) ; l'auteur du formulaire ne choisit pas les signataires.
3. **L'absence de réponse n'est jamais un accord** (contre Next.js, où le défaut est « OK »).
4. **On vérifie au moment de l'usage**, pas au moment de la collecte : la question « a-t-on le droit ? » est posée par chaque socle consommateur, à chaque usage, et la réponse peut changer demain.
5. **La preuve est consultable par celui qui a décidé** (ce qu'il a vu, quand, comment), et **le retrait est aussi simple que le don** (RGPD art. 7.3).

---

## 2. Évaluation franche de l'existant

Audit en lecture seule (sous-agent, ~40 lectures) ; **FD-01, FD-02 et FD-05 relus ligne à ligne** par le concepteur (`crates/ent-db/src/formulaires.rs:55-69, 170-195, 560-592`).

### 2.1 Ce qu'il y a [F]

| Élément | Où | Ce que ça fait |
|---|---|---|
| Formulaires | `migrations/193_formulaires.sql:6-37` | `ent_formulaire` : `type` (`consentement`, `autorisation`, `sondage`), `objet` stable (`droit_image`…), `version`, titre, texte (≤ 10 000), statut, audience (`etablissement / niveau / classe / groupe`), `cible_ids UUID[]` ; unicité `(etablissement, objet, version)` |
| Réponses | `193:39-63` | `ent_formulaire_reponse` : élève, signataire, qualité (`responsable`, `eleve_majeur`), décision (`accepte`, `refuse`, `retire`), commentaire, **empreinte** du texte, date ; historique conservé (aucune mise à jour dans le code) |
| Service | `crates/ent-db/src/formulaires.rs` | `etat_effectif` (dernière décision de chaque signataire qualifié ; **un seul refus ou retrait → refus**), `signables` (titulaires avec autorité parentale ; élève majeur par son compte), `repondre_formulaire` (audience revérifiée, sérialisation par verrou consultatif, idempotence), `publier_formulaire` (archive l'ancienne version, notifie les responsables), `consentements_classe_for_actor`, `eleves_ayant_refuse`, `eleves_photo_masquee` |
| Consommateurs | `service.rs:14504, 14572` | **trombinoscope et lecture de photo** : photo masquée si refus ; aucun autre consommateur (ni sorties, ni voyages, ni documents) |
| Routes | `communication_s28.rs:46-77` | `/formulaires` (CRUD, publier, archiver, résultats) sous `formulaires.gerer` ; `/formulaires/mes`, `/{id}/repondre` (contrôle par lien) ; `/consentements/{objet}/classes/{classe_id}` sous `consentements.lire` |
| Écrans | Leptos `web/crates/ent-web-app/src/modules/formulaires/` (≈ 610 lignes) | gestion et réponse famille ; aucun écran mobile |
| Autres accords | `062_sortie_autorisation.sql` (régime de sortie, **un état unique écrasé**, sans historique) ; `070_doc_a_signer.sql` (signature mise à jour sur place) ; `269`/`275` `ent_personne_consentement` (consentements administratifs du socle 4, « migreront vers le socle 18 ») | |
| Tests | `tests_communication.rs:903-1050`, tests unitaires de `etat_effectif` (`formulaires.rs:833-861`), `communication_s28.rs` | |
| RLS | `480:83-110` | permissive ; aucun `TenantTx` |

### 2.2 Défauts trouvés

| # | Gravité | Défaut | Preuve | Scénario | Lot |
|---|---|---|---|---|---|
| **FD-01** | **critique** | **Un accord donné sur la version N vaut pour la version N+1.** L'état se calcule par `objet`, pas par version ; `a_repondre` aussi ; à la publication de la v2, la v1 est archivée et les familles ne sont pas re-sollicitées | `formulaires.rs:180` (`r.objet = $3`), `654-669` ; test `tests_communication.rs:~987` qui l'entérine (C, relu) | Droit à l'image v1 « journal interne » ; la v2 ajoute « réseaux sociaux » : les familles de la v1 sont « accepte » sans avoir lu la v2 — consentement non éclairé (RGPD art. 4.11, 7) | FO0 |
| **FD-02** | **élevée** | **Interdiction de communication ignorée** : lecture et signature par `ent_eleve_lien_actif`, pas par le lien communicant ; `/formulaires/mes` rend le texte et le prénom de l'enfant | `formulaires.rs:570`, `186` ; vue `ent_eleve_lien_communicant` (`480:15-39`) (C, relu) | Le père sous ordonnance de protection lit le texte d'une autorisation de sortie (date, lieu, heure de retour) et la signe | FO0 |
| **FD-03** | **élevée** | **Un parent voit la décision de l'autre** : la famille reçoit l'état effectif (tous signataires) | `formulaires.rs:122-123, 648-652, 675` (C) | Parents séparés : le père voit « refusé » alors qu'il n'a rien répondu, il en déduit la réponse de la mère | FO0 |
| **FD-04** | **élevée** | **Élève majeur non prévenu ; ses parents prévenus** d'un formulaire qu'ils ne peuvent pas remplir | `publier_formulaire:415-451` ; `signables:590` (C) | Étudiant de BTS de 19 ans : rien ; ses parents : « À compléter » ; le droit à l'image reste « aucune » | FO0 |
| **FD-05** | **élevée** | **Majorité calculée sans la capacité** (date de naissance + 18 ans brute) : tutelle, curatelle, émancipation, date approximative ou contestée ignorées ; la fonction canonique `ent_eleve_majorite_le` (285) n'est pas utilisée | `formulaires.rs:205, 567, 575` ; `perimetre.rs:126-130` (C, relu) | Majeur sous tutelle qui signe seul ; mineur émancipé dont les parents signent | FO0 |
| **FD-06** | **élevée** | **« Sondages » sans contenu ni anonymat** : décisions oui / non seulement ; signataire et élève toujours stockés ; comptages qui réidentifient dans un petit groupe | `193:47, 55` ; `formulaires.rs:143` (C) | Une « enquête anonyme » de climat scolaire où la direction peut retrouver chaque votant | FO0 (retirer l'étiquette « anonyme »), FO4 |
| FD-07 | élevée | RLS inopérante (aucun `TenantTx`) ; toutes les requêtes filtrent toutefois `etablissement_id` (aucune fuite entre établissements trouvée) | (C) | Requête future sans filtre non arrêtée | FO1, FO13 |
| FD-17 | **élevée** | **Un seul accord suffit toujours** : aucune règle par objet (`tous` impossible) ; le silence du second titulaire est ignoré ; aucun désaccord remonté au personnel | `etat_effectif:55-69` (C, relu) | Photo d'un élève sur le site sur le seul accord de la mère, alors que les parents « protègent en commun » le droit à l'image (372-1) et que le père s'y oppose oralement | FO0 (règle `tous` pour l'image), FO3 |
| FD-08 | moyenne | **Titre du formulaire en clair** dans l'e-mail et le push (« À compléter : Consentement suivi psychologique ») | `notifications.rs:443-445` (le type `formulaire` n'est pas dans les neutres) ; `formulaires.rs:426` (C) | Titre sensible sur l'écran verrouillé, transmis à Apple ou Google | FO0 |
| FD-09 | moyenne | **Lecture des consentements sans portée ni journal** (`consentements.lire` à l'établissement seulement) | `formulaires.rs:515-555` ; `actions.rs:755-800` (C) | Un administratif lit les refus de toutes les classes sans trace ; un professeur ne peut pas savoir qui peut être photographié dans **sa** classe | FO1 |
| FD-10 | moyenne | **Autorisation signée sans le droit du lien requis** : seule l'autorité parentale est exigée (`autoriser_sorties` ignoré, contrairement à 062) | `signables:570-572` ; `service.rs:12324` (C) | Parent à qui la direction a retiré `autoriser_sorties` signe une autorisation de sortie | FO0 |
| FD-11 | moyenne | Journal incomplet : publication et archivage sans acteur (`publie_par`, `archive_par` absents) | `formulaires.rs:456-474` (C) | Qui a publié la v2 ? | FO0 |
| FD-12 | moyenne | **Aucune immuabilité en base** des réponses ni du texte publié | aucun déclencheur sur `ent_formulaire_reponse`, `ent_formulaire.texte` (C) | Une modification directe change le texte « accepté » après coup | FO0 |
| FD-13 | moyenne | `eleve_id` sans clé étrangère ; `cible_ids UUID[]` sans FK ; cascade depuis `ent_user` sur des preuves voisines | `193:17, 45` (C) | Cible fantôme après fusion de classe | FO1 |
| FD-14 | moyenne | Publier un brouillon de numéro **inférieur** archive la version plus récente | `publier_formulaire:389-400` (C) | Retour silencieux à un texte ancien | FO0 |
| FD-15 | moyenne | **Type non verrouillé par objet** : une « autorisation » d'objet `droit_image` devient la v2 du consentement | `193:34-35` ; `creer_formulaire` (C) | Mélange des natures | FO0 |
| FD-16 | moyenne | **Sans décision, la photo reste visible** (le port ne renvoie que les refus) | `formulaires.rs:23-26, 820` (C) | Acceptable pour un trombinoscope **réservé aux personnels** (mission), pas pour un usage vu des familles : à borner par usage | FO1 |
| FD-18 | faible | Empreinte `sha256(objet|version|titre|texte)` : séparateur non échappé ; ne lie ni la décision, ni le signataire, ni la date | `formulaires.rs:772-778` (C) | Collision artificielle possible ; preuve faible | FO1 (empreinte encodée avec longueurs ; preuve de décision au socle 7) |
| FD-19 | faible | Objet non validé en lecture | `consentements_classe` (C) | — | FO0 |
| FD-20 | moyenne | `062` : autorisation de régime de sortie **écrasée** sans historique (`UNIQUE (etab, eleve)`) | `062_sortie_autorisation.sql` (C) | On ne sait plus qui a autorisé la sortie libre en septembre | FO7 (avec le socle 14) |
| FD-21 | moyenne | `070` : document à signer accepté de **tout lien actif** (autorité non vérifiée), signature écrasée, jour UTC | `service.rs:13579-13632` (C) | Beau-parent sans autorité qui signe | socle 7 (D0) — signalé |

**Fonctions absentes** [F] : échéance, relance des non-répondants, durée de validité, réponse papier saisie par le personnel, pièce jointe, version affichée à la famille, annulation d'une réponse par l'établissement, questions, résultats autres que des comptages.

### 2.3 Verdict
- **Garder** : l'historique en ajout seul côté code ; « un seul refus protège » (en le généralisant par règle) ; le verrou par signataire et l'idempotence ; l'empreinte du texte (renforcée) ; le masquage de la photo du trombinoscope (comme premier consommateur du port) ; les écrans Leptos (à étendre).
- **Corriger tout de suite** (lot **FO0**) : FD-01 à FD-06, FD-08, FD-10 à FD-12, FD-14, FD-15, FD-17, FD-19.
- **Remplacer** : le formulaire « texte + oui / non » par **trois objets** (formulaire à questions ; objet d'accord à usages ; décision) ; `062` et les consentements administratifs de `269/275` par des objets d'accord du 18 (reprise additive) ; `070` reste au socle 7.

### 2.4 Next.js (lecture seule, D-035)
Aucun module de formulaires, de sondages ni de consentements [F]. Ce qu'il y a, et ce qu'on en fait :

| Fonction ou défaut [F] | Preuve | Reprise [R] |
|---|---|---|
| **FN-01** Droit à l'image **pré-coché « OK »** à l'ajout d'un élève dans un voyage ; booléen **vrai par défaut** en base | `app/components/travels/hub/TripElevesListPanel.tsx:869-870` ; `db/schema-ent-relational.ts:228` (relevé par `PERSONAS_42.md`) | repris **« inconnu »** (R-SV40) ; jamais « accordé » |
| **FN-02** Consentement des **portes ouvertes forcé à vrai**, sans case | `app/api/portes-ouvertes/register/route.ts:95-96` ; `app/api/accueil/portes-ouvertes/route.ts:238` | prospects : objet `recontact_prospect` (FRM-238), case **non cochée**, texte versionné |
| **FN-03** Rendez-vous d'inscription : consentement exigé en **simple booléen**, texte non versionné | `app/api/rdv-inscription/[direction]/book/route.ts:44-47` | objet versionné, preuve |
| **FN-04** Covoiturage : `consentAt` posé à la première inscription sans geste de consentement | `app/api/covoiturage/route.ts:198-211` | module perdable (T0) ; si repris : objet versionné |
| Modèle « charte photos » et case `droitImage` dans les gabarits de documents | `app/lib/rgpd-templates.ts:263-266` ; `app/lib/document-templates/catalog.ts:65` | textes de départ des objets d'accord (relus) |
| Bon exemple : inscription à la plateforme refusée sans consentement | `app/api/platform/signup-requests/route.ts:56-57` | même principe |
| Aucun bandeau de cookies ; cookies techniques seulement | `app/confidentialite/page.tsx:296-300` [à vérifier] | ScolIA : aucun traceur, donc aucun bandeau (R-FC54) |

### 2.5 Marché [H, non vérifié]
Pronote (sondages, « informations » avec accusé), École Directe (formulaires, autorisations), ENT régionaux (sondages simples) : formulaires **à plat** sans règle de signataires, droit à l'image **booléen par élève**, aucun contrôle au moment de l'usage, aucun retrait propagé. Les outils grand public (Google Forms, Framaforms) sont utilisés en contournement, avec les problèmes de transfert et d'identité qu'on imagine.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il doit faire | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Direction** | s'assurer qu'on a le droit (image, sortie, transmission) ; trancher un désaccord ; répondre au parent qui proteste | reconstituer qui a signé quoi | la photo publiée sans accord ; l'élève parti en voyage sans autorisation valable ; la liste transmise sans accord exprès | une réponse en un clic à « avait-on le droit le 12 mai ? », avec la preuve |
| **Secrétariat** | lancer les campagnes de rentrée (image, coordonnées, soins), saisir les papiers, relancer | coupons perdus, tableurs, relances à la main, enveloppes | se tromper d'enfant en saisissant un papier ; écraser un refus | « reprendre les choix de l'an dernier » en un geste pour les familles, et un guichet de saisie qui vérifie le signataire |
| **CPE, vie scolaire** | autorisations de sortie libre, activités, transport, enquêtes de climat | savoir qui a le droit de sortir seul | autoriser sur un papier signé par un parent sans autorité | l'état exact, daté, par élève, au portail |
| **Professeur, organisateur de sortie** | savoir qui peut partir, qui peut être photographié ; faire un sondage à sa classe (choix d'un sujet, d'une date) | demander à la vie scolaire | voir des réponses qu'il ne devrait pas voir (refus motivés, santé) | une liste « peut / ne peut pas » pour **sa** classe, sans le pourquoi |
| **Professeur documentaliste, référent numérique** | publications, journal, web radio | — | publier une production sans l'accord de l'élève | l'accord « productions » vérifié à l'avance |
| **Infirmière** | autorisation de soins d'urgence, questionnaire de santé de voyage | formulaire santé dans un outil non hébergé HDS | donnée de santé hors du cabinet | un formulaire de santé **dans le socle 34**, quand il sera permis |
| **Familles** (séparées, recomposées, allophones, sans numérique) | répondre vite, comprendre ce qu'on leur demande, retirer un accord | vingt coupons par an, les mêmes questions chaque année, rien sur téléphone | que l'autre parent voie leur réponse ; signer sans comprendre | un écran par enfant « ce qu'on vous demande » ; « comme l'an dernier » ; retrait en un geste ; leur langue |
| **Élève mineur** | donner son avis sur son image ; répondre aux sondages de classe ; co-signer une charte | — | que sa photo circule malgré lui | voir ses accords et dire non à sa photo |
| **Élève majeur** | décider seul | parents encore sollicités | — | être seul décideur, informé directement |
| **Tuteur, service gardien (ASE), famille d'accueil** | signer selon leur mandat | ne pas être reconnus comme signataires | signer au-delà de leur mandat | des droits du lien clairs (`signer_quotidien`) |
| **Personnels** (sujets) | accord sur leur propre image ; enquêtes internes (climat, vœux de service) | — | que l'enquête « anonyme » ne le soit pas | anonymat garanti et expliqué |
| **Association de parents** | obtenir la liste des parents qui l'acceptent (D111-8) | — | être accusée de démarchage | une liste exacte, datée, révocable |
| **DPO** | registre, AIPD, preuve des consentements, durées, droits | consentements demandés à tort (base « mission ») ; formulaires avec données sensibles | sondage réidentifiable ; santé hors HDS | une nature juridique choisie à la création, des durées par objet, des exports de preuve |
| **Juriste** | qualifier les actes (usuel, non usuel), la valeur de la preuve | — | — | un catalogue d'objets relu une fois, réutilisé partout |
| **Attaquant** (élève, parent, tiers) | signer pour un autre, voir les réponses des autres, réidentifier un sondage | — | — | — (ce que la conception doit empêcher, `PERSONAS_18.md`) |

---

## 4. Ce qui est réglementaire (vérifié le 2026-11-11)

Recherche par sous-agent (~86 lectures) : Légifrance (articles), EUR-Lex, cnil.fr, eduscol (PDF lus localement), assemblee-nationale.fr ; le BO et les pages HTML d'eduscol renvoient 403. **[V]** = source ouverte et lue le 2026-11-11 ; **[V-partiel]** = page ouverte, texte non reproduit en entier ; **[NV]** = non ouverte.

### 4.1 Sources

| # | Texte | Ce qu'il dit | URL | Statut |
|---|---|---|---|---|
| RF-1 | **C. civ. 372-1** (rétabli par la loi n° 2024-120 du 19-2-2024) | « Les parents protègent **en commun** le droit à l'image de leur enfant mineur, dans le respect du droit à la vie privée mentionné à l'article 9. Les parents **associent l'enfant** à l'exercice de son droit à l'image, selon son âge et son degré de maturité. » (en vigueur depuis le 21-2-2024) | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006426505/ | [V] |
| RF-2 | **C. civ. 373-2-6** (même loi) | le juge peut, en cas de désaccord sur le droit à l'image, « interdire à l'un des parents de diffuser tout contenu relatif à l'enfant sans l'autorisation de l'autre parent » | texte adopté, https://www.assemblee-nationale.fr/dyn/16/textes/l16t0239_texte-adopte-provisoire.pdf | [V] |
| RF-3 | **C. civ. 377** (même loi) | délégation partielle forcée limitée au droit à l'image en cas d'atteinte grave par les parents | idem | [V] |
| RF-4 | **Circulaire Justice du 23-5-2024** (JUSC2405229C) | si la diffusion « s'analyse en un acte usuel », chaque parent est réputé agir avec l'accord de l'autre (372-2) ; sinon accord des deux ; **ne tranche pas** la qualification ; « contenu » = photo, film, enregistrement sonore | copie CNB : https://cnb.avocat.fr/sites/default/files/documents/circulaire_de_presentation_des_dispositions_de_loi_n_2024-120_du_19_fevrier_2024_visant_a_garantir_le_respect_du_droit_a_limage_des_enfants_-_240523.pdf | [V] (copie) |
| RF-5 | **C. civ. 372-2** | « À l'égard des tiers de bonne foi, chacun des parents est réputé agir avec l'accord de l'autre, quand il fait seul un acte usuel de l'autorité parentale relativement à la personne de l'enfant. » | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006426536 | [V] |
| RF-6 | **CNIL, photos d'élèves** | pour utiliser les photos des élèves « dans le journal de l'école, sur un trombinoscope, sur son site ou sur tout autre support », l'établissement « doit obligatoirement demander le **consentement écrit des représentants légaux** » | https://cnil.fr/fr/cnil-direct/question/mon-enfant-ete-pris-en-photo-lecole-puis-je-mopposer-lutilisation-de-son-image | [V] (page non datée) |
| RF-7 | **Modèle eduscol** d'autorisation d'enregistrement et d'utilisation de l'image et de la voix d'un mineur (2023) | consentement **de l'élève** + autorisation des représentants (deux signataires prévus) ; **oui / non par canal** (usage en classe, site à accès réservé / ENT, Internet, support amovible, projection, réseaux sociaux) ; durée « 1 année scolaire » ; rubrique « exercer vos droits » | https://eduscol.education.gouv.fr/sites/default/files/document/autorisation-captation-utilisation-image-voix-mineur-concours-des-dix-motspdf-101859.pdf | [V] (modèle d'un concours, antérieur à 2024) |
| RF-8 | **CNIL, recommandation 4 sur les droits des mineurs** (9-6-2021) | pour le consentement parental de l'art. 45 (services en ligne), un parent suffit (acte usuel, après consultation du ministère de la Justice), l'autre étant « mis en capacité d'exprimer son opposition » ; tenir compte de la maturité | https://www.cnil.fr/fr/recommandation-4-rechercher-le-consentement-dun-parent-pour-les-mineurs-de-moins-de-15-ans | [V] |
| RF-9 | **CPI L111-1, L122-4** | l'élève est auteur de son œuvre dès la création ; toute reproduction ou représentation sans son consentement est illicite | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000042814694 ; https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006278911 | [V] ; L121-1 (droit moral) [NV] |
| RF-10 | **RGPD art. 6, 7, 8, 9, 21** | bases légales ; preuve du consentement ; demande distincte ; **retrait aussi simple que le don** ; liberté appréciée ; < 16 ans (13 à 16 au choix des États) ; données sensibles interdites sauf exceptions (consentement **explicite**) ; opposition pour les traitements fondés sur la mission (6.1.e) | https://eur-lex.europa.eu/legal-content/FR/TXT/HTML/?uri=CELEX:32016R0679 | [V] (art. 12-13 et 89 non relus mot à mot) |
| RF-11 | **Loi 78-17 art. 45** | à partir de **15 ans**, le mineur consent seul pour les services de la société de l'information ; en dessous, consentement **conjoint** du mineur et du ou des titulaires ; information claire pour le mineur | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000037823135 | [V-partiel] |
| RF-12 | **CNIL, outils collaboratifs et ENT** (24-8-2026), public **et privé** | base légale : **mission d'intérêt public** ; « le consentement semble difficilement pouvoir être retenu » (pas libre face à l'autorité) ; neutralité commerciale ; information claire, adaptée aux mineurs, à chaque rentrée ; AIPD « probable » ; contrat de sous-traitance | https://www.cnil.fr/fr/enseignement-premier-second-degres-bonnes-pratiques-outils-collaboratifs | [V] |
| RF-13 | **Guide DGESCO « L'exercice de l'autorité parentale en milieu scolaire »** (2011) | actes **usuels** (liste indicative) : inscription, justification d'absences brèves, attestation, **autorisation de sortie scolaire en France** ; **non usuels** : orientation, inscription dans le privé, redoublement ; acte non usuel s'il « rompt avec le passé » ou « engage l'avenir » ; mêmes documents aux deux parents ; image non citée | https://eduscol.education.gouv.fr/sites/default/files/document/autoriteparentale170278pdf-68094.pdf | [V] (antérieur à 2024) |
| RF-14 | **Sorties** : circulaire du 16-7-2024 (BO n° 30), guide eduscol de décembre 2025 | sortie **obligatoire** : pas d'autorisation ; **facultative** : autorisation **écrite, datée, signée** d'un titulaire, accord de l'autre **présumé sauf désaccord connu** ; AST (C. civ. 371-6) à l'étranger | `42_SORTIES_VOYAGES.md` §5.1, §5.4 (vérifié le 2026-11-10) | [V-copie] (42) |
| RF-15 | **C. éduc. D111-8** | les associations de parents n'obtiennent la liste des parents qu'avec leur **accord exprès** | `04_ADMINISTRATIF.md` §8.3 (VERIF R5) ; section D111, https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006151393/ | [V] |
| RF-16 | **Charte d'usage de l'internet**, annexée au règlement intérieur, « signée par les élèves et leurs parents » | circulaire n° 2004-035 du 18-2-2004 citée par deux réponses ministérielles (12ᵉ législature, QE 59854, 74873) | https://www.assemblee-nationale.fr/dyn/12/questions/QANR5L12QE59854.pdf | [V-partiel] ; statut 2026 [NV] |
| RF-17 | **Signature électronique** : C. civ. 1367 ; règlement eIDAS art. 25, 26 | la signature « identifie son auteur » et « manifeste son consentement » ; fiabilité **présumée** seulement pour la signature qualifiée (décret 2017-1416) ; ailleurs, preuve libre ; une signature ne peut être refusée « au seul motif » qu'elle est électronique | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000032042456 ; https://eur-lex.europa.eu/legal-content/FR/TXT/HTML/?uri=CELEX:02014R0910-20241018 | [V] ; détail au socle 7 §10 |
| RF-18 | **Vote électronique des représentants de parents** | arrêté du 2-7-2024 : système audité par un expert indépendant, séparation des listes et des votes, secret, vote de 24 h à 5 jours | https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000049871171 | [V] (établissements publics) |
| RF-19 | **Archives des EPLE** : instruction DAF/DPACI/RES/2005/003 et tableaux (BO n° 24 du 16-6-2005) | dossiers de sorties et voyages : **2 ans** (sauf contentieux), tri ; mots d'excuse : 1 an ; questionnaires d'enquêtes : 5 ans, tri ; productions audiovisuelles d'élèves : 2 ans ; **aucune ligne** pour les autorisations, le droit à l'image ni les consentements | https://www.education.gouv.fr/bo/BoAnnexes/2005/24/tableaux_encart24.pdf | [V] (2005, mise à jour [NV]) |
| RF-20 | **Accessibilité** : RGAA 4.1.2 (v5 annoncée fin 2026) | voir `17_ANNONCES.md` RG-9 | https://accessibilite.numerique.gouv.fr/obligations/champ-application/ | [V] |

### 4.2 Conséquences pour la conception
1. **La nature juridique est choisie à la création** (§6) ; « consentement » est **refusé** comme nature pour ce qui relève de la mission (fiche de renseignements, charte, règlement) (RF-12, R-FC1).
2. **Droit à l'image** : objet à **usages** (RF-7) ; **règle `tous` par défaut** et **avis de l'élève** recueilli dès 11 ans [R], son refus l'emportant (RF-1, RF-4 : la qualification n'étant pas tranchée, on choisit la prudence ; Q-FC1 au juriste) ; durée **année scolaire** (RF-7).
3. **Productions d'élèves** : objet distinct de l'image (RF-9), élève **et** titulaires pour un mineur, mode de signature (nom, prénom, anonyme).
4. **Sortie facultative en France** : règle `un` (acte usuel, RF-13, RF-14), **désaccord connu = refus** ; sortie obligatoire : **information** (prise de connaissance facultative), jamais d'autorisation exigée ; voyage à l'étranger : règle `tous` par défaut (socle 7 D77) [R].
5. **Liste des parents aux associations** : consentement **exprès**, refusé par défaut, révocable (RF-15).
6. **Charte du numérique** : prise de connaissance **S0 de l'élève et d'un titulaire** (RF-16), version par version, **sans bloquer** l'accès des parents à l'ENT (R-FC20).
7. **Preuve** : S1 (session récente ou code) pour les autorisations courantes, S2 pour les actes non usuels, guichet et papier toujours possibles (RF-17, socle 7) ; dossier de preuve consultable.
8. **Retrait** aussi simple que le don, depuis l'écran de l'accord, propagé aux usages (RF-10).
9. **Mineur de plus de 15 ans** : pour un service facultatif fondé sur le consentement (art. 45), l'élève consent seul ; **cela n'efface pas** la règle propre au droit à l'image (RF-1) (R-FC9).
10. **Données sensibles** interdites par défaut dans les formulaires ; santé au socle 34 seulement (RF-10 art. 9).
11. **Sondage ≠ scrutin** (RF-18) : libellé « consultation, sans valeur de scrutin ».
12. **Conservation** : durées par objet, à valider par le DPO ; défauts tirés de RF-7 et RF-19 (§30).

### 4.3 Incertitudes à faire trancher (juriste, DPO) — Q-FC1 à Q-FC8
1. **Image** : un titulaire (acte usuel, l'autre pouvant s'opposer, modèle CNIL RF-8) ou les deux (non usuel) ? et selon l'usage (ENT fermé vs site public) ? → défaut prudent `tous` pour tous les usages hors `classe` (Q-FC1).
2. Valeur de la **notification à l'autre titulaire avec droit d'opposition** comme alternative à la double signature (Q-FC2).
3. **Valeur probante** de S1 pour une autorisation de sortie ; besoin de S2 pour un voyage à l'étranger (Q-FC3).
4. **Privé sous contrat** : D111-8, circulaire de 2024, arrêté de 2024 ; responsable de traitement (chef d'établissement ou organisme de gestion) (Q-FC4).
5. **Art. 45** : applicable à un service facultatif d'un établissement ? âge de l'avis de l'enfant (Q-FC5).
6. **Seuil d'anonymat** et techniques ; AIPD (Q-FC6).
7. **Durées** de conservation par objet (Q-FC7).
8. Statut 2026 des circulaires 2004-035 et du guide de 2011 (Q-FC8).

---

## 5. Frontières

### 5.1 Qui possède quoi
| Objet | Possédé par | Le socle 18 |
|---|---|---|
| Formulaires, questions, réponses, sondages | **18** | possède |
| Objets d'accord (catalogue), usages, textes versionnés, décisions, état calculé, usages déclarés, alertes de cohérence | **18** | possède |
| Consentements administratifs du socle 4 (coordonnées aux associations, au transporteur, photo des personnes autorisées) | **4 définit les codes, 18 stocke** (`04_ADMINISTRATIF.md` §5) | possède le stockage (reprise de `ent_personne_consentement`) |
| Qui peut signer pour un élève (titulaires, droits du lien, désaccord, interdiction) | **4** | consomme |
| Capacité juridique, majorité, protection de l'élève | **2** | consomme |
| Preuve de signature (S0–S2, G, P), dossier de preuve, pièces jointes, scans papier | **7** | consomme |
| Relances, avis, diffusion | **8** | consomme |
| Publication d'une campagne | **17** | le 17 lie un formulaire comme attente |
| Sorties et leurs exigences | **42** | le 42 lie ses formulaires et appelle `verifier` |
| Régime de sortie, personnes autorisées à récupérer | **14** et **4** | `062` repris en objet d'accord (FO7) avec le 14 |
| Formulaires de santé | **34** (moteur du 18, stockage restreint du 34) | désactivé sans HDS |
| Enquêtes sociales, bourses | **26**, **36** | pas par un formulaire générique |
| Vote | **46** | jamais par un sondage |
| Préinscription, fiche de rentrée | **3**, **4** (leurs propres parcours) | rebranchés sur le moteur de questions (FO7) |

### 5.2 Ports exposés [R]
| Port | Consommateurs | Contrat |
|---|---|---|
| `autorisations::verifier(sujets, objet, usage, date) -> Vec<Verdict>` | 17, 42, 43, 14, 2 (trombinoscope), 33 (enregistrement visio), 32 (publication de productions), 46, 4 (transmission aux associations) | verdict ∈ `autorise`, `refuse`, `en_attente`, `desaccord`, `expire`, `a_renouveler`, `protege`, `inconnu`, `non_requis` ; **raison codée sans détail** pour un appelant non habilité ; **jamais** de booléen par défaut vrai (R-FC3) |
| `autorisations::declarer_usage(sujet, objet, usage, contexte_socle, contexte_id, du, au)` / `retirer_usage` | 17, 42, 2, 33, 32, 4 | registre des usages effectifs, pour la cohérence (§14) |
| `formulaires::creer_pour(socle, modele, cibles, echeance) -> FormulaireId` | 42 (campagne d'autorisations par sortie), 17, 43, 3, 4 | crée un **brouillon** au nom de l'utilisateur |
| `formulaires::etat_cibles(formulaire) -> Vec<(cible, etat)>` | 17 (attentes), 42 | répondu / non répondu / état d'accord, **sans contenu** |
| `formulaires::reponses(formulaire)` | auteur et habilités seulement | contenu, sous droits |
| `autorisations::accords_de(personne) -> Vec<Accord>` | 27 (droit d'accès), écran famille | |

### 5.3 Ports consommés [R]
`personnes::titulaires_autorite(eleve, date)` (avec `desaccord_declare`), `droits_lien(personne, eleve, droit, date)`, `destinataires` (4) ; `eleve::capacite(eleve, date)` (2) ; `documents::preuve_signature`, `::piece`, `::lot_impression` (7) ; `notifications::*` (8) ; `structure::eleves_de` (5) ; `personnels::de` (6).

---

## 6. Concepts : quatre natures, trois objets

### 6.1 Les quatre natures juridiques [R]
| Nature | Exemples | Base | Qui | Preuve par défaut | Retrait | Sans réponse |
|---|---|---|---|---|---|---|
| **Information** (prise de connaissance) | règlement intérieur, charte du numérique, sortie obligatoire, protocole d'examen, calendrier | mission | titulaire(s) et/ou élève selon l'objet | S0 | non (une nouvelle version demande une nouvelle prise de connaissance) | rien n'est bloqué ; relance et signal « non pris connaissance » |
| **Collecte** | fiche de renseignements, enquête de rentrée, choix d'options proposé par l'établissement, inscription à la demi-pension, questionnaire d'orientation | mission (ou obligation) | selon l'unité de réponse | S0 ou S1 | correction possible jusqu'à la clôture | relance ; saisie au guichet |
| **Autorisation** (acte d'autorité parentale) | sortie facultative, voyage, transport en véhicule d'un accompagnateur, activité à risque, sortie seul (régime), soins d'urgence pendant une sortie | autorité parentale (C. civ. 371-1 s.) | titulaire(s) selon la règle d'accord ; l'élève majeur seul | S1 (S2 pour un acte non usuel) | oui, jusqu'à l'événement (ensuite le fait est acquis) | **non autorisé** |
| **Consentement** | droit à l'image par usage, productions d'élèves, voix, diffusion des coordonnées (associations, transport), liste des représentants, mention nominative, recontact d'un prospect, service facultatif | consentement (RGPD 6.1.a ; C. civ. 9 et 372-1 ; CPI L122-4) | titulaire(s) **et** avis de l'élève ; l'élève majeur seul | S1 | **oui, à tout moment, aussi simplement** | **refusé** |

**Contrôle à la création** (R-FC1) : un objet de nature « consentement » qui porte sur un usage **nécessaire à la mission** (fiche de renseignements, liste d'appel, trombinoscope réservé aux personnels, cantine) est refusé avec l'explication « ceci relève de la mission de l'établissement : utilisez une collecte ou une information ». Inversement, une « collecte » qui demande une donnée facultative (photo pour un usage public, religion) est refusée.

### 6.2 Trois objets [R]
1. **Objet d'accord** (`ent_acc_objet`) : une chose sur laquelle on donne un accord (« droit à l'image », « sortie au musée du 12 mars », « liste des représentants de 2ᵈᵉ 4 »), avec sa nature, sa **règle d'accord**, le **droit du lien** exigé, sa **validité**, ses **usages** et ses **versions de texte**.
2. **Formulaire** (`ent_frm_formulaire`) : un questionnaire à questions, qui peut **contenir** des accords (une question « accord » renvoie à un objet d'accord) et des questions ordinaires.
3. **Décision** (`ent_acc_decision`) : ce qu'une personne a décidé, à une date, sur une version, pour un sujet, usage par usage, avec sa preuve. **Ajout seul.**

### 6.3 Vocabulaire fixé [R]
| Mot | Sens |
|---|---|
| **Sujet** | la personne concernée par l'accord (élève, personnel, parent, prospect) |
| **Décideur** | la personne qui décide (titulaire, élève, tuteur, personnel pour lui-même) |
| **Qualité** | à quel titre il décide : `titulaire`, `eleve_avis` (mineur, avis), `eleve_cosignataire`, `eleve_majeur`, `tuteur`, `curateur_assiste`, `service_gardien`, `personne_soi` |
| **Règle d'accord** | `un`, `tous`, `eleve_seul`, `eleve_et_titulaires`, `soi` |
| **Usage** | ce à quoi sert l'accord (`fil_classe`, `site_public`, `participation`…) |
| **Validité** | `annee_scolaire`, `jusqu_a` (date), `evenement` (fin de la sortie), `scolarite`, `revocable_sans_terme` |
| **Verdict** | réponse du port `verifier` |
| **Usage déclaré** | un usage effectif enregistré par un socle consommateur (photo publiée, liste transmise) |

---

## 7. Qui décide pour qui

### 7.1 Le calcul [R]
À la date, pour un sujet **élève** :
1. **Capacité** (socle 2) : mineur, majeur, émancipé (décide seul), majeur sous tutelle (le tuteur décide, l'élève est informé), sous curatelle (l'élève décide **assisté** du curateur pour un acte qui l'exige [NV]) ; date de naissance approximative ou contestée → **majorité non présumée**, décision des titulaires et alerte au secrétariat (corrige FD-05).
2. **Mineur** : décideurs = titulaires de l'autorité parentale **actifs** (`titulaires_autorite`) ayant le **droit du lien** exigé par l'objet (`signer_quotidien` pour une autorisation usuelle, `signer_documents` pour un acte non usuel, `repondre_formulaires` pour une collecte), **lien communicant** (une interdiction de communication l'exclut, corrige FD-02) ; service gardien et famille d'accueil selon leur lien (`signer_quotidien`).
3. **Règle d'accord** de l'objet :
   - `un` : il suffit d'**un** accord ; un **refus** d'un titulaire = **désaccord connu** (la présomption tombe, RF-14) → `desaccord` ; le second titulaire est **informé** et peut s'opposer [R, RF-8] ;
   - `tous` : chaque titulaire actif doit accorder ; le silence de l'un → `en_attente` ;
   - `eleve_seul` : l'élève (majeur, ou pour un objet `mineur_seul` comme la prise de connaissance de la charte) ;
   - `eleve_et_titulaires` : l'élève **co-signe** (charte, engagement, productions) ;
   - **avis de l'enfant** (droit à l'image, voix, productions) : l'élève mineur à partir de 11 ans [R, S18-04] donne son avis ; **son refus l'emporte** ; son accord ne suffit pas sans les titulaires.
4. **Désaccord déclaré** au socle 4 (objet `image`, `sortie`, `general`) → `desaccord` pour les objets de cet objet ; levée seulement par pièce (accord écrit, décision de justice) ; décision du juge (373-2-6 : « sans l'autorisation de l'autre ») → règle `tous` **imposée** pour l'image de cet élève.
5. **Élève protégé** (confidentialité renforcée, changement de classe pour protection) → `protege` pour tout usage d'image hors `interne_personnels`, quels que soient les accords.

Pour un sujet **adulte** (personnel, parent, élève majeur, prospect) : `soi`.

### 7.2 Ce que voit le décideur [R]
- **Sa** décision et sa preuve ; « **enregistré** » ; jamais la décision de l'autre titulaire (corrige FD-03).
- L'état global seulement sous la forme « en attente d'une autre réponse » quand la règle est `tous` (sans dire qui) ; « l'établissement a besoin d'un accord commun : il vous contactera » en cas de désaccord.
- L'élève voit les accords qui le concernent et son propre avis.

### 7.3 Ce que voit l'établissement [R]
- Personnel avec `autorisations.lire` dans sa portée (classe, cours, sortie qu'il organise) : **verdict** par élève (« peut / ne peut pas / en attente ») ; jamais le détail des décisions ni un motif.
- Secrétariat, direction (`autorisations.gerer`) : décisions par décideur, preuves, désaccords ; **lecture journalisée** (corrige FD-09).
- Personne ne voit un commentaire libre d'un parent sauf le destinataire désigné du formulaire.

---

## 8. Objets d'accord et usages

### 8.1 Catalogue prêt (D-023), relu une fois par un juriste (Q-FC1)
| Code | Nature | Règle par défaut | Droit du lien | Validité | Usages | Notes |
|---|---|---|---|---|---|---|
| `droit_image` | consentement | `tous` (+ avis élève ≥ 11 ans) ; `un` pour `fil_classe` seul | `signer_documents` | année scolaire | `interne_personnels`*, `fil_classe`, `fil_etablissement`, `lettre_information`, `affiche_imprimee`, `ecran_hall`, `journal_papier`, `site_public`, `reseaux_etablissement`, `presse`, `video_projet`, `photo_classe_prestataire` | *`interne_personnels` (trombinoscope des personnels) est **mission** : non requis, mais `protege` s'applique |
| `voix` | consentement | idem | idem | année | `classe`, `podcast_etablissement`, `diffusion_publique` | |
| `publication_productions` | consentement | `eleve_et_titulaires` | `signer_documents` | année | mêmes canaux que l'image ; mode de signature `nom`, `prenom_initiale`, `prenom`, `anonyme` | CPI |
| `mention_nominative` | consentement | `tous` + avis | idem | année | `fil_etablissement`, `lettre`, `site_public`, `presse` | félicitations, palmarès |
| `enregistrement_visio` | consentement | `tous` + avis | idem | année | `cours_enregistre_classe` | socle 33 |
| `sortie_facultative` | autorisation | `un` | `signer_quotidien` (`autoriser_sorties`) | événement | `participation` | créé par le 42 par sortie |
| `voyage_france` | autorisation | `un` | `signer_quotidien` | événement | `participation` | |
| `voyage_etranger` | autorisation | `tous` [R] | `signer_documents` | événement | `participation` | AST : pièce du socle 7, pas un accord ScolIA |
| `transport_vehicule_accompagnateur` | autorisation | `un` | `signer_quotidien` | événement ou année | `transport` | |
| `soins_urgence` | autorisation | `un` | `signer_quotidien` | année ou événement | `transport_urgence`, `soins_urgence` | n'empêche jamais les secours ; **aucune donnée de santé** dans l'accord |
| `activite_risque` | autorisation | `un` | `signer_quotidien` | année | `participation` | escalade, natation hors programme |
| `sortie_seul` (régime) | autorisation | `un` | `autoriser_sorties` | année | `sortie_libre`, `sortie_fin_cours`, `sortie_absence_prof` | reprise de `062` avec le 14 |
| `liste_representants` | consentement | `soi` (par parent) | — | jusqu'au retrait | `liste_classe` | 17 §16 |
| `coordonnees_associations` | consentement | `soi` | — | jusqu'au retrait | `liste_parents_association` | D111-8 ; refusé par défaut |
| `coordonnees_transport` | consentement | `soi` | — | année | `transporteur` | socle 4 / 43 |
| `photo_personne_autorisee` | consentement | `soi` | — | jusqu'au retrait | `remise_accueil` | socle 4 §8.8 |
| `recontact_prospect` | consentement | `soi` | — | 1 an | `recontact` | portes ouvertes (corrige FN-02) |
| `charte_numerique` | information | `eleve_et_titulaires` | `signer_quotidien` | version | — | n'empêche pas l'accès des parents |
| `reglement_interieur` | information | `eleve_et_titulaires` | `signer_quotidien` | version | — | |
| `outils_ia_eleve` | information + opposition | `un` (opposition possible) | `signer_quotidien` | année | `assistant_eleve` | socle 30 ; à revoir avec l'étude juridique (D-032) |

L'établissement peut **créer** un objet (nature, règle dans les bornes, usages choisis parmi une liste, texte) : la règle ne peut **jamais être abaissée** sous celle du catalogue pour la même famille d'objet (« monter, jamais baisser », comme le socle 7 DCT-99) (R-FC6).

### 8.2 Versions [R, corrige FD-01, FD-14, FD-15]
- Chaque objet a des **versions de texte** ; chaque version déclare les **usages couverts**.
- Publier une version `n+1` exige de qualifier le changement :
  - **forme** (orthographe, mise en page, même portée) : les décisions de `n` **restent valables** ; la famille voit la nouvelle version, sans nouvelle demande ;
  - **fond** (usage ajouté, durée allongée, destinataire nouveau) : les **accords** de `n` restent valables **pour les seuls usages qu'ils couvraient** et passent `a_renouveler` pour les usages nouveaux ; les **refus** restent protecteurs ; nouvelle demande aux décideurs ; l'IA ne peut pas qualifier « forme » à la place de l'auteur ;
  - **restriction** (usage retiré) : décisions valables pour les usages restants ; usages retirés `retirés` dans les usages déclarés.
- Numéro strictement croissant ; un objet a **une** nature, jamais changée ; une version publiée est **immuable** (déclencheur).

---

## 9. L'état et le port « a-t-on le droit ? »

### 9.1 Calcul [R]
Pour (sujet, objet, usage, date) :
1. objet `non_requis` pour cet usage (mission) → `non_requis` (sauf `protege`) ;
2. sujet protégé et usage d'image → `protege` ;
3. décideurs à la date (§7.1) ; leurs **dernières** décisions sur une version couvrant l'usage, encore **valides** (validité, majorité intervenue après la décision : une décision des parents **cesse** à la majorité pour un consentement, R-FC12) ;
4. application de la règle ; refus de l'élève (avis) prioritaire ;
5. désaccord déclaré → `desaccord` ;
6. sinon `en_attente`, `expire`, `a_renouveler` ou `inconnu` (reprise).

Le résultat est **calculé**, jamais stocké comme vérité ; une **projection** (`ent_acc_etat`) l'accélère et se reconstruit (R-FC4).

### 9.2 Contrat d'appel [R]
- Appel en **lot** (une classe, une sortie, une image avec 12 personnes) ; une ligne par sujet.
- Raison **codée** ; le détail (qui a refusé, quand) n'est rendu qu'aux titulaires de `autorisations.gerer`.
- **Aucun appelant ne reçoit `autorise` par défaut** : en cas d'erreur, le port renvoie `inconnu` et l'appelant doit refuser l'usage (R-FC3).
- Journal des vérifications **agrégé** (qui a vérifié quel objet pour combien de sujets) ; pas de journal par élève pour un appel de routine (minimisation) ; journal nominatif pour une consultation détaillée.

---

## 10. Retrait, expiration, renouvellement

- **Retrait** : depuis l'écran « mes accords » (famille, élève, personnel), un geste par usage ou pour tout l'objet ; confirmation simple (pas de passkey : le retrait doit être **aussi simple** que le don, RF-10) ; effet **immédiat** : événement `autorisations.retire`, re-vérification des usages déclarés (§14).
- **Retrait d'une autorisation** : possible jusqu'à l'événement ; l'organisateur est prévenu ; après le départ, un « retrait » est traité comme une demande à l'établissement (on ne fait pas descendre un élève d'un car par un clic) (R-FC14).
- **Expiration** : fin d'année scolaire, date, fin de l'événement ; les usages déclarés au-delà passent en alerte.
- **Renouvellement de rentrée** (FO8) : un écran par enfant, « **reprendre mes choix de l'an dernier** » (un geste, mais explicite, avec le texte de l'année affiché), ou modifier usage par usage ; les nouveaux usages sont proposés **décochés**.
- **Majorité** intervenue : les consentements donnés par les parents pour l'élève **cessent** ; l'élève est sollicité (R-FC12) ; les autorisations pour un événement déjà accordé restent (fait acquis) sauf décision contraire de l'élève.
- **Départ de l'élève** : plus de nouvelle demande ; les usages en cours (photos publiées) sont retirés de l'affichage à la date de départ + 30 jours, sauf archive [R, S18-06].

---

## 11. Le moteur de formulaires

### 11.1 Structure [R]
Un formulaire a une **nature** (§6.1 ; pour un formulaire mixte, chaque question « accord » porte sa propre nature), une **unité de réponse**, des **versions** (questions figées à la publication), des **sections**, des **questions**, des **conditions** d'affichage, une **audience** (critères en table, mêmes types que le socle 17 §8.1, résolus sous les droits de l'auteur), une **ouverture**, une **échéance**, des **relances**.

### 11.2 Types de questions [R]
| Type | Valeur stockée | Remarques |
|---|---|---|
| `texte_court` (≤ 200) / `texte_long` (≤ 4 000) | `valeur_texte` | désactivable dans un sondage anonyme ; lint de sensibilité |
| `choix_unique` / `choix_multiple` / `liste` | `option_id` (une ligne par option choisie) | « autre » avec précision = texte court lié |
| `oui_non` | `option_id` (deux options système) | |
| `nombre` (bornes, entier ou décimal) | `valeur_nombre NUMERIC` | |
| `date` / `heure` | `valeur_date` / `valeur_heure` | |
| `echelle` (1–5, 1–10, libellés) | `valeur_nombre` | |
| `grille` (lignes × options) | une ligne par (ligne, option) via `ligne_id` | |
| `classement` | `option_id` + `rang` | |
| `fichier` | `objet_id` (socle 7, quarantaine, types bornés) | jamais dans un sondage anonyme |
| `accord` | décision d'objet d'accord (§6.2) | crée une `ent_acc_decision` dans la même transaction |
| `prise_connaissance` | S0 | |
| `creneau` | renvoi au socle 19 | |
| `choix_tarifie` | `option_id` + tarif du socle 24 | déclenche une demande de facturation (§18) |
| `information` (texte affiché, sans réponse) | — | |

**Interdits par défaut** (R-FC22) : type de question « santé », « religion », « opinion », « origine », « orientation sexuelle », « handicap », « revenus », « situation judiciaire » ; un **lint** sur le libellé (mots-clés, IA en N0) demande la classification de sensibilité de chaque question ; C3 (santé) → refus hors formulaire du socle 34 ; C2 (social, religion pour une activité facultative du privé) → autorisé seulement à la direction, avec avertissement art. 9, consentement explicite distinct, et lecture des réponses restreinte.

### 11.3 Unités de réponse [R]
| Unité | Exemple | Qui répond | Doublons |
|---|---|---|---|
| `eleve_un_titulaire` | choix d'atelier, coupon de réunion | un titulaire (ou l'élève majeur) | la première réponse vaut ; l'autre titulaire voit « déjà répondu par un responsable » sans contenu et peut demander une modification (avis au premier) [R] |
| `eleve_chaque_titulaire` | enquête de satisfaction par parent | chaque titulaire | une par titulaire |
| `foyer` | covoiturage, disponibilité pour accompagner | un membre du foyer | une par foyer |
| `personne` | sondage des personnels, des parents | la personne | une |
| `eleve_lui_meme` | sondage de classe, choix d'un sujet d'exposé | l'élève | une |
| `accord` | question `accord` | décideurs du §7 | selon la règle |

**Fratrie** : « appliquer les mêmes réponses à mes autres enfants » (chaque réponse est enregistrée **par enfant**, explicitement).

### 11.4 Remplir [R]
- Brouillon enregistré automatiquement ; envoi explicite ; **accusé** avec copie dans le coffre (socle 7) ; modification jusqu'à la clôture (nouvelle soumission, l'ancienne garde sa date) sauf question `accord` (nouvelle décision).
- **Préremplissage** depuis le dossier (socle 4) **seulement** si l'auteur l'a choisi et si le répondant a le droit de voir la donnée ; une différence saisie devient une **proposition** au socle 4 (jamais une écriture directe) (R-FC24).
- **Langue** : textes traduits relus (comme 17 §15) ; réponses dans la langue du répondant (texte libre non traduit automatiquement pour l'établissement sans geste).
- **Accessibilité** : libellés liés aux champs, ordre de tabulation, messages d'erreur explicites, aucun captcha visuel.
- **Papier** : formulaire imprimable avec code de cible ; **guichet** : un agent saisit, choisit le mode (G constaté, P papier scanné), la date de la signature papier, le décideur (choisi parmi les décideurs calculés, jamais saisi librement) ; la décision indique « saisie par X » (R-FC26).
- **Sans compte** : lien nominatif à usage limité (D-027 point C ; socle 8 §10.5), code à usage unique au canal vérifié pour une autorisation (S1).

### 11.5 Versions du formulaire [R]
Questions **figées** à la publication ; modifier après publication = nouvelle version ; les réponses restent liées à la version répondue ; une correction de forme sans changement de question est permise (libellé, aide) et tracée.

---

## 12. Sondages anonymes

### 12.1 Ce qui est garanti [R]
« **Anonyme** : l'établissement ne peut pas relier votre réponse à votre nom. Il sait seulement que vous avez participé, pour ne pas vous relancer. » — et c'est vrai :
1. **Participation** (`ent_frm_participation`) et **réponse** (`ent_frm_soumission_anonyme`) dans deux tables **sans clé commune** ; identifiants aléatoires ; aucune heure (date seulement) ; aucun compte, aucune adresse IP ni appareil conservés avec la réponse.
2. **Insertion différée et mélangée** : les réponses sont gardées dans une file chiffrée en mémoire de service et écrites **par lots** d'au moins 10 (ou à la clôture), dans un ordre aléatoire, pour casser la corrélation temporelle avec la participation ; les journaux applicatifs ne contiennent ni le contenu ni l'identifiant de la soumission (R-FC29).
3. **Seuil** : aucun résultat affiché (ni total, ni ventilation) en dessous de **5** répondants par case (S18-07) ; ventilations par classe ou par sous-groupe seulement si chaque cellule ≥ 5 ; les combinaisons de filtres sont bornées.
4. **Texte libre** : désactivé par défaut ; activable avec avertissement (« une réponse écrite peut vous identifier ») ; jamais restitué tel quel en dessous de 5 réponses ; l'IA peut proposer une synthèse **au-dessus du seuil seulement**, sans citation.
5. **Pas de question d'identification** (classe, âge) plus fine que la ventilation autorisée.
6. Un sondage anonyme **ne peut pas** devenir nominatif ; l'inverse non plus.
7. **Limite assumée** : un administrateur de la plateforme qui observe les écritures en temps réel pourrait corréler ; le lot réduit le risque ; écrit dans la notice [R, Q-FC6].

### 12.2 Pseudonyme et nominatif [R]
- **Nominatif annoncé** : le nom est visible de l'auteur ; c'est dit en tête.
- **Confidentiel** : nominatif pour un destinataire désigné seulement (CPE, infirmière hors santé, psychologue — socle 36 si suivi), jamais pour l'auteur s'il est autre.

### 12.3 Ce qu'un sondage n'est pas
Ni un scrutin (RF-18), ni un vote d'instance (46), ni une enquête officielle du ministère (outils propres) ; libellé imposé « consultation, sans valeur de scrutin » pour tout sondage de type « choix » (R-FC31).

---

## 13. Données sensibles et santé

- Classification par question : **C0** (anodin), **C1** (personnel), **C2** (sensible : social, religion pour une activité facultative, situation familiale), **C3** (santé, handicap) — cohérente avec le socle 8.
- **C3** : uniquement par un **formulaire de santé du socle 34** (questionnaire de voyage, fiche d'urgence confidentielle sous pli), stocké dans le domaine restreint du 34, lu par l'infirmerie seulement ; **désactivé** tant que le module santé l'est (D-034) — en attendant, la fiche sanitaire reste papier sous pli (pratique actuelle, `42_SORTIES_VOYAGES.md` §5.10).
- **`soins_urgence`** est une **autorisation** sans aucune donnée de santé (« j'autorise l'établissement à faire appel aux secours et à faire transporter mon enfant »).
- L'IA ne lit **jamais** une réponse C3 ni une réponse C2 nominative (R-FC33).

---

## 14. Cohérence accord ↔ usage (A15)

### 14.1 Mécanique [R]
1. Chaque socle consommateur **déclare** ses usages effectifs (`declarer_usage`) : le 17 pour chaque image et production publiée (personne, usage, publication), le 42 pour chaque participant (autorisation), le 4 pour chaque transmission de liste à une association, le 2 pour l'affichage d'une photo hors personnels, le 33 pour un enregistrement.
2. Sur tout événement qui peut changer un verdict — `autorisations.retire`, nouvelle décision, désaccord déclaré, levée, majorité, décès, protection, départ, expiration, version de fond — le 18 **re-vérifie** les usages déclarés concernés et publie `autorisations.etat_modifie` (identifiants seulement).
3. Le consommateur **réagit** : le 17 masque l'image (< 1 minute, R-PB26), le 42 marque l'exigence non remplie et alerte l'organisateur, le 4 retire la personne de la prochaine transmission et alerte le secrétariat qu'une liste déjà transmise contient une personne qui a retiré son accord (l'association est informée de la retirer, sans motif).
4. **Contrôle nocturne** de rattrapage : tout usage déclaré dont le verdict n'est plus `autorise` ou `non_requis` → alerte (`autorisations.incoherence`) à l'auteur de l'usage et au secrétariat ; tableau de cohérence (FRM-215).

### 14.2 Tableau de cohérence [R, différenciant]
Par objet : usages en cours, verdicts, incohérences ouvertes, délai moyen de traitement ; pour le DPO : nombre de retraits propagés, délai de propagation.

---

## 15. Résultats et exports

- **Nominatif** : tableau par cible (réponses, état, date, canal) pour l'auteur et les habilités ; filtre « non répondu » ; relance des seuls non-répondants.
- **Agrégé** : graphiques par question ; ventilation par classe et niveau (seuils au §12 pour l'anonyme).
- **Exports** CSV / tableur : journalisés (acteur, formulaire, colonnes, nombre de lignes) ; **neutralisation des formules** (`=`, `+`, `-`, `@`) ; colonnes C2 exclues sauf droit ; aucun export d'un sondage anonyme sous le seuil ; un professeur exporte les réponses de **ses** formulaires à **ses** cours (ce n'est pas un export officiel au sens de D-022) [R, Q-FC9].
- **Publier les résultats** aux répondants (« les résultats du sondage sur les horaires du CDI ») : agrégats seulement, au-dessus du seuil, par une publication du 17.
- **Preuve** : dossier de preuve d'un accord (socle 7) téléchargeable par l'établissement et par le décideur.

---

## 16. Personnels

- Sujets de leurs propres accords (`soi`) : image, voix, mention nominative, coordonnées professionnelles dans un annuaire visible des familles [R].
- **Formulaires internes** : vœux de service, inscription à une formation, sondage de climat (le « pouls » du socle 6, `ent_pouls_*`, utilise ce moteur en mode anonyme), choix de dates.
- **Aucun consentement demandé** pour ce qui relève du contrat de travail ou de la mission (planning, coordonnées d'urgence internes) (R-FC1).
- Notes de service à prise de connaissance : socle 17 (§17 du 17).

---

## 17. Rebranchements

- **Socle 3** (préinscription) et **socle 4** (fiche de rentrée) : gardent leurs parcours ; leurs questions passent sur le moteur (types, conditions, langues) au lot FO7 [R] ; les accords qu'ils recueillent (image, coordonnées, soins) **sont** des décisions du 18 (une seule source).
- **Socle 4** : `ent_personne_consentement` (269/275) repris en décisions d'objets `coordonnees_associations`, `coordonnees_transport`, `photo_personne_autorisee` (reprise additive, FO1), puis lecture par le port.
- **Socle 14** : `062_sortie_autorisation` (régime de sortie écrasé) repris en objet `sortie_seul` avec historique (FO7) ; le 14 lit le verdict.
- **Socle 42** : une campagne d'autorisations par sortie (VOY-171) = un formulaire du 18 avec questions `accord` (participation, image, soins) + éventuelle question `choix_tarifie` ; le 42 lit `etat_cibles` et `verifier` ; **reprise de Next.js** : autorisations papier attestées (R-SV39) → décisions `P` « attestée à la bascule par X », droit à l'image Next.js → `inconnu` (R-SV40).
- **Socle 2** : trombinoscope et photo : port `verifier(droit_image, interne_personnels | fil_classe)` ; la photo d'un élève protégé n'est montrée qu'aux personnels habilités.

---

## 18. Formulaires avec paiement

Une question `choix_tarifie` (photos de classe, cotisation d'association sportive, participation à une sortie) **n'encaisse rien** : elle crée, à la soumission, une **demande** au socle 24 (facture ou ligne) au **payeur** du socle 4 (qui peut être un autre que le répondant) ; pendant l'année de doublon (D-034), mode **suivi** : la demande est enregistrée et exportée, sans facture (comme R-SV44) [R]. Le tarif vient du 24, jamais d'un nombre libre du formulaire.

---

## 19. Cas limites (catalogue)

### 19.1 Familles et décideurs
| # | Situation | Ce qui doit se passer [R] |
|---|---|---|
| CK-01 | Sortie facultative, règle `un` : la mère accepte, le père ne répond pas | `autorise` ; le père est informé et peut s'opposer jusqu'au départ |
| CK-02 | Même cas, le père refuse ensuite | `desaccord` ; organisateur et direction alertés (sans motif) ; les deux titulaires reçoivent un message neutre |
| CK-03 | Droit à l'image (règle `tous`) : un seul accord | `en_attente` ; relance au second ; l'image n'est pas publiable |
| CK-04 | Les deux parents acceptent, l'élève de 13 ans refuse | `refuse` (avis de l'enfant prioritaire) |
| CK-05 | L'élève de 9 ans (premier degré, socle 50) | avis non demandé (S18-04 : 11 ans) |
| CK-06 | Parent sous interdiction de communication | ne lit ni ne signe ; n'est pas compté parmi les décideurs ; la règle `tous` s'applique aux autres titulaires [NV, Q-FC2] |
| CK-07 | Parent sans autorité parentale | n'est pas décideur ; peut être informé (droit `recevoir_informations`) |
| CK-08 | Parent à qui la direction a retiré `autoriser_sorties` | ne signe pas les autorisations de sortie ; signe les consentements s'il a `signer_documents` |
| CK-09 | Décision du juge : interdiction de diffuser sans l'accord de l'autre (373-2-6) | règle `tous` imposée pour l'image de cet élève ; pièce au socle 4 |
| CK-10 | Désaccord déclaré « image » | `desaccord` sur tous les usages d'image ; levée par pièce |
| CK-11 | Un parent décède | retiré des décideurs ; `tous` porte sur le survivant |
| CK-12 | Élève placé (ASE), service gardien | le service signe `signer_quotidien` ; les actes non usuels restent aux titulaires ou au juge [NV] |
| CK-13 | Famille d'accueil | signe ce que le lien permet ; jamais un consentement d'image hors mandat |
| CK-14 | Tuteur d'un mineur | titulaire de fait (qualité `tuteur`) |
| CK-15 | Majeur sous tutelle | le tuteur décide, l'élève est informé et donne son avis |
| CK-16 | Majeur sous curatelle | l'élève décide ; pour un acte qui l'exige, avec le curateur [NV] |
| CK-17 | Mineur émancipé | décide seul |
| CK-18 | Date de naissance contestée | majorité non présumée ; décision des titulaires ; alerte |
| CK-19 | Élève qui devient majeur pendant l'année | consentements parentaux cessent ; l'élève est sollicité ; autorisations déjà accordées pour un événement maintenues |
| CK-20 | Élève majeur qui veut que ses parents restent informés | réglage du socle 2/4 ; il reste seul décideur |
| CK-21 | Fratrie : une réponse pour trois enfants | trois décisions explicites |
| CK-22 | Parent avec deux enfants dans deux établissements | deux décisions séparées (D-013) |
| CK-23 | Beau-parent qui répond | refusé s'il n'a pas le droit du lien ; message « seul un responsable légal peut répondre » |
| CK-24 | Grand-parent qui vient au guichet avec le coupon signé du père | saisie au guichet au nom du père (décideur calculé), mode P, pièce scannée |
| CK-25 | Coupon papier signé d'un nom illisible | l'agent choisit le décideur parmi les titulaires ; s'il ne peut pas : `en_attente` |
| CK-26 | Parents qui envoient chacun un coupon contradictoire | deux décisions ; la règle tranche (`desaccord` pour `un`, refus pour `tous`) |
| CK-27 | Parent qui veut répondre au nom de l'autre | impossible ; « chaque responsable répond pour lui-même » |
| CK-28 | Famille allophone | texte traduit relu, avec la version française qui fait foi |
| CK-29 | Parent illettré au guichet | lecture par l'agent, mode G, assistance tracée (socle 7 DCT-142) |

### 19.2 Versions, validité, retrait
| # | Situation | Ce qui doit se passer [R] |
|---|---|---|
| CK-30 | v2 du droit à l'image ajoute « réseaux sociaux » | accords v1 valables pour les usages v1, `a_renouveler` pour « réseaux » ; refus v1 maintenus |
| CK-31 | v2 corrige une faute | forme : rien à refaire |
| CK-32 | Auteur qui déclare « forme » une extension d'usage | impossible : un usage ajouté force « fond » (contrôle automatique) |
| CK-33 | Retrait d'accord en mai, photo dans la lettre de mars | lettre archivée : image retirée des affichages, archive accessible aux habilités ; le PDF déjà imprimé ne peut être rappelé (dit à la famille) |
| CK-34 | Retrait d'autorisation la veille du départ | organisateur alerté ; l'élève ne part pas sauf nouvel accord |
| CK-35 | « Retrait » le jour du voyage, élève dans le car | demande à l'établissement (R-FC14), pas d'effet automatique |
| CK-36 | Accord expiré au 31 août, photo de juin dans le fil jusqu'en septembre | usage au-delà de la validité → alerte ; renouvellement de rentrée |
| CK-37 | Fin de scolarité | plus de nouvelle demande ; usages retirés à départ + 30 jours |
| CK-38 | Accord donné par erreur (mauvais enfant) | nouvelle décision ; l'erreur reste dans l'historique ; signalement au secrétariat |
| CK-39 | Établissement qui veut « annuler » une réponse | jamais d'effacement : décision `annulee_par_etablissement` avec motif (ex. saisie au mauvais nom), visible du décideur |
| CK-40 | Rentrée : « comme l'an dernier » | un geste, texte de l'année affiché, nouveaux usages décochés |

### 19.3 Formulaires et sondages
| # | Situation | Ce qui doit se passer [R] |
|---|---|---|
| CK-41 | Sondage anonyme à une classe de 8 élèves, 4 répondent | aucun résultat avant 5 |
| CK-42 | Sondage anonyme avec ventilation par sexe dans une classe de 25 | cellule < 5 → masquée |
| CK-43 | Réponse libre « je suis le seul élève en fauteuil… » | texte libre désactivé par défaut ; sinon synthèse seulement au-dessus du seuil |
| CK-44 | Auteur qui veut rendre nominatif un sondage anonyme | impossible |
| CK-45 | Formulaire avec question « avez-vous une religion ? » | refusé (C2) sauf direction et finalité facultative déclarée, consentement explicite distinct |
| CK-46 | Question « allergies de votre enfant ? » dans un formulaire de sortie | refusée : renvoi au formulaire de santé du 34 (désactivé sans HDS) |
| CK-47 | Professeur qui sonde sa classe sur un sujet d'exposé | unité `eleve_lui_meme`, audience = son cours |
| CK-48 | Professeur qui vise une autre classe | refusé (portée) |
| CK-49 | Formulaire à 40 questions | avertissement de longueur ; sections ; reprise du brouillon |
| CK-50 | Réponse modifiée après la clôture | refusée ; demande à l'auteur |
| CK-51 | Choix d'atelier : places limitées | quota par option ; liste d'attente ; ordre d'arrivée ou tirage déclaré à l'avance [R] |
| CK-52 | Tirage au sort | aléa côté serveur, graine journalisée, résultat reproductible |
| CK-53 | Fichier joint (photo d'un justificatif) | socle 7 : quarantaine, types bornés |
| CK-54 | Choix tarifé, payeur ≠ répondant | demande au payeur (24) |
| CK-55 | Doublon de réponse par deux titulaires en unité `eleve_un_titulaire` | la première vaut ; le second voit « déjà répondu », sans contenu |
| CK-56 | Élève qui répond au formulaire des parents avec le compte d'un parent | impossible de le détecter sûrement ; défense : session courte, S1 par code au mobile vérifié du parent pour une autorisation (R-FC17) |
| CK-57 | Lien nominatif transféré à un tiers | à usage limité, lié à une personne, code au canal vérifié pour signer |
| CK-58 | Export avec un champ « =HYPERLINK(…) » saisi par une famille | neutralisé |
| CK-59 | Sondage présenté comme « vote » du CVL | libellé « consultation, sans valeur de scrutin » ; vote : socle 46 |
| CK-60 | Enquête de climat des personnels (petite équipe de 6) | seuil 5 ; ventilation interdite |

### 19.4 Usages et cohérence
| # | Situation | Ce qui doit se passer [R] |
|---|---|---|
| CK-61 | Photo de classe pour le photographe prestataire | usage `photo_classe_prestataire` ; contrat de sous-traitance (DPO) |
| CK-62 | Trombinoscope des personnels sans accord | `non_requis` (mission) ; élève protégé : personnels habilités seulement |
| CK-63 | Trombinoscope visible des familles | usage `fil_classe` exigé [R] |
| CK-64 | Liste des parents transmise à l'APEL le 1ᵉʳ octobre ; un parent retire son accord le 5 | alerte au secrétariat ; information neutre à l'association de le retirer |
| CK-65 | Enregistrement d'un cours en visio avec un élève sans accord | caméra et micro de l'élève coupés par défaut dans l'enregistrement, ou enregistrement refusé (socle 33) |
| CK-66 | Élève protégé sur une photo de groupe | `protege` : publication refusée |
| CK-67 | Usage déclaré sans accord (erreur d'un socle) | contrôle nocturne → alerte |
| CK-68 | Port `verifier` en panne | `inconnu` → l'appelant refuse l'usage |
| CK-69 | Reprise Next.js : « droit à l'image OK » | `inconnu` (R-SV40) |
| CK-70 | Reprise Next.js : autorisation papier rangée dans un classeur | attestée par l'organisateur (mode P « attestée à la bascule ») |
| CK-71 | Reprise de `062` (régime de sortie) | décision historique « reprise sans preuve », à renouveler à la rentrée suivante |
| CK-72 | Décès d'un élève | gel ; usages retirés ; aucune relance |
| CK-73 | Parent qui demande l'historique de ses accords | écran « mes accords » + dossier de preuve |
| CK-74 | Parent qui exerce son droit d'opposition (art. 21) à un traitement fondé sur la mission | demande au DPO (socle 27), pas un « retrait » |
| CK-75 | Changement de responsable légal en cours d'année (jugement) | décideurs recalculés à la date ; les accords de l'ancien titulaire valent jusqu'à la date du jugement, puis `a_renouveler` pour un consentement [R] |
| CK-76 | Bascule d'année | autorisations d'événement closes ; consentements « année » expirés ; campagne de renouvellement |

---

## 20. Intelligence artificielle

### 20.1 Ce que l'IA fait [R]
- **Propose** un formulaire à partir d'une description (« coupon pour la sortie au musée avec choix du pique-nique ») : questions, types, conditions — N1 (brouillon).
- **Classe** la sensibilité des questions (lint) et **suggère la nature juridique** (« ceci est une collecte, pas un consentement ») — N0 ; la décision reste à l'auteur.
- **Traduit** et propose une version **FALC** du texte d'un accord — N0, relue.
- **Synthétise** les réponses libres d'un formulaire **nominatif** pour son auteur, ou d'un anonyme **au-dessus du seuil**, sans citation — N0.
- **Prépare** la liste de relance des non-répondants — N1 ; la relance est N3.
- Côté famille : **explique** un accord en langage simple (« ce que vous acceptez, ce que vous pouvez retirer ») à partir du texte officiel — N0.

### 20.2 Ce que l'IA ne fait pas [R]
Elle ne décide **jamais** d'un accord, ne répond **jamais** à la place d'un décideur, ne déduit **jamais** un consentement, ne qualifie pas « forme » une version, ne lit **aucune** réponse C3 ni C2 nominative, ne voit **aucune** réponse anonyme sous le seuil, ne publie ni ne relance (N3).

### 20.3 Capacités déclarées [R]
| Capacité | Type | Droit | Niveau |
|---|---|---|---|
| `formulaires.proposer` | commande | `formulaires.creer` | N1 |
| `formulaires.classer_sensibilite` | lecture | idem | N0 |
| `formulaires.synthetiser_reponses` | lecture | auteur | N0 (seuil) |
| `autorisations.verdicts_lire` | lecture | `autorisations.lire` (portée) | — |
| `autorisations.expliquer` | lecture | décideur | N0 |
| `formulaires.preparer_relance` | commande | auteur | N3 (relance) |

---

## 21. Actions métier (FRM-100 à FRM-279)

Abréviations du socle 17 ; DPO délégué à la protection des données ; INF infirmerie ; ORG organisateur de sortie. ★ = N3. FRM-01 à FRM-05 (`SOCLE2_DOMAINES.md` D24) → FRM-120, FRM-150, FRM-100, FRM-170, FRM-195.

### 21.1 Objets d'accord
| ID | Action | Qui · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| FRM-100 | Activer un objet du catalogue (image, voix, productions, coordonnées…) | DIR · `autorisations.parametrer` | texte de départ relu | `autorisations.objet_active` | FO1 |
| FRM-101 | Créer un objet propre (nature, règle ≥ catalogue, usages, validité) | DIR | « monter, jamais baisser » (R-FC6) | idem | FO1 |
| FRM-102 | Rédiger une version de texte et ses usages couverts | DIR, ADF | texte ≤ 10 000, FALC proposé | — | FO1 |
| FRM-103 | Publier une version (forme / fond / restriction) ★ | DIR | §8.2 ; usage ajouté ⇒ fond | `autorisations.version_publiee` | FO0–FO1 |
| FRM-104 | Voir l'historique des versions et ce que chacune couvrait | DIR, ADF, DPO | — | — | FO1 |
| FRM-105 | Choisir la preuve exigée (S0, S1, S2) | DIR | jamais sous le minimum de la nature | — | FO1 |
| FRM-106 | Fixer l'âge de l'avis de l'élève | DIR | défaut 11 ans (S18-04) | — | FO1 |
| FRM-107 | Désactiver un objet (fin d'usage) | DIR | décisions conservées ; usages retirés | `autorisations.objet_desactive` | FO1 |
| FRM-108 | Traduire le texte (relu) | ADF | langues des familles | — | FO8 |
| FRM-109 | Voir le texte que voit un décideur (aperçu par profil) | DIR | — | — | FO1 |
| FRM-110 | Exporter le catalogue pour relecture du juriste | DIR, DPO | — | — | FO13 |
| FRM-111 | Lier un objet à un désaccord du socle 4 (`image`, `sortie`) | SYS | correspondance fixe | — | FO3 |
| FRM-112 | Créer un objet par événement (sortie) depuis un autre socle | SYS · `creer_pour` | au nom de l'utilisateur | — | FO3 |
| FRM-113 | Décider pour soi (personnel, parent, majeur) | la personne | `soi` | `autorisations.decision` | FO1 |
| FRM-114 | Décider pour un enfant | PAR (décideur calculé) | §7 ; S1 | `autorisations.decision` | FO1 |
| FRM-115 | Donner son avis sur son image (élève mineur) | ELV | refus prioritaire | `autorisations.decision` | FO1 |
| FRM-116 | Retirer un accord, usage par usage | décideur | aussi simple que le don | `autorisations.retire` | FO1 |
| FRM-117 | Voir « mes accords » par enfant (état, version, date, preuve) | PAR, ELV, PER | jamais la décision de l'autre | — | FO1 |
| FRM-118 | Télécharger le dossier de preuve d'une décision | décideur, ADF | socle 7 | — | FO3 |
| FRM-119 | Signaler une erreur sur un accord | décideur | message au secrétariat | — | FO1 |

### 21.2 Créer un formulaire
| ID | Action | Qui · droit | Règles | Lot |
|---|---|---|---|---|
| FRM-120 | Créer un formulaire (nature, unité de réponse, audience) | DIR, ADF, CPE, PP, PRF · `formulaires.creer` (portée) | critères dans la portée | FO2 |
| FRM-121 | Partir d'un modèle prêt (coupon de réunion, choix d'atelier, enquête de satisfaction, sortie, invitation) | auteur | D-023 | FO2 |
| FRM-122 | Ajouter sections et questions (types du §11.2) | auteur | limites de longueur | FO2 |
| FRM-123 | Ajouter des conditions d'affichage | auteur | sans boucle (vérifié) | FO2 |
| FRM-124 | Ajouter une question « accord » (objet existant) | auteur | règle de l'objet | FO3 |
| FRM-125 | Classer la sensibilité de chaque question (lint) | auteur, SYS | C3 refusé ; C2 réservé | FO2 |
| FRM-126 | Choisir nominatif, confidentiel ou anonyme | auteur | anonyme : contraintes du §12 | FO2, FO4 |
| FRM-127 | Préremplir depuis le dossier (champ par champ) | auteur | droit du répondant (R-FC24) | FO7 |
| FRM-128 | Fixer quotas et liste d'attente par option | auteur | CK-51 | FO2 |
| FRM-129 | Déclarer un tirage au sort | auteur | graine journalisée | FO2 |
| FRM-130 | Fixer ouverture, échéance, relances | auteur | J-7, J-3, J-1 par défaut | FO2 |
| FRM-131 | Prévisualiser comme un répondant (par profil : parent, élève, majeur) | auteur | — | FO2 |
| FRM-132 | Prévisualiser l'audience (nombres, exclus fondus) | auteur | comme 17 §8.3 | FO2 |
| FRM-133 | Publier ★ | auteur (validation selon seuil) | questions figées ; aperçu expiré refusé | FO2 |
| FRM-134 | Créer une nouvelle version après publication | auteur | réponses liées à leur version | FO2 |
| FRM-135 | Corriger un libellé sans changer la question | auteur | tracé | FO2 |
| FRM-136 | Dupliquer un formulaire | auteur | audience recalculée | FO2 |
| FRM-137 | Prolonger, clore, rouvrir | auteur | tracé | FO2 |
| FRM-138 | Archiver | auteur, DIR | — | FO2 |
| FRM-139 | Co-auteurs | auteur | — | FO2 |
| FRM-140 | Désigner un destinataire confidentiel des réponses | auteur | §12.2 | FO2 |
| FRM-141 | Joindre un document d'information (socle 7) | auteur | — | FO2 |
| FRM-142 | Proposer le formulaire par l'IA | auteur | N1 | FO11 |
| FRM-143 | Choisir une trame d'impression | auteur | D-023 | FO2 |
| FRM-144 | Lier à une publication du 17 (campagne) | auteur | attente | FO2 |
| FRM-145 | Valider un formulaire à large audience ★ | DIR · `formulaires.valider` | quatre yeux au-delà du seuil | FO2 |
| FRM-146 | Choix tarifé lié à un tarif du 24 | ADF | §18 | FO9 |
| FRM-147 | Question « créneau » (socle 19) | auteur | — | FO9 |
| FRM-148 | Formulaire de santé (moteur pour le socle 34) | INF · `sante.*` | **désactivé** sans HDS | FO12 |
| FRM-149 | Supprimer un brouillon jamais publié | auteur | seule suppression permise | FO2 |

### 21.3 Répondre
| ID | Action | Qui | Règles | Lot |
|---|---|---|---|---|
| FRM-150 | Voir « ce qu'on me demande » par enfant | PAR, ELV, PER | portail 29 | FO2 |
| FRM-151 | Remplir, enregistrer le brouillon, envoyer | répondant | accusé et copie au coffre | FO2 |
| FRM-152 | Appliquer les mêmes réponses à ses autres enfants | PAR | décisions explicites par enfant | FO2 |
| FRM-153 | Modifier jusqu'à la clôture | répondant | nouvelle soumission | FO2 |
| FRM-154 | Répondre sur mobile | répondant | socle 11 | FO10 |
| FRM-155 | Répondre dans sa langue, en version simple | PAR | traduction relue | FO8 |
| FRM-156 | Voir « déjà répondu par un responsable » (unité un titulaire) | PAR | sans contenu | FO2 |
| FRM-157 | Demander la modification d'une réponse de l'autre titulaire | PAR | avis au premier, sans contenu | FO2 |
| FRM-158 | Joindre un fichier | répondant | quarantaine | FO2 |
| FRM-159 | Signer par code à usage unique (S1) | décideur | canal vérifié | FO3 |
| FRM-160 | Signer par passkey (S2) pour un acte non usuel | décideur | socle 7 | FO3 |
| FRM-161 | Répondre par lien nominatif sans compte | décideur sans compte | D-027 C | FO3 |
| FRM-162 | Voir sa réponse et sa preuve | répondant | — | FO2 |
| FRM-163 | Imprimer sa réponse | répondant | — | FO2 |
| FRM-164 | « Reprendre mes choix de l'an dernier » | PAR | §10 | FO8 |
| FRM-165 | Poser une question à l'auteur | répondant | fil 16 | FO2 |
| FRM-166 | Être informé qu'un autre titulaire a autorisé (règle `un`) et pouvoir s'opposer | PAR | RF-8 | FO3 |
| FRM-167 | S'opposer (refus après une autorisation de l'autre) | PAR | `desaccord` | FO3 |
| FRM-168 | Voir pourquoi on ne peut pas répondre (« seul un responsable légal… ») | répondant | sans fuite | FO2 |
| FRM-169 | Recevoir le résultat d'un tirage ou d'un quota | répondant | — | FO2 |
| FRM-170 | Prendre connaissance d'un règlement ou d'une charte (S0) | ELV, PAR | version | FO2 |
| FRM-171 | Co-signer (élève et titulaires) | ELV, PAR | `eleve_et_titulaires` | FO3 |
| FRM-172 | Répondre à un sondage anonyme | répondant | §12 | FO4 |
| FRM-173 | Voir les résultats publiés | répondants | agrégats au-dessus du seuil | FO6 |
| FRM-174 | Élève majeur : décider seul et être prévenu directement | ELV majeur | corrige FD-04 | FO0 |
| FRM-175 | Tuteur, curateur, service gardien : décider selon le mandat | tiers légal | §7.1 | FO3 |
| FRM-176 | Lecture vocale et navigation clavier | répondant | accessibilité | FO2 |
| FRM-177 | Reprendre plus tard (brouillon sur un autre appareil) | répondant | — | FO2 |
| FRM-178 | Voir les échéances de ses formulaires | répondant | — | FO2 |
| FRM-179 | Désactiver ses rappels non obligatoires | répondant | préférences du 8 | FO2 |

### 21.4 Guichet et papier
| ID | Action | Qui | Règles | Lot |
|---|---|---|---|---|
| FRM-180 | Saisir une réponse papier (décideur choisi, date de signature, scan) | ADF · `formulaires.guichet` | mode P ; « saisie par » | FO3 |
| FRM-181 | Faire signer au guichet (tablette) | ADF | mode G, passkey de l'agent | FO3 |
| FRM-182 | Imprimer les formulaires nominatifs d'une classe | ADF | code de cible | FO3 |
| FRM-183 | Lire un coupon par son code (douchette ou caméra) | ADF | — | FO3 |
| FRM-184 | Valider un papier déposé (« signature présente, version identique ») | ADF | contrôle humain | FO3 |
| FRM-185 | Attester des autorisations papier antérieures (bascule) | ORG, ADF | R-SV39, mode P « attestée » | FO3 |
| FRM-186 | Annuler une saisie erronée (motif) | ADF | `annulee_par_etablissement` | FO3 |
| FRM-187 | Saisie assistée (interprète, lecture) | ADF | tracée | FO3 |
| FRM-188 | Liste « papiers non revenus » par classe | PP, ADF | — | FO3 |
| FRM-189 | Remettre au PP les coupons à distribuer | PP | émargement facultatif | FO3 |

### 21.5 Suivi, relances, résultats
| ID | Action | Qui | Règles | Lot |
|---|---|---|---|---|
| FRM-190 | Tableau par cible (état, date, canal) | auteur, habilités | — | FO2 |
| FRM-191 | Relancer les seuls non-répondants ★ | auteur | plage calme, exclusions | FO2 |
| FRM-192 | Relances automatiques | SYS | S18-02 | FO2 |
| FRM-193 | Voir les verdicts de sa classe ou de sa sortie (« peut / ne peut pas ») | PRF, PP, ORG · `autorisations.lire` (portée) | sans motif | FO1 |
| FRM-194 | Voir les décisions détaillées (décideur, date, preuve) | ADF, DIR · `autorisations.gerer` | lecture journalisée | FO1 |
| FRM-195 | Résultats agrégés par question | auteur | seuils si anonyme | FO6 |
| FRM-196 | Ventiler par classe, niveau | auteur | cellules ≥ 5 si anonyme | FO6 |
| FRM-197 | Exporter (CSV, tableur) | auteur, DIR | journalisé, formules neutralisées, C2 exclues | FO6 |
| FRM-198 | Publier les résultats (publication 17) | auteur | agrégats | FO6 |
| FRM-199 | Synthèse des réponses libres (IA) | auteur | N0, seuil | FO11 |
| FRM-200 | Alerte « désaccords à traiter » | DIR | sans motif à l'organisateur | FO3 |
| FRM-201 | Traiter un désaccord (contacter, consigner, lever sur pièce) | DIR | levée par pièce au socle 4 | FO3 |
| FRM-202 | Voir les familles qui ne répondent jamais | PP, ADF | signal interne | FO6 |
| FRM-203 | Comparer avec l'an dernier (taux de réponse) | DIR | agrégats | FO6 |
| FRM-204 | Liste d'émargement d'un événement depuis les réponses | ADF | — | FO6 |
| FRM-205 | Imprimer la liste « autorisés » d'une sortie (encadrant) | ORG | verdicts seulement | FO3 |
| FRM-206 | Fiche « accords de l'élève » pour l'encadrant (hors ligne, socle 42) | ORG | chiffrée, effacée au retour | FO3 |
| FRM-207 | Indicateurs (pilotage 47) | SYS | sans nom | FO6 |
| FRM-208 | Rapport de reprise (bascule) | ADF | `inconnu` listés | FO3 |
| FRM-209 | Relance ciblée « image à renouveler » à la rentrée | ADF ★ | — | FO8 |

### 21.6 Port et cohérence
| ID | Action | Qui | Règles | Lot |
|---|---|---|---|---|
| FRM-210 | Vérifier (port `verifier`) | socles consommateurs | §9.2 | FO1 |
| FRM-211 | Déclarer un usage | socles consommateurs | §14 | FO5 |
| FRM-212 | Retirer un usage | socles consommateurs | — | FO5 |
| FRM-213 | Re-vérifier les usages sur un fait | SYS | < 1 minute | FO5 |
| FRM-214 | Contrôle nocturne | SYS | alerte | FO5 |
| FRM-215 | Tableau de cohérence | ADF, DIR, DPO | §14.2 | FO5 |
| FRM-216 | Traiter une incohérence (retirer, demander, justifier) | auteur de l'usage, ADF | tracé | FO5 |
| FRM-217 | Voir où un accord est utilisé (« votre enfant apparaît dans… ») | décideur | ports 17, 42, 4 | FO5 |
| FRM-218 | Informer l'association d'un retrait (sans motif) | ADF | CK-64 | FO5 |
| FRM-219 | Bloquer l'enregistrement d'un cours si un participant n'a pas d'accord | SYS (33) | CK-65 | FO5 |
| FRM-220 | Masquer la photo d'un élève protégé hors personnels habilités | SYS (2) | — | FO1 |
| FRM-221 | Trombinoscope des familles conditionné | SYS (2) | CK-63 | FO1 |
| FRM-222 | Exigences d'une sortie à jour (socle 42) | SYS | `etat_modifie` | FO3 |
| FRM-223 | Transmission aux associations limitée aux accords valides | SYS (4) | D111-8 | FO5 |
| FRM-224 | Liste « parents ayant accepté » des représentants | SYS (17) | — | FO5 |

### 21.7 Sondages anonymes
| ID | Action | Qui | Règles | Lot |
|---|---|---|---|---|
| FRM-225 | Créer un sondage anonyme | auteur | §12 | FO4 |
| FRM-226 | Mettre en file les réponses, écrire par lots mélangés | SYS | ≥ 10 ou clôture | FO4 |
| FRM-227 | Enregistrer la participation sans lien | SYS | date seulement | FO4 |
| FRM-228 | Afficher les résultats au-dessus du seuil | SYS | 5 par case | FO4 |
| FRM-229 | Activer le texte libre (avertissement) | auteur | — | FO4 |
| FRM-230 | Relancer les non-participants | auteur ★ | participation seule | FO4 |
| FRM-231 | Clore et écrire le dernier lot | SYS | — | FO4 |
| FRM-232 | Notice d'anonymat (ce qui est garanti, la limite) | SYS | §12.1 | FO4 |
| FRM-233 | Climat scolaire des élèves (modèle prêt) | CPE, DIR | anonyme | FO4 |
| FRM-234 | Pouls des personnels (socle 6) sur le moteur | DIR | anonyme | FO4 |

### 21.8 Personnels, prospects, tiers
| ID | Action | Qui | Règles | Lot |
|---|---|---|---|---|
| FRM-235 | Accords d'un personnel sur lui-même | PER | `soi` | FO1 |
| FRM-236 | Formulaires internes (vœux, formations, dates) | DIR, ADF | audience personnels | FO2 |
| FRM-237 | Accords d'un intervenant extérieur (image) par lien nominatif | ADF | sans compte | FO3 |
| FRM-238 | Accord de recontact d'un prospect (portes ouvertes) | prospect | case non cochée (corrige FN-02) | FO7 |
| FRM-239 | Accords d'un parent pour lui-même (image lors d'une fête) | PAR | `soi` | FO1 |

### 21.9 Rebranchements et reprise
| ID | Action | Qui | Règles | Lot |
|---|---|---|---|---|
| FRM-240 | Reprendre les réponses de 193 en décisions (avec version réelle) | SYS | §28 | FO1 |
| FRM-241 | Reprendre `ent_personne_consentement` (269/275) | SYS | additive | FO1 |
| FRM-242 | Reprendre `062` en objet `sortie_seul` | SYS | historique « reprise sans preuve » | FO7 |
| FRM-243 | Importer les autorisations Next.js attestées | SYS | R-SV39 | FO3 |
| FRM-244 | Reprendre le droit à l'image Next.js en `inconnu` | SYS | R-SV40 | FO3 |
| FRM-245 | Questions de la fiche de rentrée sur le moteur | SYS | socle 4 | FO7 |
| FRM-246 | Questions de la préinscription sur le moteur | SYS | socle 3 | FO7 |
| FRM-247 | Double écriture vers 193 pendant une version | SYS | — | FO1 |
| FRM-248 | Campagne d'autorisations d'une sortie (42) | ORG | un formulaire, accords distincts | FO3 |
| FRM-249 | Formulaire d'inscription à une activité (43) | ADF | quotas, tarif | FO9 |

### 21.10 Paramétrage, conformité
| ID | Action | Qui | Règles | Lot |
|---|---|---|---|---|
| FRM-250 | Régler seuils (anonymat, validation), relances, âge de l'avis | DIR · `formulaires.parametrer` | bornes | FO1 |
| FRM-251 | Mentions d'information de chaque formulaire (finalité, base, durée, droits) | SYS | générées depuis la nature | FO2 |
| FRM-252 | Registre des traitements : fiche par objet et par formulaire | DPO | socle 27 | FO13 |
| FRM-253 | AIPD pré-remplie | DPO | — | FO13 |
| FRM-254 | Droit d'accès : extrait des décisions et réponses d'une personne | DPO, ADF | socle 27 | FO13 |
| FRM-255 | Droit d'effacement (consentement retiré) : traitement | DPO | preuve conservée | FO13 |
| FRM-256 | Durées de conservation par objet et par formulaire | DPO, DIR | §30 | FO13 |
| FRM-257 | Purge | SYS | §30 | FO13 |
| FRM-258 | Journal des lectures détaillées | DPO | — | FO1 |
| FRM-259 | Export de preuve pour un litige | DIR | dossier de preuve | FO3 |
| FRM-260 | Revue annuelle du catalogue | DIR | rappel en juin | FO8 |
| FRM-261 | Désactiver la santé (D-034) | SYS | verrou tant que HDS absent | FO0 |
| FRM-262 | Choisir les langues des textes | DIR | — | FO8 |
| FRM-263 | Trames d'impression (identité seulement) | DIR | D-023 | FO2 |
| FRM-264 | Délégation de la gestion d'un objet (ex. CPE pour `sortie_seul`) | DIR | datée | FO1 |

### 21.11 Système
| ID | Action | Règles | Événement | Lot |
|---|---|---|---|---|
| FRM-265 | Recalculer les décideurs sur un fait du 4 ou du 2 | consomme `lien.*`, `eleve.majeur`, `capacite.*`, `desaccord.*`, `interdiction.*` | `autorisations.etat_modifie` | FO1 |
| FRM-266 | Expirer les accords | quotidien | `autorisations.expire` | FO1 |
| FRM-267 | Cesser les consentements parentaux à la majorité | `eleve.majeur` | idem | FO0–FO1 |
| FRM-268 | Ouvrir la campagne de renouvellement de rentrée | août | — | FO8 |
| FRM-269 | Relances dans la plage calme | socle 8 | — | FO2 |
| FRM-270 | Neutraliser titres et objets hors de l'ENT | corrige FD-08 | — | FO0 |
| FRM-271 | Refuser une version de numéro inférieur | corrige FD-14 | — | FO0 |
| FRM-272 | Verrouiller la nature d'un objet | corrige FD-15 | — | FO0 |
| FRM-273 | Rendre immuables versions, décisions, soumissions | déclencheurs | — | FO0 |
| FRM-274 | Sérialiser par décideur et sujet (verrou) | conservé | — | FO1 |
| FRM-275 | Exclure des relances les interdits, « ne pas solliciter », décédés | — | — | FO0 |
| FRM-276 | Projection de l'état reconstruite à la demande | R-FC4 | — | FO1 |
| FRM-277 | Purger la file de sondage anonyme à la clôture | — | — | FO4 |
| FRM-278 | Archiver à la bascule d'année | — | — | FO13 |
| FRM-279 | Retirer les anciennes tables (193, colonnes de 062 / 269) | grep à zéro | — | FO13 |

---

## 22. Règles transverses (R-FC1 à R-FC56) [R]

| # | Règle | Lot |
|---|---|---|
| R-FC1 | Nature juridique choisie à la création ; « consentement » refusé pour la mission, « collecte » refusée pour le facultatif | FO1 |
| R-FC2 | Décideurs calculés par les socles 2 et 4, jamais choisis par l'auteur | FO0 |
| R-FC3 | Aucun verdict `autorise` par défaut ; erreur ⇒ `inconnu` ⇒ l'appelant refuse | FO1 |
| R-FC4 | L'état est calculé ; la projection se reconstruit | FO1 |
| R-FC5 | Lien **communicant** pour lire et décider ; interdiction de communication = pas décideur | FO0 |
| R-FC6 | Règle d'accord : monter, jamais baisser sous le catalogue | FO1 |
| R-FC7 | Règle `un` : un refus connu = `desaccord` ; le second titulaire est informé et peut s'opposer | FO3 |
| R-FC8 | Droit à l'image : `tous` par défaut hors `fil_classe` ; avis de l'élève dès 11 ans, refus prioritaire | FO0 |
| R-FC9 | L'art. 45 (15 ans) n'efface pas la règle propre au droit à l'image | FO1 |
| R-FC10 | Désaccord déclaré ⇒ `desaccord` ; levée par pièce | FO3 |
| R-FC11 | Élève protégé ⇒ `protege` pour toute image hors personnels habilités | FO1 |
| R-FC12 | À la majorité, les consentements parentaux cessent ; l'élève est sollicité | FO0 |
| R-FC13 | Majorité par la fonction canonique et la capacité (tutelle, curatelle, émancipation) ; date contestée ⇒ non présumée | FO0 |
| R-FC14 | Retrait d'une autorisation jusqu'à l'événement ; après le départ, demande à l'établissement | FO3 |
| R-FC15 | Retrait d'un consentement à tout moment, sans passkey, effet immédiat, propagé | FO1 |
| R-FC16 | Le décideur ne voit jamais la décision d'un autre décideur | FO0 |
| R-FC17 | Autorisation : S1 (session < 15 min ou code au canal vérifié) ; acte non usuel : S2 | FO3 |
| R-FC18 | Sans réponse : non autorisé (autorisation), refusé (consentement) | FO0 |
| R-FC19 | Accord lié à une version et aux usages qu'elle couvre ; usage ajouté ⇒ version de fond | FO0 |
| R-FC20 | Prise de connaissance de la charte ou du règlement : n'empêche jamais l'accès d'un parent à l'ENT | FO2 |
| R-FC21 | Un objet a une seule nature, jamais changée ; numéros de version croissants | FO0 |
| R-FC22 | Questions sensibles interdites par défaut ; C3 seulement par le socle 34 ; C2 réservé à la direction | FO2 |
| R-FC23 | Santé désactivée tant que le module santé l'est (D-034) | FO0 |
| R-FC24 | Préremplissage limité aux données que le répondant a le droit de voir ; différence ⇒ proposition au socle 4 | FO7 |
| R-FC25 | Fratrie : une décision par enfant, explicite | FO2 |
| R-FC26 | Saisie au guichet : décideur choisi parmi les décideurs calculés ; mode G ou P ; « saisie par » | FO3 |
| R-FC27 | Jamais d'effacement d'une décision ; annulation par l'établissement motivée et visible | FO3 |
| R-FC28 | Anonyme : participation et réponse sans lien, date sans heure | FO4 |
| R-FC29 | Anonyme : écriture par lots mélangés ; aucun contenu ni identifiant de soumission dans les journaux | FO4 |
| R-FC30 | Anonyme : seuil de 5 par case ; texte libre désactivé par défaut | FO4 |
| R-FC31 | « Consultation, sans valeur de scrutin » ; jamais de vote | FO4 |
| R-FC32 | Un sondage anonyme ne devient jamais nominatif, et inversement | FO4 |
| R-FC33 | L'IA ne lit ni C3 ni C2 nominatif ni anonyme sous le seuil ; ne décide ni ne répond jamais | FO11 |
| R-FC34 | Questions figées à la publication ; réponses liées à leur version | FO2 |
| R-FC35 | Audience par critères sous la portée de l'auteur (D-022) | FO2 |
| R-FC36 | Relances seulement aux non-répondants, jamais aux interdits ni « ne pas solliciter » | FO0 |
| R-FC37 | Titre et objet jamais en clair hors de l'ENT | FO0 |
| R-FC38 | Lecture détaillée journalisée ; verdicts de routine en journal agrégé | FO1 |
| R-FC39 | Personnel de portée : verdicts seulement, sans motif ni décideur | FO1 |
| R-FC40 | Exports journalisés, formules neutralisées, C2 exclues sans droit | FO6 |
| R-FC41 | Usages déclarés par chaque consommateur ; re-vérification sur fait en < 1 minute ; contrôle nocturne | FO5 |
| R-FC42 | Choix tarifé : demande au socle 24 au payeur, tarif du 24 ; mode suivi pendant le doublon | FO9 |
| R-FC43 | Reprise : jamais « accordé » sans preuve ; Next.js ⇒ `inconnu` ; papier ⇒ attesté | FO3 |
| R-FC44 | Versions, décisions et soumissions en ajout seul (déclencheurs) | FO0 |
| R-FC45 | Empreinte encodée avec longueurs (sans séparateur ambigu) ; preuve de décision au socle 7 | FO1 |
| R-FC46 | Tout accès par `TenantTx`, RLS, FK composites | FO1 |
| R-FC47 | Aucun JSON-mémoire ; valeurs typées à colonne unique renseignée | FO2 |
| R-FC48 | Lien nominatif sans compte : à usage limité, haché, code pour signer | FO3 |
| R-FC49 | Départ de l'élève : plus de demande ; usages retirés à départ + 30 jours | FO5 |
| R-FC50 | Changement de responsable légal : décideurs recalculés à la date | FO1 |
| R-FC51 | Avis de l'élève recueilli par son propre compte, jamais par celui d'un parent | FO1 |
| R-FC52 | Le texte que voit le décideur est rendu par le serveur et figé avec son empreinte | FO1 |
| R-FC53 | Tirage au sort côté serveur, graine journalisée | FO2 |
| R-FC54 | Aucun traceur ; aucun bandeau de cookies nécessaire | — |
| R-FC55 | Toute nouvelle demande de consentement est proposée **décochée** | FO1 |
| R-FC56 | Le lot FO0 passe avant tout autre lot du socle | FO0 |

---

## 23. Droits et minimisation

### 23.1 Catalogue d'actions (D-017) [R]
| Action | Portées | Défaut | Remplace |
|---|---|---|---|
| `formulaires.creer` | établissement, niveau, classe (PP), cours (PRF) | DIR, ADF, CPE, PP, PRF | `formulaires.gerer` (établissement seul) |
| `formulaires.valider` | établissement | DIR | — |
| `formulaires.gerer` | établissement | DIR, ADF | (conservé, sens réduit à la gestion de tous les formulaires) |
| `formulaires.guichet` | établissement | ADF, accueil | — |
| `formulaires.reponses.lire` | auteur implicite ; confidentiel : destinataire désigné | — | — |
| `formulaires.parametrer` | établissement | DIR | — |
| `autorisations.lire` (verdicts) | classe, cours, sortie organisée | PRF, PP, ORG, CPE, VS | `consentements.lire` (établissement seul) |
| `autorisations.gerer` (décisions, preuves, désaccords) | établissement | DIR, ADF | — |
| `autorisations.parametrer` (catalogue, versions) | établissement | DIR | — |
| `sante.formulaires` | établissement | INF | désactivé (D-034) |

Familles, élèves, tiers légaux : hors rôles ; décisions par les droits du lien (`signer_quotidien`, `signer_documents`, `repondre_formulaires`, `autoriser_sorties`).

### 23.2 Pièges de minimisation à tester [R]
1. Un parent ne voit jamais la décision de l'autre.
2. Un professeur ne voit que des verdicts de **sa** classe, sans motif.
3. Un sondage anonyme sous le seuil ne montre rien, pas même un total.
4. Les journaux ne contiennent aucune réponse anonyme.
5. Un parent interdit ne voit ni le texte ni le prénom.
6. Le titre d'un formulaire n'apparaît dans aucun push.
7. Un formulaire ne demande jamais une donnée de santé.
8. Le port `verifier` ne dit pas pourquoi à un appelant non habilité.
9. Un export ne contient aucune colonne C2 sans droit.
10. La fiche « accords » hors ligne de l'encadrant ne contient que des verdicts.

---

## 24. Événements [R]

| Événement | Charge (identifiants) | Consommateurs |
|---|---|---|
| `autorisations.objet_active` / `.objet_desactive` | objet | 17, 42 |
| `autorisations.version_publiee` | objet, version, type (forme / fond / restriction) | 8 (nouvelle demande), 17 |
| `autorisations.decision` | objet, sujet, décideur, version | 42, 17, 4 |
| `autorisations.retire` | objet, sujet, usages | 17, 42, 4, 2, 33 |
| `autorisations.etat_modifie` | objet, sujet, ancien / nouveau verdict (codes) | 17, 42, 4, 2, 33, 14 |
| `autorisations.desaccord` | objet, sujet | 42, direction |
| `autorisations.expire` | objet, sujets | 8, 17 |
| `autorisations.incoherence` | usage déclaré | auteur de l'usage, secrétariat |
| `formulaires.publie` / `.clos` | formulaire, version | 8, 17 |
| `formulaires.soumis` | formulaire, cible (pas de contenu ; **aucun** événement par réponse anonyme) | 17 (attentes), 24 (choix tarifé) |

Consommés : `eleve.majeur`, `eleve.capacite_modifiee`, `eleve.protection_*`, `eleve.sorti`, `eleve.decede` (2) ; `lien.*`, `eleve.desaccord_declare` / `_leve`, `eleve.interdiction_*`, `personne.ne_pas_solliciter_*` (4) ; `annee.bascule` (5) ; `publications.retiree` / `.expiree` (17).

---

## 25. Natures d'avis déclarées au socle 8 [R]

| Nature | Urgence | Sensibilité | Sollicitation |
|---|---|---|---|
| `formulaires.a_remplir` | U1 | C1 (enveloppe neutre, corrige FD-08) | oui |
| `formulaires.relance` | U1 | C1 | oui |
| `autorisations.a_donner` | U1 | C1 | oui |
| `autorisations.autre_titulaire_a_autorise` (droit d'opposition) | U1 | C1 | **non** (information due au titulaire) |
| `autorisations.renouvellement` | U1 | C1 | oui |
| `autorisations.desaccord` (à la direction) | U2 | C2 | non |
| `autorisations.incoherence` (à l'auteur de l'usage) | U2 | C1 | non |
| `autorisations.majorite` (à l'élève devenu majeur) | U1 | C1 | non |

---

## 26. Écrans clés [R]

| Écran | Route | Pour | Contenu |
|---|---|---|---|
| Ce qu'on me demande | `/web/famille/demandes` | familles, élèves | par enfant : formulaires, accords à donner, échéances |
| Mes accords | `/web/famille/accords` | décideurs, élèves | par enfant et par objet : usages, état de **ma** décision, version, preuve, retrait en un geste, « où c'est utilisé » |
| Remplir | `/web/formulaires/{id}` | répondants | sections, conditions, brouillon, S1 / S2 au moment de l'accord |
| Concevoir | `/web/formulaires/nouveau` | auteurs | nature, unité, questions, sensibilité, audience, échéance, aperçu par profil |
| Suivi | `/web/formulaires/{id}/suivi` | auteurs | par cible, relance, résultats, exports |
| Verdicts | `/web/classes/{id}/autorisations` | PRF, PP | grille élève × objet / usage : peut / ne peut pas / en attente |
| Guichet | `/web/guichet/formulaires` | secrétariat, accueil | recherche d'élève, décideurs calculés, saisie P / G, lecture de code |
| Catalogue | `/web/direction/accords` | direction | objets, versions, usages, règles |
| Cohérence | `/web/direction/accords/coherence` | secrétariat, direction, DPO | usages en cours, incohérences |
| Désaccords | `/web/direction/accords/desaccords` | direction | à traiter |
| Mobile | applis uniques | familles, élèves, personnels | demandes, accords, retrait, verdicts de la sortie (encadrant), hors ligne chiffré |

---

## 27. Modèle de données cible (brique `formulaires`, migrations 2360–2379)

Mêmes règles de revue que le socle 17 §27 (E1–E14, D-036, D-037).

```
-- Accords --------------------------------------------------------------
ent_acc_objet
  id, etablissement_id, code TEXT CHECK (code ~ '^[a-z][a-z0-9_]{1,39}$')
  nature TEXT NOT NULL CHECK (nature IN ('information','collecte','autorisation','consentement'))   -- immuable
  regle  TEXT NOT NULL CHECK (regle IN ('un','tous','eleve_seul','eleve_et_titulaires','soi'))
  avis_eleve_des_ans SMALLINT NULL                       -- 11 par défaut pour l'image
  droit_lien_code TEXT NULL → ent_lien_droit_catalogue (socle 4)
  preuve_min TEXT NOT NULL CHECK (preuve_min IN ('S0','S1','S2'))
  validite TEXT NOT NULL CHECK (validite IN ('annee_scolaire','jusqu_a','evenement','scolarite','sans_terme'))
  valide_jusqu_au DATE NULL, evenement_fin TIMESTAMPTZ NULL
  sujet_type TEXT NOT NULL CHECK (sujet_type IN ('eleve','personne'))
  catalogue_code TEXT NULL → ent_acc_catalogue (référence globale)
  desaccord_objet TEXT NULL CHECK (IN ('image','sortie','general'))   -- correspondance avec ent_eleve_desaccord
  actif BOOLEAN, cree_par_personne_id, cree_le, modifie_le
  UNIQUE (etablissement_id, code)

ent_acc_catalogue                                  -- globale de référence fournie par ScolIA (registre : « référence »)
  code PK, nature, regle_min, preuve_min, validite, libelle, version_catalogue

ent_acc_usage
  id, etablissement_id, objet_id, code TEXT, libelle, mission BOOLEAN NOT NULL DEFAULT false   -- mission ⇒ non_requis
  UNIQUE (etablissement_id, objet_id, code)

ent_acc_version                                    -- ajout seul ; immuable après publication
  id, etablissement_id, objet_id, numero INTEGER CHECK (numero >= 1)
  texte TEXT CHECK (char_length BETWEEN 1 AND 10000), empreinte TEXT NOT NULL
  type_changement TEXT NULL CHECK (IN ('forme','fond','restriction'))     -- NULL pour la v1
  publiee_le, publiee_par_personne_id
  UNIQUE (etablissement_id, objet_id, numero) ; déclencheur : numero > max publié

ent_acc_version_usage (etablissement_id, version_id, usage_id)             -- usages couverts

ent_acc_traduction (etablissement_id, version_id, langue, texte, falc BOOLEAN, relue_par, relue_le)

ent_acc_decision                                   -- ajout seul
  id, etablissement_id, objet_id, version_id
  sujet_eleve_id UUID NULL, sujet_personne_id UUID NULL      -- CHECK un des deux selon sujet_type
  decideur_personne_id UUID NOT NULL
  qualite TEXT NOT NULL CHECK (qualite IN ('titulaire','eleve_avis','eleve_cosignataire','eleve_majeur','tuteur',
                                           'curateur_assiste','service_gardien','personne_soi'))
  decision TEXT NOT NULL CHECK (decision IN ('accorde','refuse','retire','annulee_par_etablissement'))
  canal TEXT NOT NULL CHECK (canal IN ('en_ligne','code','passkey','guichet','papier','attestation_reprise','reprise_sans_preuve'))
  preuve_signature_id UUID NULL → socle 7
  date_signature_papier DATE NULL
  saisie_par_personne_id UUID NULL
  formulaire_soumission_id UUID NULL
  motif_annulation TEXT NULL CHECK (char_length <= 300)
  decide_le TIMESTAMPTZ NOT NULL
  empreinte TEXT NOT NULL                            -- (version, sujet, décideur, décision, usages, date) encodés avec longueurs

ent_acc_decision_usage (etablissement_id, decision_id, usage_id, accorde BOOLEAN NOT NULL)

ent_acc_etat                                       -- projection reconstruisible (pas une source)
  etablissement_id, objet_id, usage_id, sujet_eleve_id NULL, sujet_personne_id NULL,
  verdict TEXT CHECK (IN ('autorise','refuse','en_attente','desaccord','expire','a_renouveler','protege','inconnu','non_requis')),
  calcule_le
  UNIQUE (etablissement_id, objet_id, usage_id, sujet_eleve_id, sujet_personne_id)

ent_acc_usage_declare
  id, etablissement_id, objet_id, usage_id, sujet_eleve_id NULL, sujet_personne_id NULL
  socle SMALLINT NOT NULL, contexte_type TEXT NOT NULL CHECK (liste fermée : 'publication_image','sortie_participant',
       'transmission_association','trombinoscope_famille','enregistrement_visio','production_publiee')
  contexte_id UUID NOT NULL                        -- clé du consommateur (pas de FK inter-briques ; vérifiée par le consommateur)
  du TIMESTAMPTZ, au TIMESTAMPTZ NULL, retire_le NULL

ent_acc_incoherence
  id, etablissement_id, usage_declare_id, verdict, detecte_le, traite_le NULL, traite_par NULL, issue TEXT NULL

-- Formulaires ---------------------------------------------------------
ent_frm_formulaire
  id, etablissement_id, nature TEXT CHECK (IN ('information','collecte','mixte')), titre, statut
  CHECK (statut IN ('brouillon','soumis','publie','clos','archive'))
  unite TEXT CHECK (IN ('eleve_un_titulaire','eleve_chaque_titulaire','foyer','personne','eleve_lui_meme'))
  confidentialite TEXT CHECK (IN ('nominatif','confidentiel','anonyme'))
  destinataire_confidentiel_personne_id NULL
  ouverture, echeance, relances_auto BOOLEAN, version_courante
  sante BOOLEAN NOT NULL DEFAULT false             -- vrai : stockage du socle 34, bloqué sans HDS
  auteur_personne_id, cree_le, modifie_le

ent_frm_version (id, etablissement_id, formulaire_id, numero, empreinte, publiee_le, publiee_par)   -- ajout seul
ent_frm_section (id, etablissement_id, version_id, ordre, titre, aide)
ent_frm_question
  id, etablissement_id, version_id, section_id, ordre, type CHECK (liste du §11.2), libelle, aide,
  obligatoire BOOLEAN, sensibilite CHECK (IN ('C0','C1','C2')), min_nombre NUMERIC NULL, max_nombre NUMERIC NULL,
  longueur_max INTEGER NULL, objet_accord_id NULL → ent_acc_objet (type 'accord'), tarif_id NULL (socle 24)
ent_frm_option (id, etablissement_id, question_id, ordre, libelle, quota INTEGER NULL, tarif_id NULL)
ent_frm_ligne (id, etablissement_id, question_id, ordre, libelle)                     -- grilles
ent_frm_condition (id, etablissement_id, question_id, depend_de_question_id, option_id, operateur CHECK (IN ('egal','different')))
ent_frm_traduction (etablissement_id, version_id, langue, element_type CHECK, element_id, texte, relue_par)

ent_frm_audience_critere                            -- même forme que ent_pub_audience_critere (17 §27)

ent_frm_cible                                      -- audience résolue
  id, etablissement_id, formulaire_id, eleve_id NULL, personne_id NULL, foyer_id NULL, ajoute_le, retire_le NULL

ent_frm_soumission                                 -- nominatif / confidentiel ; ajout seul
  id, etablissement_id, version_id, cible_id, repondant_personne_id, qualite, canal, saisie_par NULL,
  statut CHECK (IN ('brouillon','envoyee','remplacee','annulee_par_etablissement')), envoyee_le, remplace_id NULL
ent_frm_valeur
  id, etablissement_id, soumission_id, question_id, option_id NULL, ligne_id NULL, rang SMALLINT NULL,
  valeur_texte TEXT NULL, valeur_nombre NUMERIC NULL, valeur_date DATE NULL, valeur_heure TIME NULL, objet_id NULL (500)
  CHECK (au plus une valeur scalaire renseignée ; cohérente avec le type de la question — déclencheur)

ent_frm_participation                              -- anonyme : qui a participé, sans lien
  etablissement_id, formulaire_id, personne_id, participe_le DATE
  PRIMARY KEY (etablissement_id, formulaire_id, personne_id)
ent_frm_soumission_anonyme (id aléatoire, etablissement_id, version_id, lot_numero INTEGER, ecrit_le DATE)
ent_frm_valeur_anonyme (comme ent_frm_valeur, rattachée à ent_frm_soumission_anonyme)

ent_frm_tirage (id, etablissement_id, formulaire_id, question_id, graine TEXT, effectue_le, effectue_par)
ent_frm_export (id, etablissement_id, formulaire_id, par_personne_id, le, colonnes TEXT CHECK (<= 2000), lignes INTEGER)
ent_frm_reglage (une ligne par version : seuil_anonymat SMALLINT (5), seuil_validation INTEGER (200),
                 age_avis SMALLINT (11), relances TEXT CHECK (IN ('j7_j3_j1','j3_j1','aucune')), du, par, motif)
ent_frm_journal (ajout seul : acte, objet, formulaire, acteur, motif, le)
```

**Justification D-036 de `ent_frm_valeur`** : ce n'est **pas** une table clé-valeur libre : chaque ligne référence une question **et** (le cas échéant) une option par clé étrangère, ses colonnes sont **typées**, une seule est renseignée selon le type déclaré de la question (contrôle en base), et toutes les requêtes (« combien ont choisi l'option 3 », « moyenne de l'échelle ») sont des requêtes SQL ordinaires. Aucune exception D-036 n'est demandée.

**Tables reprises puis retirées** : `ent_formulaire`, `ent_formulaire_reponse` (193), `ent_sortie_autorisation` (062, avec le 14), colonnes de consentement de `ent_personne_consentement` (269/275, avec le 4).

---

## 28. Plan de migration depuis l'existant

1. **FO0 (2360)** : sur les tables existantes : déclencheurs d'ajout seul sur `ent_formulaire_reponse` et le texte publié ; refus d'une version inférieure ; verrou de nature par objet ; colonne `version_couverte` lue dans le calcul (corrige FD-01 : les décisions d'une version antérieure **à usages différents** passent `a_renouveler` ; faute de déclaration d'usages dans 193, **toute** version postérieure à la décision est traitée comme un changement de fond pour les accords, les refus restant protecteurs) ; lecture par lien communicant ; filtre `autoriser_sorties` pour les autorisations ; majorité par `ent_eleve_majorite_le` et la capacité ; notification de l'élève majeur ; titre neutre ; règle `tous` pour `droit_image` ; retrait de l'étiquette « sondage » des formulaires 193 existants en « consultation nominative » (aucune promesse d'anonymat).
2. **FO1 (2361–2363)** : tables `ent_acc_*` ; **reprise additive** (2369) : chaque `ent_formulaire` de type consentement / autorisation devient un objet + version(s) ; chaque réponse une décision (`canal` = `en_ligne`, empreinte d'origine conservée, preuve « reprise ») ; `ent_personne_consentement` → décisions `soi` ; double écriture une version ; le trombinoscope lit le port.
3. **FO2 (2364–2365)** : tables `ent_frm_*` ; les formulaires « sondage » de 193 deviennent des formulaires à une question `oui_non` nominatifs.
4. **FO4 (2366)** : tables anonymes.
5. **FO5 (2367)** : usages déclarés, incohérences.
6. **FO6 (2368)** : exports, journal, réglages.
7. **FO7** : reprise de `062` avec le socle 14 (dans la plage du 18 : 2370).
8. **FO13 (2378–2379)** : RLS stricte ; retrait de 193 et des colonnes reprises après grep à zéro.

---

## 29. Lots de code (équipe Sonnet)

| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| **FO0** correctifs urgents | FD-01 à FD-06, FD-08, FD-10 à FD-12, FD-14, FD-15, FD-17, FD-19 ; FRM-174, FRM-261, FRM-267, FRM-270 à FRM-273, FRM-275 | — | v2 « fond » ⇒ accords v1 `a_renouveler` ; parent interdit ni lecture ni signature ; décision de l'autre invisible ; majeur prévenu ; tutelle et émancipation ; titre neutre ; règle `tous` pour l'image ; `autoriser_sorties` exigé ; immuabilité en base |
| **FO1** accords et port | `ent_acc_*`, catalogue, versions à usages, décisions, avis de l'élève, retrait, « mes accords », port `verifier`, reprise 193 et 269/275, `TenantTx`, RLS | FO0 ; 4 (`titulaires_autorite`, `droits_lien`) ; 2 (capacité) | `verifier` jamais vrai par défaut ; retrait < 1 minute ; isolation |
| **FO2** formulaires | moteur (types, conditions, unités, versions), audience, réponse famille / élève, brouillons, relances, accessibilité | FO1 ; 8 | D-036 (`grep -i jsonb` vide) ; valeurs typées contrôlées ; D-022 |
| **FO3** autorisations de sortie et preuve | règles `un` / `tous`, information et opposition du second titulaire, désaccords, S1 / S2, guichet, papier, lien nominatif, reprise Next.js attestée, port pour le 42 | FO2 ; 7 (S1, S2, G, P) ; 42 | **niveau production pour la rentrée 2027** ; désaccord ⇒ non autorisé ; reprise ⇒ jamais « accordé » sans preuve |
| **FO4** sondages anonymes | tables séparées, lots mélangés, seuils, texte libre, notice | FO2 | aucun lien reconstructible (test de corrélation) ; rien sous le seuil |
| **FO5** cohérence | usages déclarés (17, 42, 4, 2, 33), re-vérification, contrôle nocturne, tableau | FO1 ; 17 AN5 | retrait ⇒ masquage < 1 minute ; incohérence ⇒ alerte |
| **FO6** résultats et exports | agrégats, ventilations, exports journalisés, publication des résultats | FO2 | formules neutralisées ; C2 exclues |
| **FO7** rebranchements | préremplissage et propositions (4), fiche de rentrée et préinscription sur le moteur, `062` avec le 14, prospects | FO2 ; 3, 4, 14 | aucune écriture directe dans le 4 |
| **FO8** renouvellement de rentrée | « comme l'an dernier », campagne d'août, traductions, revue du catalogue | FO1 | nouveaux usages décochés |
| **FO9** paiements et créneaux | choix tarifé (24, mode suivi), quotas, tirage, créneaux (19), activités (43) | FO2 ; 24, 19 | aucun encaissement par le 18 |
| **FO10** mobile | demandes, accords, retrait, verdicts de sortie hors ligne | FO3 ; 11 | effacement au retour |
| **FO11** IA | capacités §20.3 | FO2 ; cœur IA | aucune décision ; injections |
| **FO12** santé | moteur pour le socle 34 | 34 ; **HDS** | désactivé tant que D-034 l'exige |
| **FO13** conformité et retraits | registre, AIPD, droit d'accès, purge, RLS stricte, retrait de 193 | tous | grep à zéro |

**Numéros** : 2360 FO0 · 2361 objets, catalogue, usages · 2362 versions, décisions · 2363 projection, journal · 2364 formulaires, questions · 2365 cibles, soumissions, valeurs · 2366 anonymes · 2367 usages déclarés, incohérences · 2368 exports, réglages, tirages · 2369 reprise de 193 et 269/275 · 2370 reprise de 062 (avec le 14) · 2371–2377 réserve · 2378 RLS stricte · 2379 retraits.

---

## 30. Conservation [R, au DPO — Q-FC7]

| Donnée | Durée proposée | Appui |
|---|---|---|
| Consentements (décisions et preuves) | durée de l'usage + 1 an, et au moins jusqu'à la fin de l'année scolaire suivante ; tant qu'un contenu est diffusé, la preuve reste | RGPD art. 7.1 ; RF-7 |
| Autorisations d'un événement | événement + 2 ans (sauf contentieux) | RF-19 (dossiers de sorties) |
| Autorisations annuelles (régime de sortie, soins) | année + 1 | [H] |
| Prises de connaissance (règlement, charte) | scolarité + 1 an | [H] |
| Réponses nominatives d'une collecte | clôture + 1 an (sauf versement au dossier par le socle propriétaire) | [H] |
| Sondages anonymes | résultats : 5 ans ; participation : clôture + 3 mois | RF-19 (questionnaires d'enquêtes : 5 ans) |
| Exports, journaux | 12 mois | [H] |

---

## 31. Tests
- **Isolation** : matrice de séparation pour chaque route ; verdicts d'une autre classe ; décision d'un autre foyer.
- **Décideurs** : CK-01 à CK-29 (au moins 01, 02, 03, 04, 06, 08, 09, 10, 15, 17, 18, 19, 23).
- **Versions** : CK-30 à CK-32.
- **Anonymat** : test de corrélation (participation × écriture) ; seuils ; journaux sans contenu.
- **Cohérence** : retrait ⇒ masquage (17) < 1 minute ; contrôle nocturne.
- **Reprise** : 193 → accords ; Next.js → `inconnu`.
- **Contrat IA** : aucune capacité de décision ; injections dans les réponses libres.

---

## 32. Réglages signalés (D-023)

| # | Réglage | Défaut | Avis |
|---|---|---|---|
| S18-01 | Seuil de validation d'un formulaire | 200 personnes ou établissement | garder |
| S18-02 | Calendrier de relances | J-7, J-3, J-1 | garder |
| S18-03 | Règle de l'image pour `fil_classe` | `un` | garder (à revoir après Q-FC1) |
| S18-04 | Âge de l'avis de l'élève | 11 ans | garder |
| S18-05 | Preuve minimale par objet | catalogue | garder (monter seulement) |
| S18-06 | Délai de retrait des usages après départ | 30 jours | garder |
| S18-07 | Seuil d'anonymat | 5 | garder (ne jamais descendre sous 5) |
| S18-08 | Texte libre dans un sondage anonyme | désactivé | garder |
| S18-09 | Langues des textes | langues des familles | garder |
| S18-10 | « Comme l'an dernier » | actif | garder |
| S18-11 | Quotas et tirage | ordre d'arrivée | garder |
| S18-12 | Export des réponses par un professeur | ses formulaires, ses cours | garder (Q-FC9) |

---

## 33. Décisions

### 33.1 Appliquées par défaut (D-031)
1. Lot **FO0** avant tout autre lot (R-FC56).
2. Quatre natures juridiques ; « consentement » refusé pour la mission (R-FC1).
3. Trois objets : objet d'accord, formulaire, décision (§6.2).
4. Décideurs calculés par les socles 2 et 4 (R-FC2).
5. Droit à l'image : `tous` par défaut hors `fil_classe`, avis de l'élève dès 11 ans, refus prioritaire (R-FC8) — **en attendant le juriste** (Q-FC1).
6. Sortie facultative en France : `un`, refus connu = désaccord, information et opposition du second titulaire (R-FC7).
7. Voyage à l'étranger : `tous` par défaut.
8. Sans réponse : non autorisé / refusé (R-FC18).
9. Aucun `autorise` par défaut ; erreur ⇒ `inconnu` (R-FC3).
10. Versions à usages ; usage ajouté ⇒ fond ; accords antérieurs valables pour leurs usages seulement (R-FC19).
11. Consentements parentaux cessent à la majorité (R-FC12).
12. Interdiction de communication ⇒ pas décideur (R-FC5).
13. Le décideur ne voit pas la décision de l'autre (R-FC16).
14. Retrait sans passkey, immédiat, propagé (R-FC15).
15. S1 pour les autorisations courantes, S2 pour le non usuel (R-FC17).
16. Sondage anonyme à tables séparées, lots mélangés, seuil 5 (§12).
17. « Consultation, sans valeur de scrutin » (R-FC31).
18. Questions sensibles interdites ; santé au socle 34, désactivée (R-FC22, R-FC23).
19. Cohérence accord ↔ usage par usages déclarés (§14).
20. Reprise : jamais « accordé » sans preuve ; Next.js ⇒ `inconnu` (R-FC43).
21. Valeurs typées pour les réponses, sans exception D-036 (§27).
22. Choix tarifé : demande au 24, mode suivi pendant le doublon (R-FC42).
23. Charte : prise de connaissance élève + titulaire, sans bloquer l'accès des parents (R-FC20).
24. Plage 2360–2379.
25. `062` repris avec le socle 14 (FO7) ; `070` laissé au socle 7 (FD-21 signalé).
26. `SOCLES.md` §18 corrigé : l'existant n'est pas « A » mais « P » (contradiction 1).

### 33.2 Soumis à Florian (et au DPO, au juriste)
1. **Qualification du droit à l'image** (un ou deux accords, par usage) — Q-FC1, Q-FC2 : **juriste**.
2. **Valeur probante** de S1 pour une autorisation de sortie et besoin de S2 pour l'étranger — Q-FC3 : juriste.
3. **Privé sous contrat** : D111-8, circulaire de 2024, responsable de traitement (chef d'établissement ou OGEC) — Q-FC4 : juriste et DPO.
4. **Durées de conservation** (§30) et **AIPD** — Q-FC6, Q-FC7 : **DPO**.
5. **Textes des objets du catalogue** (image, soins, sortie, charte) : à faire relire avant tout usage réel (rentrée 2027).
6. **Données réelles** : aucune reprise de formulaires réels (Next.js, papier) avant l'accord écrit de l'établissement et l'avis du DPO (D-034, D-035).

### 33.3 Questions (fermées, avec recommandation)
| # | Question | Recommandation |
|---|---|---|
| Q-FC1 | Image : un ou deux accords ? | deux par défaut, sauf `fil_classe` (un) ; juriste |
| Q-FC2 | Notification + opposition à la place de la double signature ? | oui pour la règle `un` (RF-8) ; à confirmer pour l'image |
| Q-FC3 | S1 suffit-il pour une autorisation de sortie ? | oui en France ; S2 pour l'étranger |
| Q-FC4 | Privé : D111-8 et circulaire de 2024 ? | appliquées comme bonnes pratiques |
| Q-FC5 | Âge de l'avis de l'élève ? | 11 ans |
| Q-FC6 | Seuil d'anonymat ? | 5 ; AIPD |
| Q-FC7 | Durées ? | §30 |
| Q-FC8 | Statut des circulaires de 2004 et du guide de 2011 ? | à vérifier sur le BO quand il est accessible |
| Q-FC9 | Un professeur peut-il exporter les réponses de ses formulaires ? | oui, ses formulaires à ses cours, journalisé (ce n'est pas un export officiel de D-022) |

---

## 34. Faits, hypothèses, contradictions

**Faits [F]** : code Rust lu (chemins §2) ; Next.js lu (chemins §2.4). Rien compilé ni exécuté.

**Hypothèses [H]** : pratiques des établissements (coupons, classeurs, renouvellement annuel) ; durées (§30) ; marché (§2.5).

**Contradictions signalées**
1. `SOCLES.md` §18 « Existant : rien (A) » est faux : 193, `formulaires.rs`, écrans Leptos et le trombinoscope existent (corrigé dans la carte par cette conception).
2. `07_DOCUMENTS.md` §10.4 propose « un parent suffit, l'autre peut signer » comme règle générale ; pour le droit à l'image, ce socle retient `tous` par défaut (RF-1, prudence) — à arbitrer par le juriste (Q-FC1).
3. `42_SORTIES_VOYAGES.md` §11.2 : « un titulaire suffit, sauf désaccord déclaré ou règle "tous" pour l'étranger » : cohérent ; ce socle ajoute le **refus connu** (pas seulement le désaccord déclaré au socle 4) comme cause de désaccord (RF-14 : « sauf désaccord connu »).
4. `16_MESSAGERIE.md` §33.6 : le consentement des représentants passe ici (objet `liste_representants`).
5. La recherche du jour n'a pas retrouvé l'abrogation de la circulaire 2011-117 ; le socle 42 l'a établie (circulaire du 16-7-2024) : conclusion du 42 retenue.
6. `04_ADMINISTRATIF.md` R-A8 : « chaque formulaire du socle 18 déclare le droit qu'il exige » — repris : c'est l'**objet d'accord** qui le déclare (`droit_lien_code`).

---

## 35. Ajout personas (`PERSONAS_18.md`, 2026-11-11) — l'emporte sur ce qui précède en cas de contradiction

### 35.1 Règles ajoutées
| # | Règle | Lot |
|---|---|---|
| R-FC57 | **Route anonyme dédiée** : réponse anonyme sur une route distincte de la participation, **exclue des journaux d'accès** (ou sans IP ni identifiant, à la minute), délai aléatoire côté client (0–120 s), **sans cookie de session** (jeton de participation à usage unique et aveugle) ; test de non-corrélation base + journaux | FO4 |
| R-FC58 | **Aucun état global aux familles** : chaque décideur voit sa décision et sa preuve ; l'élève et ses titulaires voient seulement le **résultat d'un événement** (participe / ne participe pas) ; remplace §7.2 | FO0 |
| R-FC59 | **Attestation à deux** : en masse, un second personnel confirme ; scan d'un échantillon de 10 % au-delà de 20 élèves ; seulement pour un événement repris à la bascule | FO3 |
| R-FC60 | **Délégataire de l'image** (C. civ. 377, ou décision du juge) : décide seul sur pièce ; à défaut, accord du service gardien valable pour `fil_classe` seulement [NV, Q-FC10] | FO1 |
| R-FC61 | Sortie **obligatoire** : nature « information » imposée ; un formulaire « autorisation » est refusé | FO3 |
| R-FC62 | Formulaire combiné : la demande de paiement n'est créée qu'**après** l'autorisation de participation | FO9 |
| R-FC63 | Dossier de preuve d'un **accord** : lisible du décideur et de l'établissement seulement (contredit R-D31 du socle 7 pour ce cas) | FO3 |
| R-FC64 | Titulaire sous **interdiction de contact avec l'élève** : signe une version **épurée** (sans lieu ni horaire de départ et de retour) [NV, Q-FC12] | FO3 |
| R-FC65 | L'écran de l'organisateur indique la qualité requise (service gardien, juge, titulaires) selon la nature de l'acte pour un élève confié | FO3 |
| R-FC66 | Coupon papier sans nom lisible : attribué automatiquement si l'élève n'a qu'un décideur ; sinon choix par l'agent | FO3 |
| R-FC67 | Texte de `soins_urgence` : ne laisse jamais croire qu'un refus empêche les secours | FO1 |
| R-FC68 | Sondage anonyme avec texte libre : encadré préalable vers « signaler un harcèlement » (16 MSG-336), 3018, 119 | FO4 |
| R-FC69 | Sondage anonyme : résultats visibles **à la clôture** seulement | FO4 |
| R-FC70 | Synthèse IA de réponses nominatives : pseudonymisation avant envoi ; jamais pour un formulaire C2 | FO11 |
| R-FC71 | Autorisation donnée depuis le réseau de l'établissement pendant les heures de cours : signalée à la vie scolaire | FO3 |
| R-FC72 | Objet dont le texte n'a pas été **relu** (juriste ou direction selon la nature) : bandeau « texte provisoire » et interdiction d'usage réel | FO1 |

### 35.2 Actions ajoutées
| ID | Action | Qui | Lot |
|---|---|---|---|
| FRM-280 | Autorisation annuelle de sorties facultatives de proximité listées à l'avance | ORG, DIR | FO3 |
| FRM-281 | Basculer un décideur en papier, tracer un appel de relance | ORG, ADF | FO3 |
| FRM-282 | « Je veux qu'on respecte mon refus » : changement d'avis rapide signalé au CPE | ELV | FO1 |
| FRM-283 | Mode rafale de lecture de coupons (retour sonore, annuler le dernier) | ADF | FO3 |
| FRM-284 | Verdicts de sortie hors ligne à la porte (projection chiffrée, mise à jour 5 min) | SYS (14) | FO7 |
| FRM-285 | Collecte « pli de santé reçu » sans contenu | INF | FO2 |
| FRM-286 | Liste des retraits depuis la dernière transmission à l'association | ADF | FO5 |
| FRM-287 | Export vers le photographe prestataire (élèves autorisés seulement), effacement demandé | ADF | FO6 |
| FRM-288 | Production à plusieurs auteurs : mode de signature le plus restrictif | SYS | FO5 |
| FRM-289 | Délégation du droit à l'image enregistrée sur pièce (qualité `delegataire_image`) | ADF, DIR | FO1 |
| FRM-290 | Seconde confirmation d'une attestation en masse | DIR, ADF | FO3 |
| FRM-291 | Marquer un texte d'objet « relu » (par qui, quand) | DIR | FO1 |
| FRM-292 | Version épurée d'une autorisation pour un titulaire sous interdiction de contact | SYS | FO3 |
| FRM-293 | Extrait d'accès : participations aux sondages anonymes (sans réponses) | DPO | FO13 |
| FRM-294 | Jeton de participation aveugle pour la route anonyme | SYS | FO4 |

### 35.3 Cas limites ajoutés
| # | Situation | Ce qui doit se passer |
|---|---|---|
| CK-77 | Journaux du serveur lus par un administrateur | aucune corrélation possible (R-FC57) |
| CK-78 | Mère qui voit « non autorisé » et comprend que le père a refusé | n'arrive plus : pas d'état global (R-FC58) |
| CK-79 | Organisateur qui atteste 120 autorisations seul | refusé sans second (R-FC59) |
| CK-80 | Enfant confié, parents injoignables, photo de classe | service gardien pour `fil_classe` ; public non autorisé (R-FC60) |
| CK-81 | Série piscine en sortie obligatoire | information, pas d'autorisation (R-FC61) |
| CK-82 | Paiement demandé avant l'autorisation | impossible (R-FC62) |
| CK-83 | Père qui demande le dossier de preuve de la décision de la mère | refusé (R-FC63) |
| CK-84 | Père sous interdiction de contact avec l'élève | version épurée (R-FC64) |
| CK-85 | Famille d'accueil qui veut signer un voyage à l'étranger | « décision des titulaires ou du juge requise » (R-FC65) |
| CK-86 | 400 coupons, élèves à un seul titulaire | attribution automatique (R-FC66) |
| CK-87 | Texte libre anonyme révélant un harcèlement | action sur l'information ; encadré préalable (R-FC68) |
| CK-88 | Résultats consultés pendant la collecte | indisponibles (R-FC69) |
| CK-89 | Élève de 15 ans qui signe avec le téléphone de sa mère depuis le lycée | signalé à la vie scolaire ; avis au second titulaire (R-FC71) |
| CK-90 | Texte du droit à l'image non relu à la rentrée 2027 | usage réel interdit (R-FC72) |
| CK-91 | Photographe : élèves sans accord | non transmis (FRM-287) ; règle S18-13 |
| CK-92 | Article de trois élèves, l'un refuse son nom | signature « prénoms » (FRM-288) |

### 35.4 Réglages et questions ajoutés
- **S18-13** photo de classe sans accord : photographié seul pour sa famille / pas photographié — choix de l'établissement ; garder.
- **S18-14** signal d'une autorisation donnée depuis le réseau de l'établissement — garder.
- **Q-FC10** service gardien pour `fil_classe` ? → oui à défaut de délégataire ; juriste.
- **Q-FC11** formulaires de l'AS et de l'APEL : responsable de traitement ? → l'association, l'établissement étant sous-traitant ou simple relais ; DPO.
- **Q-FC12** version épurée sous interdiction de contact ? → oui ; juriste.
- **Q-FC13** résultats anonymes à la clôture seulement ? → oui.

### 35.5 Corrections du corps
- §7.2 : remplacé par R-FC58.
- §10 : « l'élève et ses titulaires voient le résultat d'un événement » (seule exception à R-FC58).
- §21.4 FRM-185 : soumis à R-FC59.
- §27 : ajouter la qualité `delegataire_image` à `ent_acc_decision.qualite` ; `ent_frm_jeton_participation` (jeton aveugle, haché, usage unique, sans lien avec la soumission) ; colonne `texte_relu_le`, `texte_relu_par` sur `ent_acc_version`.
- §28 : les photos du journal de voyage Next.js ne sont pas reprises (perdables, T0 M01) ; aucun champ « droit à l'image » de Charlemagne n'est importé comme accord.
