# Socle 6 — Les personnels, leurs services d'enseignement et les équipes

**Date :** 2026-11-06 — **Statut :** conception, **non décidée** (à valider par Florian). Aucun code écrit, rien compilé ni testé pour ce document. Aucun accès à la base ni aux secrets de production ; l'application Next.js a été lue en lecture seule ciblée (schéma Drizzle), sans copie de code.
**Branche :** `ent-socle-personnels-conception`.
**À lire avec :** `CLAUDE.md`, `docs/VISION.md`, `docs/PRODUCT_SPEC.md`, `docs/DECISIONS.md` (D-009, D-011, D-012, D-013, D-017, D-018, D-019, D-020, D-021, D-022), `docs/ROLES_ET_DROITS.md`, `docs/SOCLES.md` (§3, socles 5, 6, 15, 49), `docs/socles/02_ELEVE.md`, `03_INSCRIPTIONS.md`, `04_ADMINISTRATIF.md` (sur lesquels ce document s'appuie), `docs/SOCLE2_DOMAINES.md` (D15, D19, REF-12, REF-13), `docs/INTEROPERABILITE_NATIONALE.md` (§4, §6), `docs/socles/MIGRATIONS.md`.

**Conventions.** **[F]** fait vérifié dans le dépôt (chemin cité) ; **[F Next]** fait lu dans l'application Next.js (lecture seule) ; **[S]** fait sourcé repris d'un document du dépôt qui cite sa source ; **[H]** hypothèse (mémoire ou déduction) ; **[NV]** règle réglementaire citée de mémoire, **à vérifier sur un texte officiel avant de coder** ; **[R]** recommandation (non décidée). Les identifiants d'actions métier **`PRS-xx`** sont nouveaux et stables (le préfixe `PER-` est déjà pris par le périscolaire et `ADM-` par le socle 4, voir §20) ; la correspondance avec `RH-xx`, `ADM-xx` (D15) et `REF-12/13` de `SOCLE2_DOMAINES.md` est indiquée. **Plage de migrations : 350–369** (`docs/socles/MIGRATIONS.md`).

**[personas] Passe personas du 2026-11-07** (`docs/socles/PERSONAS_05-06.md`, branche `ent-personas-socles-5-6`) : ajouts marqués **[personas]** — cas limites P47 à P60 (§4.3), notes de correction (§7, §8.2), modèle §8.14, actions PRS-116 à PRS-147 (§10.11), règles R-P1 à R-P26 (§10.12), droits §14.3, événements §15.3, lots P15 et P16 (§18), questions Q-P28 à Q-P35. **À lire d'abord : `PERSONAS_05-06.md` §0** (contradictions avec le socle 5 — cours / service, fonction de périmètre — et engagements simultanés, à trancher avant les lots P0, P1, P3, P4). Rien n'a été supprimé.

**Méthode et limites.** Trois sous-agents Haiku ont relevé en lecture seule : le catalogue d'actions et le moteur de rôles (`ent-auth`, `droits_repo.rs`), le périmètre et les tables du référentiel (`perimetre.rs`, migrations 092, 098, 110, 080), le modèle des personnels de Next.js (`db/schema.ts`, `db/schema-ent-relational.ts`, `app/lib/intranet-roles.ts`, `app/lib/personnel-types.ts`). Les points structurants ont été revérifiés directement (migrations 005, 092, 098, 110, 120 ; `auth_repo.rs:820-833` ; `service.rs:3175-3290, 4394` ; `perimetre.rs:1-26` ; `routes.rs:196-201, 755-761, 1035-1101`). **La session du socle 5** (`ent-socle-structure-conception`) **n'était pas poussée** au moment de la rédaction (`git ls-remote` le 2026-11-06) : la frontière suit `SOCLES.md` §3 et les points à accorder sont listés au §5.4.

Sommaire : §1 Thèse · §2 Évaluation de l'existant · §3 Acteurs et ce qu'ils vivent · §4 Les situations réelles · §5 Frontières · §6 Concepts · §7 Le temps et l'historique · §8 Modèle de données cible · §9 Comment ce socle alimente les droits · §10 Actions métier · §11 Logiciel ou paramétrage · §12 Échanges avec l'extérieur · §13 Sécurité et données sensibles · §14 Droits (catalogue) · §15 Événements · §16 Écrans clés · §17 Plan de migration · §18 Lots de code · §19 Questions à trancher · §20 Récapitulatif.

---

## 1. Thèse : ce que doit savoir l'établissement sur ceux qui y travaillent

Les socles 2 à 4 disent qui est l'élève et qui en répond. Le socle 6 dit **qui travaille dans l'établissement, à quel titre, depuis quand et jusqu'à quand, qui enseigne quoi à quels élèves, qui est responsable de quoi, et comment ces personnes s'organisent**. C'est le socle dont dépendent **presque tous les droits d'accès** : ce qu'un professeur voit (D-022), ce qu'une CPE traite, ce qu'un remplaçant peut saisir, qui reçoit le message « équipe de la 4B », qui siège au conseil de classe, qui encadre une sortie, qui peut signer un certificat. Une erreur ici ne fait pas un écran faux : elle **ouvre ou ferme l'accès aux données de centaines d'enfants**. Le compte d'un contractuel parti en février encore actif en juin, un remplaçant qui voit toute la classe alors qu'il n'a qu'un groupe, un professeur principal qui n'est plus PP mais garde ses droits : ce sont les défauts classiques d'un ENT, et l'existant les a presque tous (§2.2).

Dix principes [R] :

1. **Quatre choses distinctes : la personne, l'engagement, l'accès, les droits.** La *personne* existe une fois dans l'établissement (le socle 4 la porte, D-020) ; l'*engagement* dit à quel titre et sur quelle période elle travaille ici (titulaire, contractuel, AED, salarié de l'organisme de gestion, agent de la collectivité, intervenant, bénévole) ; l'*accès* est son compte, ouvert ou fermé (socle 1) ; les *droits* viennent des rôles, des services et des missions. Aujourd'hui les quatre sont écrasés dans une seule ligne `ent_membership (user_id, etablissement_id, role)` [F `005_ent_auth.sql:14-20`].
2. **Le service d'enseignement est une unité stable ; les personnes qui l'assurent changent.** « Mathématiques en 4B, 3 h 30 » existe de septembre à juin. Le titulaire, la suppléante pendant son congé, la co-intervenante en enseignement scientifique **se succèdent ou se cumulent dessus**, chacun avec ses dates. Notes, cahier de textes, progression et appel s'accrochent au service : une suppléante qui arrive un lundi retrouve tout, le titulaire qui revient aussi. Aujourd'hui le service *est* le professeur (`prof_user_id` dans la clé) [F `092:87-109`].
3. **Tout est daté, y compris dans le futur.** Arrivée le 4 novembre, PP à partir du 1ᵉʳ septembre, suppléance du 12 janvier au 3 avril, fin de contrat le 7 juillet : on saisit à l'avance, le logiciel applique le jour venu, seul, en heure de Paris. Aujourd'hui une attribution de rôle commence obligatoirement le jour de la saisie [F `droits_repo.rs:476`] et un service n'a pas de dates [F `092`].
4. **Les droits découlent des faits, ils ne se saisissent pas deux fois.** On ne « donne pas l'accès à la 4B » à un professeur : on lui attribue le service « Allemand LV2, groupe ALL-4 » et le droit s'ensuit, **limité au groupe** (D-022), pendant la période du service. On ne « donne pas les droits de PP » : on le nomme PP de la classe et les droits du modèle « Professeur principal » s'appliquent sur cette classe, pour cette année. Quand le fait s'arrête, le droit s'arrête.
5. **Les équipes sont calculées quand elles peuvent l'être, constituées seulement quand il le faut.** L'équipe pédagogique de la 4B, c'est la liste des intervenants actifs sur les services de ses élèves, plus le PP, plus la CPE référente : personne ne la tient à jour. Le conseil pédagogique, la cellule de veille, l'équipe EPS, l'équipe d'un projet : ce sont des équipes constituées, avec des membres datés.
6. **Arriver et partir sont des parcours, pas des formulaires.** L'arrivée enchaîne fiche, engagement, services, missions, rôles proposés selon la fonction, invitation, passkey, charte, clés et badge ; le départ enchaîne reprise des services, transfert des missions, tâches en cours, fermeture de l'accès à la date, matériel à rendre. Chaque étape cochée seule quand le fait est constaté (passkey enrôlée, charte signée). **Aujourd'hui aucun départ n'est possible** : il n'existe ni désactivation de compte ni fin de rattachement à l'établissement [F, §2.2 n° 1].
7. **Une personne, plusieurs casquettes, aucune confusion.** Le professeur de mathématiques est aussi le père de Léa, en 6ᵉ dans le même collège ; l'AED est un ancien élève ; la mère de Jules anime l'atelier théâtre. Une seule personne, une seule passkey (D-013), mais ce qu'elle fait comme professeur et comme parent ne se mélange jamais, et les conflits d'intérêts (noter son propre enfant) sont visibles. Aujourd'hui c'est impossible : un compte n'a qu'un profil par établissement [F `005:19`, `service.rs:4394`].
8. **Plusieurs établissements, aucune visibilité croisée.** Un professeur en complément de service dans deux collèges ScolIA a une identité et deux engagements ; chaque établissement ne connaît que le sien (D-013). Seul l'intéressé peut, s'il le veut, voir ses deux emplois du temps ensemble ou transmettre ses indisponibilités.
9. **Public, privé sous contrat, hors contrat : un modèle, des statuts.** Le logiciel ne code ni la paie ni le droit du travail (socle 49) ; il enregistre la **nature de l'engagement** et l'**employeur** (État, collectivité, organisme de gestion, association, aucun), parce qu'ils changent ce qu'on peut demander, ce qu'on exporte (STS-web, AAF) et qui on prévient.
10. **Les personnels sont aussi des personnes protégées.** Données minimales (pas d'adresse personnelle, pas de date de naissance, pas de NUMEN sans nécessité démontrée), coordonnées personnelles invisibles des familles et des collègues sauf choix de l'intéressé, aucun classement nominatif, indicateurs d'activité réservés à la direction et annoncés, droit de l'agent à voir sa propre fiche et ses droits. Un ENT qui surveille ses professeurs sera combattu par eux.

Ce qui fera la différence au quotidien [H — comparaison de mémoire avec les logiciels du marché, non vérifiée] : la **rentrée des personnels en une matinée** (import ou saisie, invitations en lot, enrôlement par QR code en salle des professeurs) ; la **répartition des services** préparée en juin sur l'année suivante, avec contrôle des heures dues et des trous, puis ouverte d'un geste à la rentrée ; la **suppléance qui hérite de tout** (cahier, notes, progression) sans copier un fichier ; les **droits qui expirent seuls** ; le bouton **« pourquoi peut-il voir cet élève ? »** ; le **départ sans compte orphelin** ; la **revue des accès** trimestrielle en dix minutes ; l'écran famille **« l'équipe de mon enfant »** toujours juste.

---

## 2. Évaluation franche de l'existant

Sources : migrations `005_ent_auth.sql`, `080_absence_rh.sql`, `089_passkey.sql`, `092_referentiel_scolaire.sql`, `095_session_assurance.sql`, `098_roles_parametrables.sql`, `110_referentiel_structure.sql`, `120_personne.sql` ; `ent-auth/src/actions.rs`, `roles.rs` ; `ent-db/src/droits_repo.rs`, `perimetre.rs`, `referentiel_repo.rs:979-1010`, `referentiel_structure.rs:655-698`, `auth_repo.rs`, `service.rs` (extraits cités) ; `ent-api/src/routes.rs`, `referentiel.rs`, `roles.rs` ; écran Leptos `web/crates/ent-web-app/src/modules/roles` (existence vérifiée, non relu). Next.js [F Next] : `db/schema.ts`, `db/schema-ent-relational.ts`, `db/schema-charlemagne-p1.ts`, `app/lib/intranet-roles.ts`, `app/lib/personnel-types.ts`.

### 2.1 Ce qui tient (à garder)

| Élément | Pourquoi c'est bon | Preuve [F] |
|---|---|---|
| **Identité unique, rattachement par établissement** : `ent_user` (e-mail unique) + `ent_membership` (clé `user_id, etablissement_id`) | Exactement D-013 : une identité, plusieurs établissements | `005:4-20` |
| **Moteur de rôles paramétrables** : catalogue fixe de 47 actions, quatre portées, rôles libres de l'établissement, cumul par union, attribution avec fin facultative, lus à chaque requête | Le bon cadre pour tout ce socle : on n'invente pas un second moteur | `actions.rs:13-126`, `droits_repo.rs:57-102`, `098` |
| **Anti-escalade** (chaque droit accordé doit être couvert par l'acteur, cibles comprises) et journal des rôles | Garde-fou indispensable quand la direction délègue | `droits_repo.rs:633-659, 738` |
| **Modèles par défaut = comportement historique**, avec test oracle profil × action | Bascule sans régression ; méthode à réutiliser | `actions.rs:381-540` ; `ROLES_ET_DROITS.md` (test `r2_modele_identique_aux_capacites_historiques`) |
| **Service d'enseignement sur classe *ou* groupe** (`CHECK` d'exclusivité), relié aux créneaux et aux séances datées | Base juste de D-022 : la cible peut être un groupe | `092:87-101` ; `110:188-214` |
| **`enseigne_a(prof, élève, date)`** : service ∩ affectation datée de l'élève | La bonne requête de périmètre, déjà testée | `referentiel_repo.rs:979-1010` |
| **Responsables de groupes et d'ateliers** (`ent_groupe_responsable`) et portée `mes_groupes` | Le « professeur de théâtre » de D-017 fonctionne | `098:48-57` ; `droits_repo.rs:216-241` |
| **Sessions révocables par compte**, assurance (`mot_de_passe`, `passkey`, `sso`), inactivité contrôlée en base, politique « passkey obligatoire » qui ferme les sessions des comptes non enrôlés | Briques du départ et de l'enrôlement obligatoire (D-009, D-011) | `095` ; `auth_repo.rs:1139` ; `securite_repo.rs:86-134` |
| **Contrôle de cible des actions sur les comptes** (qui peut changer l'e-mail, réinitialiser les passkeys de qui, refus pour un compte rattaché à un autre établissement) | Bonne hiérarchie, à reprendre dans les actions du catalogue | `service.rs:2619-2680` |
| **Provisionnement silencieux si l'adresse existe ailleurs** (réponse identique à un succès) | Ne révèle pas qu'une personne est connue d'un autre établissement (D-013) | `service.rs:3207-3248` |
| **Écrans Leptos d'administration des rôles** (liste, création depuis un modèle, tableau action × portée, membres avec date de fin, responsables des groupes) | Socle de l'écran « accès » de la fiche agent | `PROJECT_STATUS.md` (socle 1) ; `modules/roles` |

### 2.2 Ce qui est à revoir

Classé par gravité. Les points 1 à 6 sont des **défauts de sécurité ou de comportement** (lot P0, sauf le point 5 : lot P4) ; les suivants des limites de modèle.

| # | Problème | Conséquence | Preuve [F] | Correction [R] |
|---|---|---|---|---|
| 1 | **Aucun départ possible.** `ent_membership` n'a ni état ni date de fin ; aucune fonction ne la supprime ; `ent_user` n'a pas de colonne d'activité ; la validité d'une session est une simple jointure sur `ent_membership` | Un contractuel parti, un AED en fin de contrat, un professeur muté **gardent leur accès indéfiniment** : on peut révoquer leurs sessions, ils se reconnectent. C'est le premier constat de tout audit d'accès | `005:14-20` ; `auth_repo.rs:820-833` ; aucun `DELETE FROM ent_membership` (relevé) | État de l'accès par établissement, date de fin, contrôle à chaque session ; fermeture à la date de fin d'engagement (§8.4, PRS-30 à 34) |
| 2 | **Garde-fou « administrateur indélébile » non implémenté** (annoncé par `ROLES_ET_DROITS.md` garde-fou 3) | Un établissement peut se retrouver sans administrateur ; le futur départ (point 1) aggraverait le risque | recherche négative (relevé) | Contrôle dans toute commande qui ferme un accès, retire un profil ou un rôle (§13.1) |
| 3 | **Mot de passe temporaire renvoyé en clair** dans la réponse JSON du provisionnement du personnel | Le secret transite par l'écran et le presse-papiers du secrétariat ; il est souvent envoyé par messagerie personnelle ; contraire à D-009 (passkey) | `routes.rs:1073-1101` (`mot_de_passe_temporaire`) ; `service.rs:3194` | Invitation à usage unique par e-mail ou QR code, enrôlement de la passkey à la première connexion (§8.4, PRS-20) |
| 4 | **Un compte = un seul profil par établissement.** Les droits famille dépendent du profil de la session (`parsed == Role::Parent`, 76 occurrences dans `service.rs`) | Un **professeur parent d'élève** dans son établissement (cas fréquent, surtout dans le privé [H]) ne peut pas agir comme parent (justifier, signer) ; à l'inverse il voit son enfant **avec ses droits de professeur**. Une mère qui anime un atelier ne peut pas en être responsable sans perdre son espace parent | `005:19` ; `service.rs:4394, 4908, 4943, 4999` | Casquettes multiples, une casquette active par session, bascule en un geste (§8.4, Q-P2) |
| 5 | **Le repli « classe texte » du périmètre ouvre la classe entière** à tout professeur qui a un créneau sur la classe, et le remplaçant d'**une** séance voit toute la classe **14 jours avant et après** | Contredit D-022 (professeur de LV2 limité à son groupe) ; un remplacement d'une heure ouvre un mois de données | `perimetre.rs:1-26, 41-79, 128-139` (`FENETRE_REMPLACEMENT_JOURS = 14`) | Périmètre par intervenant de service et par période ; remplacement de séance limité à la séance (socle 15) ; repli texte coupé établissement par établissement quand le référentiel est complet (§9.4) |
| 6 | **Dates calculées en UTC** (`today_utc`) dans le périmètre | Un service ou une attribution qui finit le 31 août reste actif jusqu'à 2 h du matin heure de Paris | `perimetre.rs:15` | Dates métier en `Europe/Paris`, comme le socle 4 (`04_ADMINISTRATIF.md` §2.2 n° 4) |
| 7 | **Le service est le professeur.** Clé `prof_user_id` sur le **compte** ; pas de dates, pas d'heures, pas de modalité ; une suppléance = un autre service | Pas de suppléance qui hérite du cahier et des notes ; pas de co-enseignement propre ; un service ne peut exister avant que le professeur ait un compte (rentrée, poste vacant, import STS) ; changer de professeur en cours d'année casse l'historique | `092:87-109` | Service = unité ; intervenants datés (§8.5) |
| 8 | **Professeur principal = un booléen sur un service**, **un seul par classe** (index unique sans date) | Pas de co-PP (deux PP en terminale [NV]) ; un PP doit avoir un service *sur la classe* (pas sur un groupe) ; un changement de PP efface l'ancien ; aucun droit propre au PP | `092:97-113` ; `referentiel_structure.rs:681-691` | Mission datée, nombre paramétrable, droits par modèle de rôle (§8.6) |
| 9 | **Attribution de rôle sans historique ni début futur** : clé `(role_id, user_id)`, `ON CONFLICT … DO UPDATE` qui écrase, début forcé à aujourd'hui | « Qui avait ce rôle le 12 mars ? » sans réponse ; impossible de préparer la rentrée ; une réattribution efface la précédente | `098:32-42` ; `droits_repo.rs:474-478` | Attribution identifiée, datée, jamais écrasée, avec source (§8.9) |
| 10 | **Responsables de groupe sans dates**, remplacés en bloc (`DELETE` puis `INSERT`) | Un atelier qui change d'animateur en janvier perd l'historique ; aucun animateur futur | `098:48-57` ; `droits_repo.rs:580-589` | Responsabilité datée (§8.7) |
| 11 | **Aucune fiche personnel** : ni fonction, ni discipline, ni statut, ni quotité, ni dates, ni employeur ; le « personnel » est déduit du profil (`Role::is_staff`) | Annuaire réduit à e-mail, profil, nom, téléphone ; aucune répartition des services possible ; aucun import STS-web ; aucune vue « qui est là aujourd'hui » | `roles.rs:55-57` ; `routes.rs:755-761` | `ent_agent` et ses engagements (§8.1–8.3) |
| 12 | **L'annuaire du personnel expose e-mail et téléphone de tous à tout le personnel** | Le téléphone (souvent personnel) d'un professeur est lisible par tous les collègues, y compris un AED de 19 ans | `routes.rs:1035-1053` (`search_personnel`), `PersonnelDto` `routes.rs:755-761` | Coordonnées professionnelles seules dans l'annuaire ; personnelles selon le choix de l'agent (§13.3) |
| 13 | **Création des comptes du personnel réservée aux profils Admin et Direction codés en dur**, hors catalogue | Un établissement ne peut pas confier l'arrivée des AED à la CPE ou celle des agents au secrétariat par un rôle | `service.rs:3182-3185` | Actions `personnels.*` du catalogue (§14) |
| 14 | **Aucune action du catalogue** pour les personnels, comptes, services, missions, équipes, délégations ; services et PP passent par `referentiel.gerer` | Impossible de séparer « gérer les classes » (socle 5) et « répartir les services » (socle 6) | `actions.rs:183-230` | §14 |
| 15 | **Portée `moi` documentée mais absente** du code | Pas de base propre pour « mes informations », « mes services » | `ROLES_ET_DROITS.md` l.15 contre `actions.rs:117-126` | Implémenter (Q-P8) |
| 16 | **Le personnel n'a pas de personne** (`ent_personne` ne couvre que responsables, contacts et élèves majeurs) ; la reprise 120 ne crée des personnes que pour les comptes liés à un élève | Le professeur-parent a une personne (comme parent) mais pas de fiche de personnel ; impossible de rattacher un agent sans compte | `120:57-73` | Le personnel devient une personne du socle 4 (§6.2, Q-P1) |
| 17 | **Absences des personnels, créneaux, séances, remplaçants rattachés au compte** (`user_id`, `prof_user_id`, `remplacant_user_id`) | Même défaut que n° 7 pour les socles 10 et 15 : un remplaçant doit avoir un compte avant d'être planifié | `080:3-21` ; `021:9` ; `110:204, 213` | Colonnes `agent_id` en parallèle, projection (§17, migration 362) |
| 18 | **Clés étrangères simples, pas de RLS** sur services, rôles, attributions, responsables | Isolation par filtres applicatifs seulement (même constat que `02_ELEVE.md` §10.3) | `092`, `098` | Clés composées, RLS (§13.1) |

### 2.3 Ce qui manque

- **Fiche de l'agent** dans l'établissement : fonctions, disciplines de poste, qualifications utiles (secourisme, encadrement, langues), engagements successifs, quotité, employeur, site.
- **Arrivée et départ** : parcours, invitation, enrôlement, transfert des services et missions, fermeture de l'accès, revue des tâches en cours.
- **Suspension d'accès** sans fin d'engagement (mesure conservatoire, congé long à la demande).
- **Services datés à intervenants multiples** ; heures ; modalités ; services prévus pour l'année suivante.
- **Missions** : professeur principal (plusieurs), référents (décrochage, harcèlement, numérique, culture, laïcité…), coordinateurs (discipline, niveau, cycle), tuteurs, et missions créées par l'établissement.
- **Équipes** calculées et constituées.
- **Délégations** datées (intérim de direction, PP par intérim, délégation de signature).
- **Intervenants extérieurs et bénévoles** sans compte, ou avec un accès limité et daté.
- **Casquettes multiples** d'un même compte.
- **Explication des droits** (« pourquoi », « depuis quand », « jusqu'à quand »), simulation avant attribution.
- **Revue périodique des accès**.
- **Imports et exports** : STS-web, AAF, logiciels d'emploi du temps, CSV de reprise ; annuaire GAR.
- **Vue famille et élève** de l'équipe.

### 2.4 Ce que Next.js fait et qu'il ne faut pas perdre (lecture seule)

| Fonction [F Next] | Où | Reprise [R] |
|---|---|---|
| Table `personnel` distincte du compte : `category`, `jobTitle`, `hireDate`, `active`, `managerId`, e-mails professionnel et personnel | `db/schema.ts:840-872` | Bonne intuition (fiche ≠ compte) ; reprise en `ent_agent` + engagements ; `managerId` (supérieur hiérarchique) : **non repris** dans ce socle (organigramme hiérarchique utile au privé, relève du socle 49) ; deux e-mails distincts repris comme coordonnées à usages différents |
| Tables filles RH : congés, documents partagés, documents, formations, **habilitations**, entretiens, attributs | `db/schema-ent-relational.ts:596-720` | Congés → socle 15 ; documents, formations, entretiens → socle 49 ; **habilitations** : seules les qualifications utiles à l'organisation de la journée (secourisme, encadrement de sortie, permis) sont reprises ici (§8.3) |
| Liste des rôles d'intranet : admin, administratif, **accueil**, professeur, direction d'école / collège / lycée, comptabilité, **maintenance**, infirmerie, psychologue, **surveillant**, **internat**, CPE | `app/lib/intranet-roles.ts:4-22` | Confirme le besoin de fonctions au-delà des dix profils Rust : reprises comme **fonctions** (§8.3) et **modèles de rôles** (§14), pas comme profils figés |
| Catégories de personnel non enseignant (administratif, maintenance, surveillant, CPE, comptabilité) | `app/lib/personnel-types.ts:10-15` | Fonctions du catalogue |
| Affectation professeur ↔ classe par **clé texte** de classe, unique par classe ; pas de professeur principal | `db/schema.ts:707-727` | **Défaut à ne pas reproduire** (clé texte, un seul professeur par classe) |
| Lien matière ↔ classe avec `enseignantUserId` **et** `enseignantNom` (texte) | `db/schema-charlemagne-p1.ts:157-179` | Le nom texte est une copie qui diverge : **non repris** ; le service porte l'agent |
| Planning enseignant A/B et **remplacements** | `db/schema.ts:759-833` | Socles 10 et 15 |
| Équipe de portes ouvertes par créneau (ambassadeur, enseignant, personnel) | `db/schema-portes-ouvertes.ts:120-143` | Cas d'**équipe constituée ponctuelle** (§8.8), utile au socle 3 |
| Compte par établissement (`user` unique par e-mail **et** établissement) et `user_membership` à contextes (`staff`, `parent`, `eleve`) | `db/schema.ts:53-84, 245-266` | Le **multi-contexte** confirme le besoin de casquettes (§8.4) ; l'identité, elle, reste unique (D-013) |

---

## 3. Acteurs et ce qu'ils vivent

| Acteur | Ce qu'il vit aujourd'hui [H] | Ce qu'il redoute | Ce qui le ravirait | Fréquence |
|---|---|---|---|---|
| **Chef d'établissement, adjoint** | Construit la répartition des services dans un tableur en juin, la recopie dans le logiciel d'emploi du temps puis dans STS-web ; découvre en septembre les postes non pourvus ; nomme les PP par courriel ; gère les arrivées de contractuels au fil de l'eau | Une classe sans professeur à la rentrée ; un parti qui a encore accès ; ne pas savoir qui peut voir quoi en cas de contrôle ou d'incident ; une plainte d'un professeur pour surveillance | Une répartition préparée dans l'outil, avec heures dues, trous et doublons signalés ; la rentrée ouverte d'un clic ; un tableau des accès certifiable ; « pourquoi M. X voit-il cet élève ? » en un clic | quotidienne ; pics en juin et septembre |
| **Secrétariat de direction, secrétariat de scolarité** | Crée les comptes un par un, envoie des mots de passe, répond aux « je n'arrive pas à me connecter » ; tient la liste des personnels dans un fichier à part | Envoyer un identifiant à la mauvaise personne ; oublier quelqu'un ; ressaisir trois fois | Un import ou une saisie qui invite tout le monde ; un enrôlement autonome par QR code ; une liste de qui n'a pas encore activé son accès | hebdomadaire ; pic en septembre |
| **Nouveau professeur, contractuel, suppléant** | Arrive sans accès, sans liste d'élèves, sans cahier de textes ; on lui transmet des photocopies ; perd une semaine | Faire l'appel sur papier ; ne pas savoir où en était le titulaire ; ne pas connaître les élèves à besoins particuliers | En dix minutes : passkey sur son téléphone, emploi du temps, listes avec photos, progression et cahier du titulaire, aménagements des élèves (socle 35), qui est le PP, la CPE, le collègue de la même discipline | ponctuelle, mais décisive |
| **Professeur titulaire** | Voit des classes qu'il n'a pas ou n'en voit pas une qu'il a ; découvre un groupe supprimé ; part en congé sans savoir qui reprend | Que ses notes disparaissent ; qu'on le juge sur un tableau de bord ; recevoir des messages pendant son congé maladie | Voir exactement ses services, ses groupes, ses missions ; retrouver tout à son retour ; pouvoir mettre ses notifications en pause | quotidienne |
| **Professeur principal** | Doit réunir l'équipe, préparer le conseil, joindre les familles ; ne sait pas toujours qui enseigne dans sa classe | Oublier un collègue ; ne pas avoir accès à ce que le rôle exige | L'équipe de sa classe calculée, une liste de diffusion juste, les droits utiles à son rôle et rien de plus | hebdomadaire |
| **Professeur en complément de service** (deux établissements) | Deux comptes, deux mots de passe, deux emplois du temps qui se chevauchent | Être convoqué à deux conseils en même temps | Une identité, une passkey, ses deux emplois du temps visibles par lui seul, ses indisponibilités transmises d'un geste | quotidienne |
| **CPE, vie scolaire** | Gère les AED (planning, accès), cherche « où est M. X maintenant ? » pour un élève malade ou un incident | Qu'un AED garde un accès après son contrat ; ne pas joindre un professeur en cours | Les AED gérés par elle (arrivée, accès limité, fin) ; « où est-il maintenant » d'après l'emploi du temps | quotidienne |
| **AED, surveillant** | Contrat court, souvent étudiant, a besoin de l'appel et des absences, pas des notes | Être accusé d'avoir consulté ce qu'il ne devait pas | Un accès net, limité, qui s'arrête seul | quotidienne |
| **AESH** | Accompagne un ou plusieurs élèves, parfois sur plusieurs établissements | Ne pas avoir accès à l'emploi du temps et au cahier de textes de l'élève accompagné ; ou en voir trop | Accès aux seuls élèves accompagnés (socle 35), sur la période | quotidienne |
| **Agents (accueil, entretien, restauration, maintenance)** — souvent employés de la collectivité dans le public [NV] | Absents du logiciel ou présents par un compte générique partagé | Être oubliés lors d'une alerte ; recevoir des données d'élèves inutiles | Être dans l'annuaire et la liste de mise en sûreté ; un accès seulement si la fonction le demande (accueil, cuisine) | variable |
| **Personnels de l'organisme de gestion (privé)** | Gérés à part par l'OGEC ; le logiciel ne sait pas qui ils sont | Que leurs données d'employeur (salaire, contrat) circulent | Une fiche d'établissement simple, sans données de paie (socle 49) | variable |
| **Intervenant extérieur** (artiste, association, policier de prévention, animateur en pastorale) | Reçoit une liste papier ; ne fait pas l'appel ; n'existe nulle part | Être tenu pour responsable sans rien avoir | Être connu (dates, groupe, référent), éventuellement faire l'appel de son atelier | ponctuelle |
| **Personnel itinérant** (psychologue de l'Éducation nationale, assistante sociale, infirmière de secteur, médecin) | Plusieurs établissements, jours de présence | Que ses dossiers sensibles soient vus par d'autres ; être rattaché à tort à un établissement | Jours de présence affichés ; accès sensible strictement à son rôle | hebdomadaire |
| **Familles** | Ne savent pas qui est le professeur de sciences de leur enfant depuis le congé de la titulaire ; écrivent à l'ancien | Écrire au mauvais professeur ; ne pas être prévenues d'un changement | « L'équipe de mon enfant » toujours juste, avec un bouton pour écrire ; information quand un professeur change | ponctuelle |
| **Élèves** | Même besoin | — | « Mes professeurs », avec la matière et le bouton « écrire » | ponctuelle |
| **Organisme de gestion, tutelle, rectorat** | Demandent des listes (effectifs de personnels, enseignants par discipline) | Données fausses | Exports justes, minimaux, tracés | annuelle |
| **Domaines internes** | EDT, appel, notes, cahier, conseils, messagerie, sorties, examens, élections attendent : qui enseigne à qui, qui est PP, qui est dans l'équipe, qui est présent | Données périmées | Des ports de lecture fiables, datés, et des événements sans données personnelles | permanent |

---
## 4. Les situations réelles (catalogue de cas limites)

Chaque cas doit pouvoir être saisi **sans contournement** (pas de « compte partagé accueil », pas de « mettre le suppléant à la place du titulaire et remettre ensuite ») et produire les bons effets. Les règles statutaires citées sont de mémoire : **[NV]** partout, à faire vérifier avant les lots concernés (Q-P26).

### 4.1 Statuts et natures d'engagement

| Établissement | Natures d'engagement courantes [NV] | Employeur [NV] | Conséquences pour le logiciel |
|---|---|---|---|
| **Public (EPLE)** | professeur titulaire (certifié, agrégé, PLP, professeur des écoles en SEGPA/ULIS), stagiaire, contractuel (CDD, CDI), vacataire ; CPE ; documentaliste ; personnels de direction ; AED (contrat de droit public) ; AESH ; personnels administratifs, sociaux et de santé de l'État ; psychologue de l'Éducation nationale ; assistant de langue | État (ministère de l'Éducation nationale ; de l'Agriculture pour les lycées agricoles) | Personnels connus de STS-web et de l'AAF : l'**import** est la voie normale, la saisie l'exception (§12) ; connexion par le guichet Agents attendue (`INTEROPERABILITE_NATIONALE.md` §4) |
| | agents d'accueil, d'entretien, de restauration, de maintenance | **Collectivité** (département pour les collèges, région pour les lycées) | Pas dans STS-web [H] ; saisis ou importés d'un fichier ; employeur ≠ État : l'établissement n'est pas leur employeur, il les encadre au quotidien |
| **Privé sous contrat** | maîtres contractuels ou agréés (agents publics rémunérés par l'État), délégués auxiliaires, suppléants ; chef d'établissement (nommé par la tutelle) | État pour les maîtres ; organisme de gestion (OGEC) ou tutelle pour la direction | Maîtres présents dans STS-web [H, `INTEROPERABILITE_NATIONALE.md` §6] ; accès au guichet Agents des maîtres **[NV]** |
| | personnels de vie scolaire, administratifs, comptables, éducateurs d'internat, agents, animateurs en pastorale, cuisine | **Organisme de gestion** (droit privé, convention collective [NV]) | Saisis ; le contrat et la paie relèvent du socle 49 |
| **Privé hors contrat** | tous salariés de droit privé ; bénévoles parfois | Association ou société gestionnaire | Saisie ou import CSV ; aucun flux national de personnels [H] |
| **Tous** | intervenants extérieurs (artiste, association sportive, prévention), bénévoles (accompagnateurs, aide aux devoirs), service civique, apprentis, stagiaires (dont élèves en stage d'observation), personnels itinérants (psychologue, assistante sociale, infirmière, médecin), personnels mis à disposition par un autre établissement | variable, parfois aucun | Engagement daté obligatoire ; compte **facultatif** ; droits limités par défaut |

### 4.2 Cas limites

| # | Situation | Ce que le socle enregistre | Effets attendus |
|---|---|---|---|
| P1 | Titulaire à temps plein, toute l'année | 1 agent, 1 engagement `titulaire` sans fin, fonction `enseignant`, discipline de poste, services de l'année | Rien à faire en cours d'année ; reconduction proposée à la préparation de N+1 |
| P2 | **Complément de service dans deux établissements ScolIA** | 1 identité (D-013), 2 agents (un par établissement), chacun son engagement et sa quotité ; aucun lien visible entre eux | Chaque établissement ne voit que sa part ; l'agent seul voit ses deux emplois du temps ensemble (vue « mes établissements », au niveau de la couche identité, jamais par les établissements) ; il peut **transmettre** ses créneaux pris ailleurs comme indisponibilités, sans dire où |
| P3 | Complément de service dans un établissement hors ScolIA | Indisponibilités déclarées (socle 10) ; quotité partielle | Pas de convocation sur ces créneaux (socles 19, 23) |
| P4 | Poste vacant à la rentrée, contractuel nommé le 15 octobre | Service créé sans intervenant (`vacant`) dès la préparation ; intervenant ajouté le 15 octobre | Alerte « service sans professeur » dès juin ; cours affichés « non assuré » (socle 10) ; à l'arrivée : droits sur les élèves du service, familles informées du nom du professeur (socle 8) |
| P5 | **Suppléance longue** (congé de maternité, de maladie) | Titulaire **reste** titulaire du service (engagement actif, absence au socle 15) ; suppléant ajouté comme intervenant `suppleant` du 12 janvier au 3 avril, avec deux jours de recouvrement facultatifs | Suppléant : appel, notes, cahier, appréciations sur la période ; il voit le cahier et les évaluations antérieures du service ; titulaire : lecture selon le paramétrage (Q-P5), plus de notifications ; au retour, saisie rendue au titulaire, rien à copier ; famille : « Mme B. remplace M. A. jusqu'au 3 avril » |
| P6 | Suppléance en cascade (le suppléant est lui-même absent) | Deuxième intervenant `suppleant` sur une sous-période | Une seule personne en saisie à une date donnée (règle §8.5) ; historique lisible |
| P7 | Départ en cours d'année (retraite au 1ᵉʳ février, démission, fin anticipée de contrat, mutation) | Fin d'engagement datée, motif **codé** (pas de texte) ; parcours de départ | Services : successeur ou `vacant` ; missions transférées ou vacantes ; délégations qu'il a accordées révoquées ; accès fermé le soir de la date de fin ; ses saisies restent signées de son nom |
| P8 | **Décès** d'un personnel | Fin d'engagement datée, motif `deces` ; fait de vie sur la personne (socle 4) | Accès fermé, aucune relance ni message automatique (« bienvenue », « tâches en retard », « votre passkey ») ; information des élèves et familles **rédigée par la direction** ; services vacants ; lien vers la cellule d'écoute (socle 36) |
| P9 | **Suspension conservatoire** d'un agent (enquête, faits graves) | Accès `suspendu` immédiat, motif réservé à la direction ; engagement non clos | Toutes les sessions fermées sur cet établissement, notifications coupées ; élèves et familles voient seulement « absent » ; services confiés à un suppléant ; rien de visible pour les collègues hors direction |
| P10 | Congé long (maladie, formation, disponibilité) | Engagement actif, absence au socle 15, ou engagement `suspendu` pour une disponibilité | Accès conservé par défaut, **notifications en pause** (droit à la déconnexion [NV]) ; l'agent peut choisir de suspendre lui-même son accès |
| P11 | Temps partiel, mi-temps thérapeutique | Quotité de l'engagement, jours travaillés | Pas de convocation les jours non travaillés ; heures dues ajustées dans la répartition |
| P12 | Professeur stagiaire à mi-temps, avec tuteur | Engagement `stagiaire` ; mission `tuteur` donnée à un collègue, périmètre = l'agent stagiaire | Le tuteur voit l'emploi du temps et le cahier du stagiaire ; **pas** ses élèves par défaut (droit ajoutable par le modèle de la mission) |
| P13 | Co-enseignement (deux professeurs sur un même service : sciences, discipline non linguistique, co-intervention en lycée professionnel, ULIS) | Deux intervenants `co_enseignant` actifs, heures réparties | Les deux saisissent ; l'appel est fait une fois par séance (premier qui valide, l'autre voit) ; bulletin : un ou deux noms selon paramétrage |
| P14 | Groupe interclasses (allemand LV2 de 4A, 4B, 4C) | Service sur le **groupe** (socle 5 porte le groupe et ses élèves) | Le professeur voit les élèves du groupe, pas les classes (D-022) |
| P15 | Demi-groupes de TP avec deux professeurs | Deux services (un par demi-groupe) ou un service avec deux intervenants selon l'organisation | Chaque professeur voit son demi-groupe |
| P16 | Option ou module semestriel, stage de six semaines, accompagnement personnalisé tournant | Service avec dates de début et de fin dans l'année | Droits sur la période seulement |
| P17 | **Deux professeurs principaux** dans une classe (terminale [NV], ou choix de l'établissement) | Deux missions `professeur_principal`, même périmètre | Tous deux ont les droits du modèle ; la famille voit les deux ; la cardinalité maximale est un paramètre |
| P18 | PP qui n'enseigne pas dans la classe (documentaliste, CPE dans certains dispositifs [H]) | Mission autorisée, avec avertissement | Les droits viennent de la mission, pas d'un service |
| P19 | Changement de PP en cours d'année | Fin de la première mission, début de la seconde | Droits basculent à la date ; conversations en cours avec les familles **ne sont pas transférées** automatiquement (le nouvel arrivant voit seulement ce qui est adressé « au PP de la classe », socle 16) |
| P20 | **Professeur parent d'un élève** de l'établissement | Une personne (socle 4) : un lien de responsabilité **et** un agent ; deux casquettes | Il agit comme parent dans la casquette parent (justifier, signer) ; comme professeur, accès à son propre enfant **signalé et tracé** ; s'il enseigne à son enfant, avertissement à la direction ; noter son enfant reste possible sauf paramétrage contraire (Q-P3) |
| P21 | Parent bénévole ou intervenant (animatrice de l'atelier théâtre, accompagnatrice de sortie) | Même personne ; engagement `benevole` daté ; responsabilité du groupe ou rôle d'encadrante (socle 42) | Casquette « intervenant » limitée au groupe et à la période ; sa casquette parent inchangée |
| P22 | Ancien élève devenu AED | Nouvelle personne (le dossier élève est au socle 2 ; aucun lien automatique) | Rien de particulier ; ne pas rapprocher les deux dossiers |
| P23 | AED étudiant, contrat d'un an renouvelable [NV] | Engagement `aed` daté ; fonction `surveillant` ; mission `surveillance_internat` éventuelle | Accès appel, absences, vie scolaire selon le modèle « Assistant d'éducation » ; fermeture seule à la date ; renouvellement = nouvel engagement, même agent |
| P24 | **AESH** sur plusieurs élèves et plusieurs établissements (organisation en pôle [NV]) | Agent dans chaque établissement ; l'**accompagnement** élève par élève est au socle 35 | Accès aux seuls élèves accompagnés (périmètre fourni par le socle 35 via une mission, §8.6) |
| P25 | Agents de la collectivité (cuisine, entretien, accueil) | Agent, engagement `agent_collectivite`, employeur = organisme collectivité (socle 4 `ent_organisme`) ; compte **seulement** si la fonction l'exige | Annuaire interne, liste de mise en sûreté (socle 14) ; accueil : rôle « Accueil » ; cuisine : rôle « Restauration » (socle 40) |
| P26 | Personnel de l'organisme de gestion (privé) | Engagement `salarie_organisme`, employeur = OGEC | Socle 49 lira l'engagement plus tard ; ici, aucune donnée de contrat |
| P27 | Chef d'un ensemble scolaire (collège + lycée, deux numéros UAI) | Selon le découpage des établissements du socle 5 : un établissement à deux UAI, ou deux établissements | **Dépend du socle 5** (§5.4) ; dans le second cas : deux agents, aucun lien visible |
| P28 | Animateur en pastorale (privé catholique) | Fonction créée par l'établissement ; responsable de groupes | Droits limités à ses groupes |
| P29 | **Intervenant extérieur ponctuel** (artiste en résidence dix séances, policier de prévention une séance) | Agent, engagement `intervenant` daté, référent interne obligatoire ; responsabilité d'un groupe ou présence sur des séances | Sans compte par défaut (le référent fait l'appel) ; compte invité limité possible (Q-P10) |
| P30 | Assistant de langue étrangère (quelques mois [NV]) | Engagement `assistant_langue` ; co-intervenant sans saisie de notes | Voit les élèves des services où il intervient, sans notes par défaut |
| P31 | Personnel itinérant (psychologue de l'Éducation nationale, assistante sociale, infirmière de secteur) | Agent, quotité ou jours de présence | Droits sensibles via son rôle (`psy.*`, `sante.*`) ; jours de présence affichés aux collègues |
| P32 | Service civique, apprenti mineur, stagiaire (dont un élève de 3ᵉ en stage au secrétariat) | Engagement daté ; **personnel mineur** signalé | Aucun accès aux données d'élèves par défaut ; données minimales ; un élève stagiaire n'a jamais de droits sur d'autres élèves |
| P33 | **Intérim de direction** (chef d'établissement absent, adjoint faisant fonction) | Délégation datée des missions et de la signature (§8.9) | Les droits et pouvoirs de signature passent à l'adjoint sur la période, puis reviennent seuls |
| P34 | Changement de nom d'un agent | Version d'identité de la personne (socle 4) | Affichage partout ; historique conservé |
| P35 | Agent menacé (identité à protéger des familles et des élèves) | Indicateur « identité restreinte » posé par la direction | Familles et élèves voient la civilité et l'initiale, ou un intitulé de fonction (Q-P14) ; collègues inchangés |
| P36 | **Trente arrivées le même jour** (rentrée) | Import STS-web ou CSV ; invitations en lot ; séance d'enrôlement collective | Tableau « qui a activé son accès » ; relance groupée ; enrôlement par QR code |
| P37 | Contrat qui finit le 7 juillet, conseils et bulletins juste avant [NV] | Date de fin ; tâches en cours signalées dès J-15 | Alerte à l'agent et à la direction : « appréciations non saisies » ; délai de grâce **en lecture** paramétrable (Q-P6) |
| P38 | Retour d'un agent parti (réembauché l'année suivante) | Même agent, nouvel engagement | Historique conservé ; même identité ; nouvelle invitation si l'accès a été fermé |
| P39 | Mutation dans une autre académie ou un autre établissement ScolIA | Fin d'engagement ici ; ailleurs, nouvel agent créé par l'autre établissement | L'identité et la passkey restent (D-013) ; aucune donnée ne suit sans action de la personne |
| P40 | Agent sans téléphone ni ordinateur personnel, ou qui refuse d'utiliser ses appareils personnels | Passkey sur une **clé de sécurité** fournie par l'établissement, ou sur un poste de l'établissement | D-009 tenu sans imposer d'appareil personnel (question juridique [NV], Q-P18) |
| P41 | Erreur de saisie d'un service (mauvais professeur pendant trois semaines, notes déjà saisies) | **Correction** de l'intervention (pas un changement) | Les notes gardent leur auteur réel ; les droits sont recalculés ; journal ; aucune note déplacée |
| P42 | Agent qui a deux fonctions (professeur et coordinateur numérique ; CPE et responsable d'internat) | Plusieurs fonctions et missions | Droits additionnés par les rôles (D-017) |
| P43 | Personnel mis à disposition par un autre établissement (professeur d'un lycée qui assure une option au collège voisin) | Engagement `mis_a_disposition` ; employeur inchangé | Comme P2 |
| P44 | Personnel dont l'e-mail académique change | Coordonnée remplacée (socle 4) ; l'identité reste | Pas de nouveau compte |
| P45 | Enseignant d'un dispositif hors classe (ULIS, UPE2A, classe relais) | Service sur un groupe de dispositif | Voit les élèves du dispositif ; les professeurs des classes d'inclusion voient les élèves inclus dans leurs services |
| P46 | Équipe éducative ou de suivi réunie autour d'un élève | Équipe constituée ponctuelle, membres datés, objet = élève | Liste de diffusion et convocation (socle 19) ; **aucun droit** par l'équipe seule (Q-P9) |

### 4.3 Cas limites ajoutés par la passe personas [personas]

| # | Situation | Ce que le socle enregistre | Effets attendus |
|---|---|---|---|
| P47 | **Maître contractuel (État) qui fait aussi des surveillances payées par l'OGEC** | Deux engagements **simultanés** : `maitre_contractuel` (employeur État) et `salarie_organisme` (OGEC), quotités distinctes (R-P1) | Un seul agent ; la paie (49) ne lit que l'engagement OGEC ; les services d'enseignement suivent le premier, le planning de surveillance (PRS-133) le second |
| P48 | **AED qui est aussi étudiante en BTS du même lycée** | Casquette personnel et casquette élève (majeure) sur la même personne | La portée de vie scolaire **exclut sa propre classe et ses groupes** (R-P5) ; alerte à la direction à la création ; ses consultations de camarades sont impossibles, pas seulement tracées |
| P49 | **Contractuelle renouvelée de mois en mois** ; un renouvellement saisi le 2 janvier pour un contrat fini le 31 décembre | Engagements successifs ; prolongation | Alertes J-7 et J-1 (« l'accès se fermera ce soir ») ; si la prolongation arrive après la fermeture, réouverture **sans nouvelle invitation** dans les 30 jours (R-P9) |
| P50 | **Contractuel nommé le lundi, arrivé le jeudi** ; procès-verbal d'installation signé à l'arrivée [NV] | Date prévue (`du`) et date de prise de fonctions réelle (`installe_le`, PRS-116) | Droits élèves à la prise de poste, pas au premier cours (R-P3) ; le procès-verbal rejoint « mes documents » (PRS-140) |
| P51 | **Titulaire sur zone de remplacement** : trois suppléances dans trois établissements ScolIA, l'une prolongée cinq fois | Un agent par établissement (D-013) ; interventions `suppleant` datées ; prolongations | Chaque prolongation décale la fin **sans** nouvelle annonce aux familles (R-P19) ; « mes établissements » pour elle seule |
| P52 | **Inspectrice** en visite pour une professeure (rendez-vous de carrière [NV]) ; **formateur** qui visite un stagiaire | Accès temporaire de consultation (PRS-125), casquette `intervenant` datée d'un jour | Lecture du cahier, de la progression, de l'emploi du temps de l'enseignante et de la liste de la classe visitée ; aucune donnée de vie scolaire ni de santé ; l'enseignante est informée (R-P8) |
| P53 | **Infirmière qui intervient deux heures** dans chaque classe de 4ᵉ (éducation à la sexualité [NV]) ; policier de prévention une séance | Intervention ponctuelle sur des séances (PRS-124), sans service | Liste de la séance, le jour de la séance ; figure dans l'équipe de la classe ces jours-là |
| P54 | **Agent multi-établissements qui perd sa passkey le jour de la rentrée** | Récupération D-018 en cours (délai de 24 à 48 h) ; clé de prêt locale (PRS-120) | Accès à **cet** établissement seulement, au plus 48 h, alerte à la personne et dans ses autres établissements (R-P16) |
| P55 | **Professeure en disponibilité un an** (ou détachement, congé parental) | Suspension datée de l'engagement (PRS-147, R-P4), engagement non clos | Accès suspendu par défaut ; services en suppléance ou vacants ; missions à transférer ; retour : suspension levée, rien à recréer |
| P56 | **AED qui devient contractuel enseignant en mars** | Fin de l'engagement `aed`, nouvel engagement `contractuel` ; nouvelle fonction (PRS-146) | Rôles proposés de nouveau (le modèle « Assistant d'éducation » n'est pas gardé par défaut) ; historique conservé |
| P57 | **Vacataire payé à l'heure** (hors contrat, études de l'OGEC) | Engagement `vacataire` ; interventions | Relevé du **service fait** par séance assurée (PRS-145), base de la paie (49) |
| P58 | **AESH** qui accompagne un élève d'ULIS inclus en 5B pour deux cours | Mission AESH (socle 35) sur l'élève ; inclusions du socle 5 (STR-102) | Elle reçoit les changements de classe, de groupe et d'inclusion de l'élève (R-P20) ; elle figure dans l'équipe de la 5B pour les conseils (R-P21) |
| P59 | **Professeur dont le fils du voisin** (ou une nièce) est dans sa classe | Lien personnel déclaré (PRS-137) | Déport proposé au conseil de classe, à la constitution, au conseil de discipline ; consultations du dossier tracées à part (R-P6) |
| P60 | **Bénévole d'aide aux devoirs** et intervenants d'un établissement hors contrat | Engagement `benevole` ou `intervenant` ; vérification d'honorabilité datée (PRS-143) [NV] | Échéance de renouvellement signalée ; aucune intervention planifiée sans vérification à jour si l'établissement l'exige |

---

## 5. Frontières

### 5.1 Tableau

| Sujet | Socle 1 (sécurité) | Socle 4 (personnes) | Socle 5 (structure) | **Socle 6 (ce document)** | Autres socles |
|---|---|---|---|---|---|
| Identité au niveau de l'ENT, passkeys, sessions, SSO, moteur `autoriser` | **possède** | — | — | appelle (invitation, fermeture d'accès, révocation) | 9 (fédération nationale) |
| Personne (nom, prénom, civilité, coordonnées) | — | **possède** | — | crée et lit **par les ports du socle 4** ; décide des usages professionnels | — |
| Accès d'une personne à l'établissement (état, dates, casquettes) | **possède** la table `ent_membership` et le contrôle de session | — | — | **pilote** l'accès des personnels (ouverture, suspension, fermeture) par commandes du socle 1 | 2 (élève), 4 (familles) pilotent les leurs |
| Fiche agent, engagements, fonctions, disciplines de poste, qualifications utiles | — | — | — | **possède** | 49 (contrat, paie, congés, entretiens) lira |
| Classes, groupes, élèves des groupes, matières, niveaux, sites, années, heures prévues par matière | — | — | **possède** | lit | 2 (affectations des élèves) |
| Services d'enseignement et intervenants | — | — | — | **possède** | 10 (créneaux), 12 (appel), 20, 21, 23 s'y rattachent |
| Professeur principal, référents, coordinateurs, tuteurs, missions de l'établissement | — | — | — | **possède** | 35 (accompagnement AESH) et 39 (tuteur de stage) **appellent** le port de missions |
| Responsables de groupes et d'ateliers | — | — | le groupe | **possède** la responsabilité | 43 (activités) |
| Équipes calculées et constituées | — | — | — | **possède** | 16 (listes de diffusion), 19 (réunions), 23 (conseils) lisent |
| Instances élues (conseil d'administration, conseil pédagogique s'il est élu [NV]) | — | — | — | fournit le **corps électoral** des personnels | **46** possède |
| Rôles et attributions | **possède** le moteur | — | — | crée des attributions **issues** des missions, délégations et fonctions | — |
| Absences des personnels, recherche et organisation des remplacements, heures supplémentaires effectives | — | — | — | fournit l'agent et le port « nommer un suppléant » | **15** possède |
| Créneaux, séances, remplacement d'une séance, indisponibilités et vœux d'emploi du temps | — | — | — | fournit services, intervenants, quotité et jours travaillés | **10** possède |
| Accompagnement d'un élève par un AESH (notification, heures) | — | — | — | l'agent AESH | **35** possède |
| Encadrants d'une sortie, ordres de mission | — | — | — | qualifications utiles, agents et bénévoles | **42** possède |
| Clés, badges, matériel prêté au personnel | — | — | — | étape du parcours d'arrivée et de départ | **44** possède |
| Contrats, rémunération, congés, formation, entretiens, dossier du personnel, déclarations sociales | — | — | — | engagement (nature, dates, employeur, quotité) | **49** possède |
| Pièces (charte, lettre de mission, arrêté de nomination) | — | — | — | qualifie la pièce | **7** stocke |
| Import STS-web, AAF, annuaire GAR | — | — | structures | **cible** des imports, source des exports de personnels et de services | **9** possède le cadre d'import et les connecteurs |

### 5.2 Règles de frontière [R]

1. **Personne n'écrit dans les tables du socle 6** hors de ses commandes. Les socles 10, 15, 35, 39 et 42 appellent ses ports ; les imports (socle 9) produisent des propositions ou des commandes.
2. **Le socle 6 n'écrit pas les personnes** : il appelle `personnes::correspondances` puis `personnes::creer` ou `personnes::rattacher` du socle 4 (`04_ADMINISTRATIF.md` §5.3), ce qui donne une seule personne au professeur-parent.
3. **Le socle 6 n'écrit pas les tables du socle 1** : il appelle des commandes `acces::ouvrir`, `acces::suspendre`, `acces::fermer`, `attributions::poser`, `attributions::clore` (à créer au socle 1, lot P0), qui gardent l'anti-escalade et le garde-fou « dernier administrateur ».
4. **Le socle 6 ne porte aucune donnée de paie ni de contrat de travail**, ni motif détaillé d'absence ou de départ.
5. **Garde de code** (comme `04_ADMINISTRATIF.md` §5.2 point 4) : un test lit le SQL des autres modules et échoue s'il contient `INSERT`, `UPDATE` ou `DELETE` sur `ent_agent*`, `ent_service_enseignement`, `ent_service_intervenant`, `ent_mission*`, `ent_equipe*`, `ent_groupe_responsable`, `ent_delegation`, `ent_parcours_agent*`, `ent_revue_acces*`.

### 5.3 Ports exposés

Chaque port est une fonction Rust ; les écritures prennent la transaction en cours (`&mut TenantTx`) ; toutes les lectures prennent une **date** (Europe/Paris).

| Port | Appelé par | Sens | Remarques |
|---|---|---|---|
| `personnels::agent_du_compte(user_id, date) -> Option<AgentActif>` | 1 (session, casquette), tous | lecture | Agent actif avec engagement à la date |
| `personnels::annuaire(filtre, date) -> Vec<AgentResume>` | 16, 19, 29, 42 | lecture | Selon les droits du lecteur (§13.3) |
| `services::intervenants(service_id, date) -> Vec<Intervention>` | 10, 12, 20, 21, 23 | lecture | Nature, droits de l'intervention |
| `services::en_saisie(service_id, date) -> Option<AgentId>` | 12 (appel), 21 (notes) | lecture | L'intervenant qui saisit à cette date (§8.5) |
| `services::de_l_agent(agent_id, periode) -> Vec<ServiceResume>` | 10, 15, 29 | lecture | « Mes services » |
| `services::de_l_eleve(eleve_id, date) -> Vec<(ServiceResume, Vec<AgentResume>)>` | 2 (fiche), famille, élève | lecture | « L'équipe de mon enfant » |
| `services::nommer_suppleant(service_id, agent_id, du, au, origine)` | **15**, direction | écriture | Origine = absence du socle 15 ou vacance ; refuse deux saisies simultanées |
| `services::eleves_vus(agent_id, date) -> PerimetreEleves` | 1 (`autoriser`) | lecture | Remplace `eleves_suivis_par_prof` (§9) |
| `missions::actives(type, perimetre, date) -> Vec<Mission>` | 16, 19, 23, 37 | lecture | « Le PP de la 4B », « le référent harcèlement » |
| `missions::attribuer(type, agent_id, perimetre, du, au, origine)` | **35** (AESH : périmètre = élèves), **39** (tuteur de stage), direction | écriture | Le socle appelant reste propriétaire de sa décision ; la mission porte l'accès |
| `equipes::pedagogique(classe_id ou groupe_id, date) -> Vec<MembreEquipe>` | 16, 19, 23 | lecture | Calculée (§8.8) |
| `equipes::membres(equipe_id, date)` | 16, 19, 46 | lecture | Constituée |
| `personnels::electeurs(college_electoral, date)` | 46 | lecture | Collèges électoraux des personnels à définir avec le socle 46 [NV] |
| `personnels::qualifies(qualification, date) -> Vec<AgentId>` | 14 (secourisme pour la mise en sûreté), 42 | lecture | Qualification en cours de validité |
| `personnels::explique_acces(agent_id, eleve_id, action, date) -> Explication` | 1, écrans | lecture | Chaîne : rôle → portée → service / mission / groupe → dates (§9.5) |

### 5.4 Points à accorder avec le socle 5 (conception parallèle, non consultée)

1. **Groupes** : le socle 5 porte le groupe et ses élèves (affectations, socle 2) ; le socle 6 porte qui l'enseigne (service) et qui en est responsable (atelier). Le groupe doit pouvoir être créé **avant** qu'un service y soit rattaché.
2. **Heures prévues par matière et par niveau** (offre de formation, horaires des programmes [NV]) : nécessaires au contrôle de couverture de la répartition (§10.4). Recommandation : le socle 5 les porte, le socle 6 les lit.
3. **Année en préparation** : la répartition des services de N+1 suppose que le socle 5 ouvre une année « en préparation » avec ses classes et groupes prévus (`SOCLES.md` §3 socle 5 : bascule d'année).
4. **Événements de bascule** consommés ici : `annee.ouverte`, `annee.cloturee`, `classe.archivee`, `groupe.archive` (noms à confirmer).
5. **Établissement à plusieurs UAI et ensembles scolaires** (cas P27, `SOCLES.md` §7 « Ensembles scolaires ») : la granularité de l'établissement décide si un personnel commun est un agent ou deux.
6. **Matières** : le service pointe une matière du socle 5 (code national si importé) ; `matiere_libelle` texte reste une projection d'affichage.
7. **Sites** : la portée « sites désignés » proposée par les socles 2 et 4 doit s'appliquer aussi aux personnels (un secrétariat de site).

---

## 6. Concepts

### 6.1 Vocabulaire fixé [R]

| Concept | Définition | Table |
|---|---|---|
| **Agent** | Une personne en tant qu'elle travaille ou intervient dans **cet** établissement, quel que soit son statut (« personnel » au sens large : enseignant, AED, agent, intervenant, bénévole) | `ent_agent` |
| **Engagement** | Une période continue de présence à un titre donné (nature, employeur, quotité, dates, motif d'arrivée et de départ codés). Un agent en a un ou plusieurs, successifs | `ent_agent_engagement` |
| **Fonction** | Ce que fait l'agent (enseignant, CPE, AED, secrétaire, agent d'accueil…) : code du logiciel, libellé de l'établissement, fonctions créées par l'établissement. Informative ; elle **propose** des modèles de rôles, elle n'accorde rien | `ent_agent_fonction` |
| **Discipline de poste** | Ce qu'un enseignant est qualifié à enseigner (code national si importé). Sert à la répartition, pas aux droits | `ent_agent_discipline` |
| **Qualification** | Compétence datée utile à l'organisation : secourisme, encadrement, permis, langue, habilitation | `ent_agent_qualification` |
| **Accès** | Le droit de se connecter à l'établissement, pour une casquette : ouvert, suspendu, fermé, avec dates | `ent_membership` (socle 1, enrichie) |
| **Casquette** | Le titre sous lequel une session agit : personnel, parent, élève, intervenant | `ent_membership_casquette` (socle 1) |
| **Service d'enseignement** | Une unité d'enseignement d'une année : matière × cible (classe ou groupe) × période × heures × modalité | `ent_service_enseignement` (existante, enrichie) |
| **Intervention** | La présence d'un agent sur un service, avec nature (titulaire, co-enseignant, suppléant, assistant, intervenant), dates, heures et droits | `ent_service_intervenant` |
| **Service vacant** | Service sans intervenant en saisie à une date | (calculé) |
| **Mission** | Une responsabilité confiée à un agent sur un périmètre et une période : professeur principal, référent, coordinateur, tuteur, mission créée par l'établissement. Peut porter un modèle de rôle | `ent_mission_type`, `ent_mission` |
| **Périmètre** | Ce sur quoi porte une mission ou une attribution : établissement, site, niveaux, classes, groupes, élèves désignés, agents désignés, discipline | `ent_mission_perimetre`, colonne de l'attribution |
| **Responsabilité de groupe** | Un agent responsable ou animateur d'un groupe ou d'un atelier, datée | `ent_groupe_responsable` (existante, enrichie) |
| **Équipe calculée** | Équipe définie par une règle (équipe pédagogique d'une classe, d'un niveau, d'une discipline) | (règles, pas de table de membres) |
| **Équipe constituée** | Équipe à membres choisis et datés (conseil pédagogique, cellule de veille, équipe d'un projet, équipe éducative autour d'un élève) | `ent_equipe`, `ent_equipe_membre` |
| **Délégation** | Transfert daté, motivé, révocable, d'une mission, d'un rôle ou d'un pouvoir de signature d'un agent à un autre | `ent_delegation` |
| **Pouvoir de signature** | Habilitation à signer au nom de l'établissement certains documents (certificat de scolarité, convention de stage, ordre de mission, bulletin) | `ent_habilitation_signature` |
| **Parcours** | Liste d'étapes d'arrivée ou de départ d'un agent, cochées à la main ou par les faits | `ent_parcours_agent`, `ent_parcours_etape` |
| **Revue des accès** | Vérification datée, par la direction, de la liste des accès du personnel et de leurs droits | `ent_revue_acces` |

### 6.2 Pourquoi l'agent est une personne du socle 4 [R] (Q-P1)

Trois options : (a) l'agent est un compte (`ent_user`), comme aujourd'hui ; (b) l'agent a sa propre identité dans le socle 6 ; (c) l'agent est une **personne** du socle 4 avec une fiche d'agent à côté.

(a) est écartée : un service, une mission, une suppléance doivent exister **avant** le compte (rentrée préparée en juin, import STS-web, intervenant sans compte). (b) crée deux fiches pour le professeur-parent, deux jeux de coordonnées, et contredit la contrainte déjà posée « une personne par compte et par établissement » [F `120` : `UNIQUE (etablissement_id, user_id)` sur `ent_personne`, citée par `04_ADMINISTRATIF.md` §2.1]. (c) donne une personne unique par humain dans l'établissement, des coordonnées tenues une fois, un rapprochement de doublons commun (`personnes::correspondances`), et laisse au socle 6 tout ce qui est propre au travail.

**Garde-fou indispensable** avec (c) : les **groupes de lecture** du socle 4 ne doivent jamais exposer à une famille quoi que ce soit d'un agent, et les coordonnées d'une personne ont des **usages** (`pro`, `perso`, `urgence`) dont la visibilité dépend du lecteur (§13.3). La modification de l'identité d'une personne qui a un agent actif exige `personnels.gerer` **ou** `personnes.gerer` (les deux secrétariats peuvent corriger une faute de frappe), et la commande prévient l'autre domaine par événement.

### 6.3 Le mot « professeur » a quatre sens : on les sépare

| Sens | Question | Porté par | Exemple qui casse si on confond |
|---|---|---|---|
| **Fonction** | Que fait-il ici ? | `ent_agent_fonction` | Une documentaliste PP n'a pas de service mais a une mission |
| **Profil de compte** (ancien `ent_membership.role = prof`) | Quel modèle de droits par défaut ? | socle 1 | Un professeur qui est aussi coordinateur numérique a deux rôles |
| **Intervenant d'un service** | À qui enseigne-t-il, quand ? | `ent_service_intervenant` | La suppléante enseigne sans être « professeur de l'établissement » toute l'année |
| **Titulaire du service** | Qui en répond sur l'année ? | intervention de nature `titulaire` | Pendant la suppléance, le titulaire reste titulaire, sans saisir |

---

## 7. Le temps et l'historique

Mêmes trois mécanismes que `02_ELEVE.md` §6 : **lignes datées**, **journal en ajout seul** (`ent_journal_modif` [F]), **événements outbox** (charges utiles sans données personnelles).

| Donnée | Historique [R] | Nature d'une modification |
|---|---|---|
| Engagement | lignes datées, jamais supprimées ; une prolongation ferme et remplace (`remplace_id`) ou décale la fin | `arrivee`, `prolongation`, `modification` (quotité, nature), `fin`, `correction` |
| Fonctions, disciplines, qualifications | lignes datées | `ajout`, `fin`, `correction` |
| Accès | états datés (`ouvert`, `suspendu`, `ferme`) avec acteur et motif codé | `ouverture`, `suspension`, `retablissement`, `fermeture` |
| Service | colonnes datées ; changement de cible ou de matière = **nouveau service** qui remplace l'ancien (`remplace_service_id`) à une date | `creation`, `modification_heures`, `cloture`, `remplacement`, `correction` |
| Intervention | lignes datées | `ajout`, `fin`, `correction` (erreur d'agent : l'intervention erronée est **annulée**, pas close ; les saisies gardent leur auteur réel) |
| Mission, responsabilité de groupe, membre d'équipe, délégation, attribution | lignes datées, jamais écrasées | `attribution`, `fin`, `revocation`, `correction` |

Règles [R] :
- **Dates métier en Europe/Paris**, bornes **incluses** ; « jusqu'au 3 avril » = fin le 3 avril à 23 h 59, heure de Paris.
  - **[personas] Correction proposée (R-P2)** : lire « dans le **fuseau de l'établissement** » (profil du socle 5, STR-03 ; Europe/Paris par défaut), comme la passe 2–4 l'a fait pour les socles 2 à 4 (A0-a). Vaut pour tout ce document, y compris le lot P0 et le §8.
- **Le futur est normal** : arrivée, mission, suppléance, fin de contrat se saisissent à l'avance ; une tâche quotidienne applique les effets (ouverture et fermeture d'accès, attributions) et publie les événements **le jour J**, de façon idempotente. Les droits, eux, sont calculés à chaque requête (D-017 [F]) : la tâche ne sert qu'aux effets de bord (sessions, notifications).
- **Correction ≠ changement** : corriger l'agent d'une intervention saisie par erreur ne crée pas de « changement de professeur » pour les familles ; il n'y a pas d'information automatique.
- **Lecture à une date** : tous les ports prennent une date ; « qui enseignait l'histoire en 3C le 12 mars ? » a une réponse.
- **Une commande = une transaction** : fait, journal (noms de champs et identifiants), événement outbox.

---
## 8. Modèle de données cible

Notation : pseudo-DDL PostgreSQL, à préciser par le lot qui l'implémente. Toutes les tables portent `etablissement_id UUID NOT NULL`, des clés étrangères **composées** `(etablissement_id, x_id)` (qui supposent `UNIQUE (etablissement_id, id)` sur les tables cibles, comme aux socles 2 et 4), la RLS (§13.1), `cree_le`, `cree_par_user_id`. Les codes en `CHECK` sont des **codes du logiciel** ; les libellés affichés sont paramétrables. Dates métier en **Europe/Paris**, bornes incluses.

### 8.1 `ent_agent` (nouvelle)

```
ent_agent
  id UUID PK, etablissement_id
  personne_id UUID NOT NULL           -- socle 4 ; UNIQUE (etablissement_id, personne_id)
  matricule_interne TEXT NULL         -- numéro propre à l'établissement, facultatif
  nom_affiche_familles TEXT NULL      -- NULL = civilité + nom d'usage ; sinon choix encadré (§13.3, Q-P14)
  identite_restreinte BOOLEAN DEFAULT false   -- cas P35, posé par la direction seulement
  photo_document_id UUID NULL         -- socle 7 ; affichage selon consentement (§13.3)
  personnel_mineur BOOLEAN DEFAULT false      -- cas P32 (déduit de la date de naissance si connue, sinon déclaré)
  etat TEXT NOT NULL                  -- projection : prevu | actif | suspendu | parti | archive (calculé des engagements et de l'accès)
  notifications_en_pause_jusqu_au DATE NULL   -- choix de l'agent (cas P10)
  source TEXT NOT NULL                -- saisie | import_sts | import_aaf | import_csv | reprise
```

Pas de date de naissance, d'adresse, de nationalité, de situation de famille, de NUMEN (§13.4). L'identité (nom, prénom, civilité, nom d'usage) et les coordonnées sont celles de la **personne** (socle 4).

### 8.2 `ent_agent_engagement`

```
ent_agent_engagement
  id, etablissement_id, agent_id
  du DATE NOT NULL, au DATE NULL
  nature TEXT NOT NULL CHECK (nature IN (
     'titulaire','stagiaire','contractuel','vacataire',
     'maitre_contractuel','maitre_agree','delegue_auxiliaire',
     'aed','aesh','assistant_langue',
     'salarie_organisme','agent_collectivite','mis_a_disposition',
     'intervenant','benevole','service_civique','apprenti','stagiaire_ecole',
     'a_preciser'))                   -- liste [NV] ; libellés paramétrables ; 'a_preciser' pour la reprise
  employeur_type TEXT NOT NULL CHECK (employeur_type IN
     ('etat','collectivite','organisme_gestion','association','autre_etablissement','aucun','inconnu'))
  employeur_organisme_id UUID NULL    -- socle 4 ent_organisme (région, département, OGEC…)
  quotite_pct NUMERIC(5,2) NULL       -- part dans CET établissement (0 < q ≤ 100)
  jours_travailles SMALLINT[] NULL    -- 1 = lundi … 6 = samedi ; NULL = tous
  heures_dues_hebdo NUMERIC(4,2) NULL -- enseignants : service attendu dans l'établissement (Q-P22) ; aucune donnée de grade
  motif_arrivee TEXT NULL CHECK (… IN ('rentree','nomination','mutation','remplacement','creation_poste','renouvellement','autre'))
  motif_fin TEXT NULL CHECK (… IN ('fin_contrat','mutation','retraite','demission','deces','fin_mission','autre'))
                                       -- jamais de texte libre ; le motif détaillé relève du socle 49
  remplace_agent_id UUID NULL         -- engagement conclu pour remplacer quelqu'un (information, pas de droit)
  referent_agent_id UUID NULL         -- obligatoire pour 'intervenant', 'benevole', 'stagiaire_ecole', 'service_civique'
  remplace_engagement_id UUID NULL    -- prolongation, correction
  annule_le TIMESTAMPTZ NULL          -- correction d'une saisie erronée
  CHECK (au IS NULL OR au >= du)
  EXCLUDE USING gist (etablissement_id WITH =, agent_id WITH =, daterange(du, au, '[]') WITH &&)
     WHERE (annule_le IS NULL)        -- pas deux engagements simultanés dans un même établissement
```

**[personas] Correction proposée (R-P1, Q-P29)** : la contrainte d'exclusion ci-dessus interdit deux engagements simultanés. Or un maître payé par l'État fait aussi des surveillances payées par l'OGEC, un AED est aussi vacataire d'étude (cas P47, P57). L'exclusion doit porter sur `(agent_id, employeur_type, coalesce(employeur_organisme_id), nature)`, pas sur l'agent seul ; une alerte de qualité signale deux engagements simultanés de même employeur. **À changer dans la migration 352, avant le lot P1.** Voir aussi la suspension d'engagement (§8.14, R-P4) : le cas P10 parle d'« engagement suspendu », que le modèle ci-dessus ne permet pas.

Règles [R] : un engagement `intervenant`, `benevole`, `stagiaire_ecole`, `service_civique` **exige** une date de fin (au plus un an, prolongeable) et un référent ; un engagement sans fin pour un `contractuel` déclenche une alerte de qualité (« date de fin à saisir ? ») mais n'est pas bloqué (CDI [NV]).

### 8.3 Fonctions, disciplines, qualifications, contact d'urgence

```
ent_fonction                         -- catalogue : codes du logiciel + fonctions créées par l'établissement
  id, etablissement_id NULL          -- NULL = fourni par ScolIA
  code TEXT, libelle TEXT, famille TEXT CHECK (famille IN
     ('direction','enseignement','education','administration','sante_social','accompagnement',
      'service_agents','technique','intervention','autre'))
  modeles_roles_proposes UUID[]      -- modèles proposés à l'arrivée (jamais accordés seuls)
  code_national TEXT NULL            -- fonction STS / AAF si elle existe [NV]

ent_agent_fonction
  id, etablissement_id, agent_id, fonction_id, site_id NULL
  principale BOOLEAN, du DATE, au DATE NULL

ent_agent_discipline                 -- enseignants : qualification
  id, etablissement_id, agent_id, matiere_id NULL (socle 5), code_discipline_poste TEXT NULL [NV],
  principale BOOLEAN, du, au

ent_agent_qualification              -- utiles à la journée (socles 14, 42) ; pas de formation continue (49)
  id, etablissement_id, agent_id
  code TEXT CHECK (code IN ('secourisme','encadrement_sortie','permis_b','permis_transport',
                            'langue','habilitation_electrique','surveillance_baignade','autre'))
  precision TEXT NULL (≤ 80), obtenue_le DATE NULL, valable_jusqu_au DATE NULL
  piece_document_id UUID NULL        -- socle 7, facultative

ent_agent_contact_urgence            -- facultatif, sur décision de l'agent (Q-P12)
  agent_id, personne_id (socle 4), ordre, du, au
```

Fonctions fournies [R] (libellés modifiables) : chef d'établissement, adjoint, directeur délégué aux formations [NV], directeur d'école, gestionnaire ou adjoint gestionnaire, secrétaire de direction, secrétaire de scolarité, comptable, enseignant, documentaliste, CPE, AED, surveillant, éducateur d'internat, AESH, infirmier, assistant de service social, psychologue, médecin, agent d'accueil, agent d'entretien, agent de restauration, chef de cuisine, agent de maintenance, technicien informatique, technicien de laboratoire, animateur, intervenant extérieur, bénévole. L'établissement en ajoute (« animateur en pastorale », « coordinatrice ULIS », « référente vie lycéenne »…).

### 8.4 Accès et casquettes (tables du socle 1, pilotées par le socle 6)

Le socle 6 a besoin de quatre choses que le socle 1 n'a pas [F §2.2 n° 1, 4] : un **état** de l'accès, des **dates**, plusieurs **casquettes** pour un même compte, une **invitation** sans mot de passe.

```
ent_membership (existante)           -- (user_id, etablissement_id) conservée
  + etat TEXT NOT NULL DEFAULT 'ouvert' CHECK (etat IN ('ouvert','suspendu','ferme'))
  + etat_depuis TIMESTAMPTZ, etat_motif TEXT NULL CHECK (codes), etat_par_user_id UUID NULL
  role TEXT                          -- conservé : profil de base = modèle par défaut (D-017)

ent_membership_casquette (nouvelle)
  user_id, etablissement_id
  casquette TEXT CHECK (casquette IN ('personnel','parent','eleve','intervenant'))
  profil_base TEXT NULL              -- pour 'personnel' : admin | direction | administratif | cpe | compta | prof | infirmerie | psy
  source_ref UUID                    -- agent_id, personne_id (lien), eleve_id
  du DATE, au DATE NULL, etat TEXT (ouvert|suspendu|ferme)

ent_session (existante)
  + casquette TEXT NOT NULL DEFAULT 'principale'  -- casquette active de la session

ent_agent_invitation (nouvelle)
  id, etablissement_id, agent_id
  canal TEXT CHECK (canal IN ('email','qr'))
  email_cible TEXT NULL              -- copie de la coordonnée au moment de l'envoi
  jeton_hash TEXT UNIQUE, expire_le TIMESTAMPTZ   -- e-mail : 7 jours ; QR : 10 minutes [R]
  utilisee_le TIMESTAMPTZ NULL, user_id_rattache UUID NULL
  annulee_le TIMESTAMPTZ NULL
```

Règles [R] :
- **La session n'est valide que si la casquette de la session est ouverte à la date**, en plus des contrôles actuels (`auth_repo.rs:820-833`). Fermer un accès = état `ferme` + révocation des sessions **de cet établissement** (pas des autres, contrairement à `revoke_all_for_user` [F `auth_repo.rs:1139`] qui révoque tout) + suppression des jetons de notification push de cet établissement.
- **Une session = une casquette.** Le professeur-parent choisit à la connexion (ou bascule sans se réauthentifier, dans la fenêtre d'assurance de la session) entre « espace professeur » et « parent de Léa ». Les droits de la session sont ceux de la casquette ; les contrôles famille actuels (`parsed == Role::Parent`) deviennent « casquette = parent ». Raisons : aucune union accidentelle de droits relationnels et professionnels, interface claire, journal lisible (« a consulté en tant que parent »). Alternative écartée : union de toutes les casquettes (Q-P2).
- **Invitation** : l'agent reçoit un lien à usage unique (ou scanne un QR code affiché sur l'écran du secrétariat) ; s'il a déjà une identité ScolIA (autre établissement, ou parent ici), il se connecte avec sa passkey et **confirme** le rattachement (D-013) ; sinon il crée son identité et enrôle sa passkey **avant** tout accès (D-009, vague « personnels d'abord »). Plus aucun mot de passe temporaire (fin de `routes.rs:1073-1101`).
- L'accès d'un personnel s'ouvre **à la date de début de l'engagement** ; avant, l'invitation peut être acceptée et la passkey enrôlée, avec un accès réduit à « préparer mon arrivée » (son emploi du temps, le cahier de textes et la progression de ses services **sans données d'élèves**, la liste de ses collègues) [R].
- L'accès se ferme **le soir de la date de fin** du dernier engagement, sauf délai de grâce en lecture paramétré (Q-P6).

### 8.5 Services et interventions

```
ent_service_enseignement (existante, enrichie)
  id, etablissement_id, annee_id                 -- inchangés
  matiere_id, matiere_libelle                    -- inchangés (libellé = projection d'affichage)
  classe_id XOR groupe_id                        -- inchangé (CHECK existant)
  + code TEXT NULL                               -- code court affiché (« MATHS-4B »)
  + modalite TEXT NOT NULL DEFAULT 'cours' CHECK (modalite IN
       ('cours','tp','co_intervention','accompagnement','option','dispositif','autre'))
  + heures_hebdo NUMERIC(4,2) NULL               -- heures élève par semaine
  + heures_annuelles NUMERIC(6,2) NULL           -- pour les modules non hebdomadaires
  + du DATE NULL, au DATE NULL                   -- NULL = bornes de l'année
  + etat TEXT NOT NULL DEFAULT 'ouvert' CHECK (etat IN ('prevu','ouvert','clos','annule'))
  + source TEXT CHECK (source IN ('saisie','import_sts','import_edt','reconduction','reprise'))
  + code_national TEXT NULL                      -- identifiant STS-web du service [NV]
  + remplace_service_id UUID NULL                -- changement de cible ou de matière en cours d'année
  prof_user_id                                   -- CONSERVÉ en projection : intervenant en saisie aujourd'hui
                                                 -- (trigger), rendu NULLABLE (service vacant) ; retiré au lot P14
  professeur_principal                           -- CONSERVÉ en projection de la mission PP (trigger) ; retiré au lot P14

ent_service_intervenant (nouvelle)
  id, etablissement_id, service_id, agent_id
  nature TEXT NOT NULL CHECK (nature IN
     ('titulaire','co_enseignant','suppleant','assistant','intervenant','stagiaire_observateur'))
  du DATE NOT NULL, au DATE NULL
  heures_hebdo NUMERIC(4,2) NULL                 -- part des heures du service (co-enseignement)
  saisie BOOLEAN NOT NULL                        -- saisit appel, notes, cahier, appréciations à la date
  droits TEXT[] NULL                             -- restriction facultative : {appel, notes, cahier, appreciations}
  origine TEXT CHECK (origine IN ('saisie','absence','vacance','import','reconduction','reprise'))
  origine_ref UUID NULL                          -- absence du socle 15, lot d'import
  annule_le TIMESTAMPTZ NULL                     -- correction d'une erreur (cas P41)
```

Règles [R] :
- **Une intervention en saisie par service et par jour**, sauf co-enseignement (`co_enseignant` : toutes en saisie). Pendant une suppléance, la base **retire la saisie** au titulaire sur la période (calcul, pas écriture) : `en_saisie(service, date)` = co-enseignants actifs, sinon suppléant actif, sinon titulaire actif. Le titulaire garde la **lecture** selon Q-P5.
- **Valeurs par défaut par nature** (paramétrables) : `titulaire`, `co_enseignant`, `suppleant` : appel, notes, cahier, appréciations ; `assistant`, `intervenant` : appel facultatif, cahier en lecture, pas de notes ; `stagiaire_observateur` : lecture de la liste seulement.
- **Service vacant** = aucune intervention en saisie à la date ; alerte dès qu'un service ouvert est vacant plus de N jours ouvrés (paramètre) ; les séances du service sont « non assurées » pour l'appel (le socle 12 n'attend pas d'appel) et l'affichage famille (socle 10).
- **Continuité** : évaluations, cahier de textes, progression, séances appartiennent au **service** (les socles 20, 21 le garantissent par leur clé `service_id`, à confirmer dans leurs conceptions) ; l'auteur réel de chaque saisie reste l'agent qui l'a faite.
- **Unicité** remplacée : un même agent ne peut avoir deux interventions actives chevauchantes sur le même service (contrainte d'exclusion) ; l'ancien index `ent_service_unique_idx` (`092:103-109`), qui contient `prof_user_id`, est remplacé par `UNIQUE (etablissement_id, annee_id, coalesce(classe_id, groupe_id), matiere_id, modalite, code)` quand il n'y a pas de code, à trancher au lot P3 sur les données de labo.
- **Contrôles de cohérence** (alertes, jamais blocage) : agent intervenant hors de sa période d'engagement ; matière hors de ses disciplines de poste ; heures attribuées > heures dues + seuil ; heures du service ≠ heures prévues pour la matière au niveau (lecture socle 5) ; deux services de même matière sur une même classe et un groupe qui la recouvre (doublon probable).

### 8.6 Missions

```
ent_mission_type
  id, etablissement_id NULL          -- NULL = fourni par ScolIA
  code TEXT, libelle TEXT, description TEXT NULL
  perimetres_admis TEXT[]            -- etablissement | site | niveau | classe | groupe | eleves | agents | discipline
  cardinalite_max SMALLINT NULL      -- par périmètre (PP : 1 par défaut, 2 permis)
  annuelle BOOLEAN                   -- se termine à la clôture de l'année (PP) ou non (référent numérique)
  modele_role_id UUID NULL           -- rôle appliqué au périmètre pendant la mission (§9.3)
  visible_familles BOOLEAN           -- PP, référent harcèlement : oui ; tuteur : non
  visible_eleves BOOLEAN
  regle_eligibilite TEXT NULL CHECK (… IN ('enseigne_dans_perimetre','fonction','aucune'))
  effet_regle TEXT CHECK (… IN ('avertir','bloquer'))
  proprietaire_socle SMALLINT NULL   -- 35 pour « accompagnement AESH », 39 pour « tuteur de stage » : seul ce socle l'attribue

ent_mission
  id, etablissement_id, type_id, agent_id, annee_id NULL
  du DATE NOT NULL, au DATE NULL
  origine TEXT (saisie | delegation | import | reconduction | socle_tiers), origine_ref UUID NULL
  lettre_document_id UUID NULL       -- socle 7, facultative
  annule_le TIMESTAMPTZ NULL

ent_mission_perimetre
  mission_id, nature TEXT (comme perimetres_admis), cible_id UUID NULL, discipline_code TEXT NULL
```

Types fournis [R] (libellés modifiables, désactivables) : **professeur principal** (classe ; annuelle ; cardinalité 1, 2 autorisés ; modèle « Professeur principal ») ; **professeur référent** d'un groupe ou d'un dispositif ; **CPE référent** (niveaux ou classes ; modèle « CPE référent ») ; **coordinateur de discipline** (discipline) ; **coordinateur de niveau, de cycle** ; **référent décrochage**, **référent harcèlement** [NV : programme de lutte contre le harcèlement], **référent numérique**, **référent culture**, **référent laïcité et citoyenneté**, **référent égalité**, **référent handicap / inclusion**, **référent orientation**, **référent vie lycéenne** [NV] ; **tuteur** (agents : stagiaire, nouvel arrivant) ; **responsable de laboratoire, de salle spécialisée** ; **responsable de l'association sportive** [NV] ; **chef de travaux / DDFPT** (périmètre : formations) [NV] ; **accompagnement AESH** (élèves ; propriétaire : socle 35) ; **tuteur de stage** (élèves ; propriétaire : socle 39) ; **surveillance d'internat** (site ou bâtiment). L'établissement crée les siens (« référent séjour à Londres », « responsable du foyer »).

### 8.7 Responsabilités de groupes et d'ateliers

```
ent_groupe_responsable (existante, enrichie)
  + id UUID PK (la clé (groupe_id, user_id) devient une contrainte d'exclusion datée)
  + agent_id UUID NOT NULL           -- reprise depuis user_id ; user_id conservé en projection
  + nature TEXT CHECK (nature IN ('responsable','animateur','co_animateur'))
  + du DATE NOT NULL, au DATE NULL
```

`definir_responsables_groupe_for_actor` (remplacement en bloc [F `droits_repo.rs:540-589`]) devient deux commandes datées (ajouter, terminer). La portée `mes_groupes` lit les responsabilités actives à la date.

### 8.8 Équipes

**Équipes calculées** (aucune table de membres ; fonctions SQL et Rust, testées) :

| Équipe | Règle [R] |
|---|---|
| Équipe pédagogique d'une **classe** à la date D | intervenants en saisie ou titulaires des services dont la cible est la classe **ou un groupe qui contient au moins un élève de la classe** à D ; + missions actives sur la classe (PP, CPE référent, professeur référent) ; + direction de niveau si une mission existe |
| Équipe pédagogique d'un **groupe** | intervenants des services du groupe + responsables du groupe |
| Équipe d'un **niveau** | union des équipes des classes du niveau + coordinateurs de niveau |
| Équipe d'une **discipline** | agents ayant la discipline de poste, ou un service de la matière, actifs à D ; + coordinateur de discipline |
| Équipe d'un **élève** | intervenants des services de l'élève + missions dont le périmètre contient l'élève ou sa classe (PP, CPE, AESH, tuteur) ; c'est « l'équipe de mon enfant » |
| Vie scolaire, direction, administration, agents | agents dont la fonction est de la famille correspondante |

**Équipes constituées** :

```
ent_equipe_type (catalogue + types de l'établissement)
  code, libelle : conseil_pedagogique, cellule_veille, equipe_educative_eleve, equipe_projet,
                  equipe_direction, equipe_disciplinaire_elargie, portes_ouvertes, autre
ent_equipe
  id, etablissement_id, type_id, libelle, annee_id NULL, du, au NULL
  objet_eleve_id UUID NULL           -- équipe éducative ou de suivi autour d'un élève (cas P46)
  liste_diffusion BOOLEAN            -- utilisable comme destinataire (socle 16)
  visible_annuaire BOOLEAN
  confidentielle BOOLEAN             -- cellule de veille, équipe autour d'un élève : liste visible des seuls membres et de la direction
ent_equipe_membre
  equipe_id, agent_id NULL, personne_id NULL   -- une famille peut être membre d'une équipe éducative (socle 19 invite)
  role_dans_equipe TEXT (coordinateur | membre | invite), du, au NULL
```

**Une équipe ne donne aucun droit sur les élèves** (Q-P9) : seuls les services, missions, responsabilités de groupe et rôles en donnent. Une cellule de veille qui doit lire les suivis de certains élèves passe par une mission ou un rôle à périmètre « élèves désignés ».

### 8.9 Attributions, délégations, pouvoirs de signature

```
ent_role_attribution (existante, socle 1, enrichie par les lots P0 : dates et historique, et P4 : périmètre)
  + id UUID PK                       -- la clé (role_id, user_id) devient une contrainte d'exclusion datée
  debut, fin                          -- début futur permis
  + perimetre JSONB NULL             -- {classes:[…], groupes:[…], niveaux:[…], sites:[…], eleves:[…]} pour la portée « mon_perimetre » (§9.3)
  + source TEXT CHECK (source IN ('manuelle','mission','delegation','fonction','import'))
  + source_ref UUID NULL             -- mission_id, delegation_id
  + clos_le TIMESTAMPTZ NULL, clos_par_user_id UUID NULL   -- plus d'écrasement (ON CONFLICT supprimé)

ent_delegation
  id, etablissement_id
  delegant_agent_id, delegataire_agent_id
  objet TEXT CHECK (objet IN ('mission','role','signature','interim_direction'))
  objet_ref UUID NULL                -- mission_id, role_id, habilitation
  du DATE NOT NULL, au DATE NOT NULL -- fin obligatoire (« les délégations expirent seules »)
  motif_code TEXT                    -- absence | surcharge | organisation | autre
  revoquee_le TIMESTAMPTZ NULL, revoquee_par UUID NULL

ent_habilitation_signature
  id, etablissement_id, agent_id
  documents TEXT[]                   -- certificat_scolarite, convention_stage, ordre_mission, bulletin, attestation, courrier_famille…
  du, au NULL, origine (fonction | delegation), origine_ref
```

Règles [R] :
- **Délégation = attributions dérivées.** Accorder une délégation crée, dans la même transaction, les attributions (source `delegation`) ou la mission (origine `delegation`) correspondantes, avec les mêmes dates ; révoquer la délégation les clôt. L'**anti-escalade** s'applique au délégant : on ne délègue que ce qu'on détient, sur une portée au plus égale [F `droits_repo.rs:633-659`].
- **Intérim de direction** : délégation `interim_direction` = attributions des rôles du chef d'établissement (sauf `roles.gerer` et `securite.politique` si le paramétrage le décide) + pouvoirs de signature, pour la période.
- **Pouvoirs de signature** : le socle 7 demande `peut_signer(agent, nature_document, date)` avant d'apposer une signature au nom de l'établissement ; le chef d'établissement a tous les pouvoirs par sa fonction ; les autres par habilitation [NV : délégation de signature du chef d'établissement].

### 8.10 Parcours d'arrivée et de départ

```
ent_parcours_modele                  -- paramétrage : étapes par nature d'engagement et fonction
  id, etablissement_id, sens (arrivee | depart), natures TEXT[], fonctions UUID[]
  etapes JSONB                       -- [{code, libelle, responsable_fonction, automatique_sur_evenement, obligatoire}]
ent_parcours_agent
  id, etablissement_id, agent_id, engagement_id, sens, ouvert_le, clos_le NULL
ent_parcours_etape
  parcours_id, code, etat (a_faire | fait | sans_objet | bloque), fait_le, fait_par, preuve_ref NULL
```

Étapes fournies [R] — **arrivée** : fiche complète ; engagement ; fonctions ; services repris ; missions ; rôles confirmés ; invitation envoyée ; **identité rattachée** (`personne.compte_rattache`) ; **passkey enrôlée** (événement du socle 1) ; charte informatique et règlement signés (socle 7) ; photo (avec consentement) ; clés et badge remis (socle 44) ; livret d'accueil lu ; tuteur désigné ; présentation à l'équipe ; première connexion à l'application mobile. **Départ** : date et motif ; services transférés ou déclarés vacants ; missions transférées ou vacantes ; délégations accordées et reçues révoquées ; **tâches en cours** (appels non faits, évaluations non publiées, appréciations manquantes, documents à signer, conversations non lues, séances futures attribuées : lus aux socles 10, 12, 16, 20, 21, 23 par leurs ports) ; matériel rendu (socle 44) ; accès fermé ; information des familles concernées (texte proposé, envoi humain) ; écriture d'adieu facultative.

### 8.11 Identifiants externes

```
ent_agent_identifiant
  id, etablissement_id, agent_id
  systeme TEXT CHECK (systeme IN ('sts','aaf_jointure','guichet_agents','gar','edt_logiciel','ancien_logiciel','annuaire_entreprise'))
  valeur TEXT, du, au NULL
  UNIQUE (etablissement_id, systeme, valeur) WHERE au IS NULL
```

Le NUMEN n'y figure pas (Q-P15). Les identifiants de connexion fédérée (`sub` du guichet Agents) restent au socle 1 (`ent_oidc_identity` [F `090`]) ; la **jointure** issue de l'AAF est ici, parce qu'elle rattache une personne importée à une connexion (`INTEROPERABILITE_NATIONALE.md` §4 [S]).

### 8.12 Revue des accès

```
ent_revue_acces
  id, etablissement_id, ouverte_le, close_le NULL, par_user_id, perimetre (tout | site)
ent_revue_acces_ligne
  revue_id, user_id, agent_id NULL, casquette, constat TEXT[]  -- sans_engagement, engagement_termine, sans_passkey,
                                                               -- profil_admin, role_large, inactif_90_jours, delegation_ouverte
  decision TEXT (conserve | ferme | corrige), decide_le, decide_par
```

### 8.13 Paramétrage du domaine

### 8.14 Compléments de modèle [personas]

Pseudo-DDL, mêmes règles que §8. **Numéros de migration à attribuer** dans `MIGRATIONS.md` (la plage 350–369 est entièrement répartie, §17). Si Q-P28 retient une table unique « cours » (socle 5), les ajouts sur `ent_service_enseignement` s'appliquent à cette table.

```
ent_agent_engagement (§8.2)
  + installe_le DATE NULL, + pv_installation_document_id UUID NULL      -- PRS-116 : prise de fonctions réelle [NV]
  + motif_fin : codes ajoutés 'fin_periode_essai','rupture_conventionnelle','licenciement','abandon_de_poste',
                'detachement','radiation'                                -- R-P13 [NV]
  -- exclusion remplacée : par (agent, employeur_type, employeur_organisme_id, nature)  (R-P1)
ent_agent_engagement_suspension                                           -- R-P4 : disponibilité, détachement, congé parental
  id, engagement_id, du DATE, au DATE NULL, motif_code, acces_suspendu BOOLEAN DEFAULT true, annule_le

ent_agent_dossier_arrivee                                                 -- PRS-117, PRS-118 (trou T-07)
  id, etablissement_id, agent_id, engagement_id, ouvert_le, soumis_le NULL, valide_le NULL, valide_par
ent_agent_dossier_piece  dossier_id, code_piece, document_id NULL (socle 7), etat ('attendue','deposee','validee','refusee','sans_objet')
  -- pièces demandées selon la nature d'engagement ; les pièces d'employeur de droit privé (paie) relèvent du socle 49

ent_service_note_passation                                                -- PRS-122, R-P7
  id, etablissement_id, service_id, auteur_agent_id, texte (≤ 4 000 car.), progression_ref NULL, ecrite_le, remplace_id NULL

ent_intervention_ponctuelle                                               -- PRS-124 (T-15, part socle 6)
  id, etablissement_id, agent_id NULL, personne_id NULL, objet_code, seances UUID[] (socle 10) | (cours_id, du, au),
  referent_agent_id, annule_le
ent_acces_temporaire                                                      -- PRS-125, R-P8 : inspection, formateur, tuteur extérieur
  id, etablissement_id, personne_id, nature ('inspection','formation','tutelle_externe','controle'),
  agent_concerne_id NULL, classe_ou_cours_ids UUID[], du TIMESTAMPTZ, au TIMESTAMPTZ (≤ 1 jour, prolongeable à 5),
  lectures TEXT[] ('cahier','progression','edt','liste_classe'), demande_par, informe_agent_le, revoque_le

ent_agent_voeu_service                                                    -- PRS-127
  id, etablissement_id, agent_id, annee_id (N+1), voeux JSONB (niveaux, cours, options), heures_sup_acceptees NUMERIC NULL,
  commentaire (≤ 500), depose_le
ent_repartition_proposition   id, annee_id, discipline, propose_par (coordinateur), contenu JSONB, etat, arbitre_le   -- PRS-128
ent_annee_repartition_publication  annee_id, publiee_le, par                -- PRS-129, R-P17

ent_agent_service_du                                                      -- PRS-130 [NV : composantes exactes]
  id, etablissement_id, agent_id, annee_id, obligation_hebdo NUMERIC, du DATE, au DATE NULL
ent_agent_service_du_composante  service_du_id, nature ('decharge','ponderation','reduction','complement_ailleurs','autre'),
  libelle, heures NUMERIC, motif_code
ent_etat_service                                                          -- PRS-131 : état de service signé [NV]
  id, agent_id, annee_id, version, contenu JSONB (figé), document_id, signe_le NULL, reserves TEXT NULL (≤ 1 000)

ent_mission + remuneration_nature TEXT NULL CHECK (… IN ('aucune','imp','part_modulable_pp','pacte','hse','ogec','autre'))
            + volume NUMERIC NULL, + unite TEXT NULL ('heures','taux','forfait')  -- PRS-132, R-P11 : jamais de montant

ent_poste_service                                                         -- PRS-133 (trou T-09) : grille, cantine, étude, nuit…
  id, etablissement_id, site_id NULL, code, libelle, heure_debut, heure_fin, jours SMALLINT[], semaines TEXT NULL,
  effectif_min SMALLINT, nature ('surveillance','accueil','restauration','entretien','internat_nuit','astreinte','autre'),
  responsable_fonction_id NULL, ferme_le
ent_planning_regle     id, agent_id, poste_id, jours, semaines, du, au NULL                -- cycles A/B (semaines du socle 5)
ent_planning_exception id, agent_id, poste_id NULL, date, nature ('ajout','retrait','echange'), motif_code, valide_par
ent_planning_echange   id, demandeur_agent_id, receveur_agent_id, exceptions UUID[], etat, valide_par, valide_le   -- PRS-134
  -- compteur : heures planifiées vs heures dues de l'engagement (annualisées) ; aucune donnée de paie

ent_agent_lien_eleve                                                      -- PRS-137, R-P6
  id, etablissement_id, agent_id, eleve_id, nature ('famille_hors_lien_legal','proche','voisin','autre'),
  declare_par ('agent','direction'), du, au NULL
  -- lisible : l'agent, la direction ; jamais les familles ni les collègues

ent_mission_type + code fourni 'encadrement' (périmètre : agents)          -- PRS-139 : responsable fonctionnel
ent_agent_verification                                                    -- PRS-143 [NV]
  id, agent_id, nature ('honorabilite'), verifiee_le DATE, par, resultat ('conforme','non_conforme'), echeance DATE NULL
  -- jamais le contenu d'un extrait ni d'un fichier consulté
ent_cle_securite_pret                                                     -- PRS-119, PRS-120 (ou socle 44)
  id, etablissement_id, numero_inventaire, agent_id NULL, pretee_le, rendue_le, locale_seulement BOOLEAN, expire_le NULL

ent_pouls_campagne / ent_pouls_reponse                                    -- PRS-144, R-P12 : DÉSACTIVÉ par défaut
  reponse : campagne_id, date, valeur SMALLINT (1..5), fonction_famille TEXT NULL — AUCUN agent_id, aucun texte libre
ent_agent_qualification.code + 'sst','ssiap','caces','haccp','bafa','bnssa'  -- R-P14 [NV]
```

`ent_param_personnels` (une ligne par établissement) : délai de grâce en lecture après fin d'intervention ou d'engagement (Q-P6) ; lecture du titulaire pendant une suppléance (Q-P5) ; cardinalité et règle d'éligibilité du PP ; nombre de jours de vacance avant alerte ; visibilité de l'annuaire pour les familles et les élèves (fonctions affichées, photo) ; casquettes autorisées ; profils de base par fonction ; durée de validité des invitations ; parcours (modèles) ; fréquence proposée de la revue des accès ; indicateurs d'activité activés ou non (Q-P21) ; accès des intervenants (sans compte, compte invité).

---

## 9. Comment ce socle alimente les droits (D-017, D-022)

### 9.1 Ce qui ouvre un droit sur un élève

Le moteur `autoriser(acteur, action, cible, date)` du socle 1 reste **le seul juge** [F `droits_repo.rs:120-210`]. Le socle 6 lui fournit les **faits** ; une portée se résout ainsi :

| Portée | Résolution cible [R] | Aujourd'hui [F] |
|---|---|---|
| `etablissement` | tout l'établissement | idem |
| `mes_classes` (libellé : « mes élèves par l'enseignement ») | élèves affectés (socle 2) à D à la cible (classe **ou groupe**) d'un service dont l'acteur a une intervention active à D **qui ouvre cette action** (saisie pour `notes.saisir`, lecture pour `notes.lire`) ; + délai de grâce ; + séances du jour où il est remplaçant **de séance** (socle 15), pour la séance seulement | `enseigne_a` + repli classe texte + remplacement ±14 jours (`perimetre.rs`) |
| `mes_groupes` | élèves des groupes dont l'acteur a une responsabilité active à D | responsables non datés |
| `designes` | cibles fixées **dans le rôle** (inchangé) | idem |
| **`mon_perimetre`** (nouvelle) | cibles portées **par l'attribution** : périmètre de la mission (PP → la classe ; CPE référent → ses niveaux ; AESH → ses élèves) ou de l'attribution manuelle (« CPE » attribuée avec les niveaux 6ᵉ et 5ᵉ) | n'existe pas |
| **`moi`** (documentée, à implémenter) | l'acteur lui-même (sa fiche, ses services, ses droits) | absente |
| **`sites_designes`** (proposée par les socles 2 et 4) | élèves, personnes ou agents des sites désignés | absente |

Pourquoi une nouvelle portée plutôt que `designes` : avec `designes`, il faudrait **un rôle par classe** (« PP de la 4B », « PP de la 4C »…) puisque les cibles sont dans le rôle [F `098:22-30`]. Avec `mon_perimetre`, un seul modèle « Professeur principal » sert à toutes les classes, la cible est dans l'attribution, et l'attribution naît et meurt avec la mission.

### 9.2 La règle D-022, rendue exacte

« Un professeur ne voit que les élèves des groupes et services qui lui sont attribués » devient : **portée `mes_classes` = intervention active sur un service dont la cible contient l'élève à la date**. Conséquences :
- si l'établissement a modélisé l'allemand LV2 de la 4B comme un **groupe**, le professeur d'allemand ne voit que le groupe ; s'il a laissé un service « allemand » sur la **classe**, il voit la classe : la règle ne devine rien, elle applique le paramétrage (D-022, principe de Florian) ;
- un **contrôle de qualité** signale les services sur une classe dont la matière est enseignée par plusieurs professeurs dans la même classe (signe qu'il manque des groupes) ;
- le **repli classe texte** (`perimetre.rs:41-79, 128-139`) est coupé **établissement par établissement** dès que tous les créneaux de l'emploi du temps sont rattachés à un service (indicateur « créneaux sans service = 0 », lot P3) ; d'ici là il est conservé et signalé (bandeau « périmètre provisoire » dans l'écran des droits).

### 9.3 Missions, fonctions et rôles

- **Mission → attribution** : un type de mission qui porte un `modele_role_id` crée, à l'attribution de la mission, une attribution de ce rôle à l'agent avec `perimetre` = périmètre de la mission, `source = mission`, mêmes dates. Fin de la mission = fin de l'attribution. Modèles fournis [R] :
  - **Professeur principal** : `dossier.lire`, `absences.lire`, `notes.lire` (toutes matières), `suivi.eleve`, `messagerie.eleve`, `conseils.gerer`, `journal.lire` en portée `mon_perimetre` ;
  - **CPE référent** : actions de vie scolaire en `mon_perimetre` (l'établissement peut garder « CPE » en portée établissement, comme aujourd'hui) ;
  - **Accompagnement AESH** : `dossier.lire` réduit, emploi du temps et cahier de textes de l'élève, aménagements (socle 35), en `mon_perimetre` ; ni notes ni vie scolaire par défaut ;
  - **Tuteur** : lecture de l'emploi du temps et du cahier des agents du périmètre (action `personnels.services.lire`, §14), pas leurs élèves.
- **Fonction → proposition** : à l'arrivée, la fonction **propose** des modèles de rôles (enseignant → « Professeur » ; AED → « Assistant d'éducation » ; agent d'accueil → « Accueil ») ; la direction confirme. Rien n'est accordé sans confirmation, et l'anti-escalade s'applique à qui confirme.
- **Profil de base** (`ent_membership.role`) : reste le **modèle par défaut** (D-017) tant que des rôles explicites ne le remplacent pas ; il est déduit de la fonction principale pour les nouveaux agents et modifiable.

### 9.4 Dates, délais, cas particuliers

- **Avant le début** d'une intervention : aucune donnée d'élève (seulement la préparation, §8.4).
- **Après la fin** d'une intervention : délai de grâce paramétré (défaut [R] 7 jours, 0 à 15), **en lecture et en saisie de ses propres évaluations de la période seulement** (finir de saisir une évaluation faite avant le départ), jamais de nouvelles évaluations ni de lecture d'autres données.
- **Années passées** : un professeur garde la lecture **de ses propres saisies** des années précédentes (notes, appréciations) pour répondre à une famille ou à un recours, pendant la durée de conservation ; pas le reste du dossier [R] (Q-P7).
- **Remplacement d'une seule séance** (socle 15) : liste d'appel et cahier de **la séance**, le jour même ; fin de la fenêtre de ±14 jours [F `perimetre.rs:26`].
- **Professeur-parent** : en casquette personnel, l'accès au dossier de son propre enfant est **tracé à part** (domaine `consultation`, comme la santé [F `ROLES_ET_DROITS.md` R2]) et visible de la direction ; s'il a un service qui contient son enfant, alerte à la direction à la création (Q-P3).
- **Accès suspendu** (cas P9) : aucune casquette personnel ouverte ; les autres casquettes (parent) restent sauf décision.

### 9.5 Explication et simulation

- **« Pourquoi peut-il voir cet élève ? »** : `explique_acces` renvoie la chaîne (rôle « Professeur » → portée « mes élèves par l'enseignement » → intervention « suppléant » sur le service « SVT 4B » du 12 janvier au 3 avril → élève affecté à la 4B depuis le 2 septembre). Affichée à la direction sur la fiche agent et la fiche élève, et à l'agent lui-même pour ses propres droits.
- **Simulation avant attribution** : avant de confirmer une mission, une attribution ou une délégation, l'écran affiche « Mme X verra 28 élèves de plus (4B), pourra saisir les appréciations de PP, jusqu'au 4 juillet ».
- **Indicateur de largeur** : nombre d'élèves visibles par agent, comparé à ses services (repère pour la revue des accès, jamais affiché aux collègues).

---
## 10. Actions métier

★ = confirmation avec récapitulatif. Acteurs : DIR direction, SEC secrétariat, CPE, PP, AGT l'agent lui-même, FAM famille, ELV élève, SYS tâche automatique. Les acteurs sont indicatifs : le droit réel est l'action du catalogue (§14), paramétrable (D-017).

### 10.1 Fiche, engagements, fonctions

| ID | Action | Acteur | Règles clés | Événement | Ancien |
|---|---|---|---|---|---|
| PRS-01 | Créer un agent | DIR, SEC | recherche de la personne d'abord (`personnes::correspondances`, socle 4) : « est-ce M. Martin, père de Léa ? » ; jamais d'autre établissement visible | `personnel.cree` | RH-01, ADM-02 |
| PRS-02 | Modifier l'identité d'un agent | DIR, SEC | par le port du socle 4 ; correction ≠ changement | `personne.identite_modifiee` (socle 4) | — |
| PRS-03 | Ajouter un engagement (arrivée, renouvellement) | DIR, SEC | dates, nature, employeur, quotité ; pas de chevauchement ; référent obligatoire pour les natures limitées | `personnel.engagement_ajoute` | RH-01 |
| PRS-04 | Prolonger, modifier, corriger un engagement | DIR, SEC | correction annule la ligne erronée ; prolongation conserve l'historique | `personnel.engagement_modifie` | — |
| PRS-05 | Définir fonctions et site | DIR, SEC | une principale ; propose des modèles de rôles | `personnel.fonctions_modifiees` | RH-01 |
| PRS-06 | Définir disciplines de poste | DIR, SEC ; import | code national si connu | `personnel.disciplines_modifiees` | RH-01 |
| PRS-07 | Enregistrer une qualification (secourisme, encadrement, permis) | DIR, SEC ; AGT propose | date de validité ; alerte avant expiration | `personnel.qualification_modifiee` | — |
| PRS-08 | Déclarer son contact d'urgence | AGT | facultatif ; visible de la direction seule | — | — |
| PRS-09 | Déposer ou retirer sa photo, choisir qui la voit | AGT ; SEC avec accord | consentement daté ; collègues, élèves, familles séparément | `personnel.photo_modifiee` | — |
| PRS-10 | Mettre ses notifications en pause | AGT | jusqu'à une date ; avis de sécurité non suspendus | — | — |
| PRS-11 | Poser l'identité restreinte d'un agent menacé | DIR ★ | motif réservé ; effet sur l'affichage familles et élèves | `personnel.identite_restreinte` | — |
| PRS-12 | Signaler un personnel mineur | SEC | restrictions par défaut | — | — |
| PRS-13 | Fusionner deux agents en double (même personne créée deux fois) | DIR ★ | via fusion de personnes du socle 4, réversible ; jamais deux comptes différents (D-013) | `personnel.fusionne` | — |

### 10.2 Arrivée, invitation, accès

| ID | Action | Acteur | Règles clés | Événement | Ancien |
|---|---|---|---|---|---|
| PRS-20 | Inviter un agent (e-mail) | DIR, SEC | lien à usage unique, 7 jours ; plus de mot de passe temporaire | `personnel.invite` | ADM-02 |
| PRS-21 | Inviter par QR code (au guichet ou en séance collective) | SEC | QR valable 10 minutes, affiché sur un écran authentifié | `personnel.invite` | — |
| PRS-22 | Inviter en lot | SEC ★ | aperçu des destinataires, adresses manquantes listées | `personnel.invitations_lot` | — |
| PRS-23 | Relancer, annuler une invitation | SEC | | — | — |
| PRS-24 | Accepter l'invitation, rattacher son identité | AGT | si identité existante : connexion par passkey et confirmation ; sinon création et enrôlement **avant** accès | `personne.compte_rattache` (socle 4), `personnel.acces_rattache` | — |
| PRS-25 | Préparer son arrivée (accès réduit avant la date de début) | AGT | emploi du temps, cahier et progression des services, collègues ; aucune donnée d'élève | — | — |
| PRS-26 | Ouvrir l'accès à la date de début | SYS | tâche quotidienne idempotente | `personnel.arrive` | — |
| PRS-27 | Ouvrir le parcours d'arrivée et suivre ses étapes | SYS ; SEC, DIR | modèle selon nature et fonction ; étapes cochées par les faits | `personnel.parcours_ouvert` / `_clos` | — |
| PRS-28 | Arrivée guidée (assistant complet : fiche → engagement → fonctions → services → missions → rôles → invitation) | DIR, SEC ★ | une seule transaction ; viser 3 minutes | `personnel.cree`, … | — |
| PRS-29 | Basculer de casquette (professeur ↔ parent) | AGT | sans réauthentification dans la fenêtre d'assurance ; journalisée | `session.casquette_changee` (socle 1) | — |

### 10.3 Départ, suspension, retour

| ID | Action | Acteur | Règles clés | Événement | Ancien |
|---|---|---|---|---|---|
| PRS-30 | Planifier un départ (date, motif codé) | DIR, SEC ★ | ouvre le parcours de départ ; alerte J-15 à l'agent et à la direction sur ses tâches en cours | `personnel.depart_planifie` | ADM-05 |
| PRS-31 | Départ guidé (services, missions, délégations, tâches, matériel, information des familles) | DIR ★ | récapitulatif avant / après ; services sans successeur déclarés vacants | `service.intervenant_termine`, `mission.terminee`, … | ADM-05 |
| PRS-32 | Fermer l'accès à la date de fin | SYS | sessions de **cet** établissement révoquées, jetons push supprimés, identité conservée ; garde-fou du dernier administrateur | `personnel.parti`, `acces.ferme` | ADM-05 |
| PRS-33 | Suspendre l'accès immédiatement (mesure conservatoire, compromission) | DIR ★ | motif codé réservé ; effet en moins d'une minute ; casquette parent conservée sauf décision | `acces.suspendu` | — |
| PRS-34 | Rétablir l'accès | DIR ★ | | `acces.retabli` | — |
| PRS-35 | Enregistrer un décès | DIR ★ | aucune communication automatique ; relances et notifications programmées annulées | `personnel.parti` (motif `deces`) | — |
| PRS-36 | Réengager un ancien agent | DIR, SEC | même agent, nouvel engagement, nouvelle invitation si l'accès est fermé | `personnel.engagement_ajoute` | — |
| PRS-37 | Suspendre soi-même son accès pendant un congé | AGT | réversible par l'agent | `acces.suspendu` (motif `demande_agent`) | — |

### 10.4 Services d'enseignement et répartition

| ID | Action | Acteur | Règles clés | Événement | Ancien |
|---|---|---|---|---|---|
| PRS-40 | Créer un service (matière, cible, heures, modalité, période) | DIR, SEC | cible du socle 5 ; peut être créé sans intervenant (vacant) | `service.cree` | REF-12 |
| PRS-41 | Modifier les heures, la période, la modalité | DIR, SEC | | `service.modifie` | — |
| PRS-42 | Changer la cible ou la matière en cours d'année | DIR ★ | nouveau service qui remplace l'ancien à la date ; évaluations et cahier : rattachement proposé, jamais automatique | `service.remplace` | — |
| PRS-43 | Clore, annuler un service | DIR, SEC ★ | annuler seulement sans aucune saisie | `service.clos` / `_annule` | REF-12 |
| PRS-44 | Attribuer le titulaire | DIR, SEC | alertes de cohérence (§8.5) | `service.intervenant_ajoute` | REF-12 |
| PRS-45 | Ajouter un co-enseignant, un assistant, un intervenant | DIR, SEC | heures réparties | `service.intervenant_ajoute` | — |
| PRS-46 | Nommer un suppléant (depuis une absence du socle 15 ou une vacance) | DIR ★ ; socle 15 | une saisie par jour ; recouvrement facultatif ; familles informées du nom (socle 8) | `service.suppleance_ouverte` / `_close` | RH-04 |
| PRS-47 | Corriger une intervention erronée | DIR ★ | annulation ; saisies conservées avec leur auteur réel ; pas d'information automatique | `service.intervention_corrigee` | — |
| PRS-48 | Remplacer un professeur sur tous ses services d'un coup (successeur) | DIR ★ | depuis la fiche de l'agent ; aperçu par service | `service.intervenant_ajoute` (lot) | — |
| PRS-49 | Voir la répartition (par classe, par professeur, par matière) avec couverture | DIR, SEC, AGT (les siens) | heures prévues (socle 5) vs attribuées ; heures dues vs attribuées ; vacants ; doublons | — | — |
| PRS-50 | **Préparer la répartition de N+1** | DIR | services `prevu` sur l'année en préparation (socle 5) ; reconduction depuis N en un geste (même professeur, même matière, classe « montée ») ; comparaison N / N+1 ; aucun effet sur les droits | `service.prevu_cree` (lot) | — |
| PRS-51 | Simuler un scénario de répartition | DIR | plusieurs brouillons, un seul retenu | — | — |
| PRS-52 | Ouvrir la répartition de N+1 à la rentrée | DIR ★ | services `prevu` → `ouvert` à l'ouverture de l'année (socle 5) | `service.ouvert` (lot) | — |
| PRS-53 | Exporter vers le logiciel d'emploi du temps / recevoir ses services | SEC | formats [NV] ; rapprochement sans écrasement (socle 9) | `export.realise`, `import.lot_applique` | IOP-02 |
| PRS-54 | Voir ses services et ses heures | AGT | « mes services » ; écarts signalés à l'agent | — | — |
| PRS-55 | Déclarer ses indisponibilités (complément de service, temps partiel) | AGT ; SEC | transmises au socle 10 ; sans dire où (cas P2) | `personnel.indisponibilites` | — |

### 10.5 Missions et responsabilités

| ID | Action | Acteur | Règles clés | Événement | Ancien |
|---|---|---|---|---|---|
| PRS-60 | Nommer un professeur principal | DIR | cardinalité, éligibilité (avertir / bloquer) ; droits du modèle sur la classe | `mission.attribuee` (type PP) | REF-13 |
| PRS-61 | Nommer les PP de toutes les classes en un écran | DIR ★ | tableau classes × candidats (professeurs de la classe en tête) ; classes sans PP signalées | `mission.attribuee` (lot) | REF-13 |
| PRS-62 | Nommer un référent, un coordinateur, un tuteur | DIR | périmètre selon le type | `mission.attribuee` | — |
| PRS-63 | Terminer, transférer une mission | DIR | transfert = fin + attribution, même date | `mission.terminee`, `mission.attribuee` | — |
| PRS-64 | Créer un type de mission propre à l'établissement | Admin | périmètres admis, cardinalité, modèle de rôle facultatif (anti-escalade sur le modèle) | `mission.type_cree` | — |
| PRS-65 | Reconduire les missions annuelles sur N+1 | DIR ★ | proposition, pas automatisme | `mission.attribuee` (lot) | — |
| PRS-66 | Désigner les responsables et animateurs d'un groupe ou atelier | DIR, SEC, CPE | datés | `groupe.responsable_ajoute` / `_termine` | D-017 |
| PRS-67 | Recevoir une mission d'un autre socle (AESH du socle 35, tuteur de stage du socle 39) | socle tiers | le socle tiers est seul à pouvoir attribuer ses types | `mission.attribuee` | — |

### 10.6 Équipes

| ID | Action | Acteur | Règles clés | Événement | Ancien |
|---|---|---|---|---|---|
| PRS-70 | Voir l'équipe pédagogique d'une classe, d'un groupe, d'un niveau, d'une discipline, d'un élève | personnels selon droits ; FAM, ELV (équipe de l'élève) | calculée à la date | — | — |
| PRS-71 | Créer une équipe constituée | DIR, CPE | type, membres datés, liste de diffusion, confidentialité | `equipe.creee` | — |
| PRS-72 | Modifier les membres | coordinateur, DIR | datés | `equipe.membres_modifies` | — |
| PRS-73 | Constituer l'équipe éducative autour d'un élève | DIR, CPE, PP | confidentielle ; peut inviter la famille (socle 19) ; aucun droit par elle-même | `equipe.creee` (objet élève) | ACC-04 |
| PRS-74 | Écrire à une équipe | membres | via socle 16 ; liste calculée au moment de l'envoi | (socle 16) | — |
| PRS-75 | Clore une équipe | coordinateur, DIR | | `equipe.close` | — |

### 10.7 Délégations, intérim, signature

| ID | Action | Acteur | Règles clés | Événement | Ancien |
|---|---|---|---|---|---|
| PRS-80 | Déléguer une mission ou un rôle, datée | DIR ; agent pour ses missions si paramétré | fin obligatoire ; anti-escalade | `delegation.accordee` | ADM-04 |
| PRS-81 | Organiser l'intérim de direction | Chef d'établissement, Admin ★ | rôles et signature ; exclusions paramétrées | `delegation.accordee` (objet `interim_direction`) | ADM-04 |
| PRS-82 | Habiliter à signer certains documents | Chef d'établissement ★ | liste des natures de documents ; dates | `signature.habilitation_modifiee` | — |
| PRS-83 | Révoquer une délégation | délégant, DIR | effet immédiat | `delegation.revoquee` | — |
| PRS-84 | Expiration des délégations | SYS | à la date ; prévenir le délégant 3 jours avant | `delegation.expiree` | ADM-04 |

### 10.8 Annuaire et vues des familles et des élèves

| ID | Action | Acteur | Règles clés | Événement | Ancien |
|---|---|---|---|---|---|
| PRS-90 | Chercher un collègue (annuaire interne) | personnels | coordonnées professionnelles ; photo selon consentement ; présence du jour (« en cours en salle B12 », d'après le socle 10) réservée aux fonctions qui en ont besoin | — | — |
| PRS-91 | Voir la fiche complète d'un agent | DIR, SEC selon droits | groupes de lecture (§13.3) | consultation tracée | — |
| PRS-92 | Voir ce que je peux faire et pourquoi (« mes droits ») | AGT | explication par la chaîne §9.5 | — | — |
| PRS-93 | Voir « l'équipe de mon enfant » | FAM | professeurs par matière, PP, CPE, direction, référents visibles ; bouton écrire ; aucune coordonnée personnelle | — | — |
| PRS-94 | Voir « mes professeurs » | ELV | idem | — | — |
| PRS-95 | Être informé d'un changement de professeur | FAM, ELV | suppléance ouverte, nouveau titulaire, fin de vacance ; jamais le motif ; regroupé (socle 8) | `service.suppleance_ouverte` → socle 8 | RH-05 |
| PRS-96 | Trombinoscope des personnels | personnels ; FAM si activé | consentement de chaque agent | — | — |

### 10.9 Contrôle, qualité, revue des accès

| ID | Action | Acteur | Règles clés | Événement | Ancien |
|---|---|---|---|---|---|
| PRS-100 | Tableau de qualité des personnels | DIR, SEC | tuiles : invitations non acceptées ; agents sans passkey ; engagements sans date de fin (contractuels) ; agents partis avec accès ouvert (doit être 0) ; services vacants ; classes sans PP ; heures dues ≠ attribuées ; créneaux sans service ; qualifications expirées ; intervenants sans référent ; délégations ouvertes depuis plus de 60 jours | — | — |
| PRS-101 | Expliquer l'accès d'un agent à un élève | DIR | §9.5 | consultation tracée | — |
| PRS-102 | Simuler une attribution | DIR | §9.5 | — | — |
| PRS-103 | Ouvrir et conduire une revue des accès | DIR ★ | une ligne par accès personnel ; constats calculés ; décisions ; clôture signée | `acces.revue_close` | — |
| PRS-104 | Détection d'accès massifs inhabituels | SYS | ex. plus de N dossiers ouverts en M minutes par un même compte hors usage habituel ; alerte à la direction, jamais de blocage automatique sauf seuil de sécurité (socle 1) | `securite.alerte_acces` | — |
| PRS-105 | Indicateurs d'activité par agent (appels non faits, évaluations non publiées) | DIR | désactivés par défaut ; annoncés aux personnels ; aucun classement (Q-P21) | — | — |
| PRS-106 | Consulter qui a consulté ma fiche | AGT | journal par fonction (« secrétariat »), pas nominatif (comme Q9 de `SOCLE2_DOMAINES.md`) | — | — |

### 10.10 Imports, exports, paramétrage

| ID | Action | Acteur | Règles clés | Événement | Ancien |
|---|---|---|---|---|---|
| PRS-110 | Importer les personnels et services depuis STS-web | SEC ★ | cadre du socle 9 : prévisualisation, différences, rejets, annulation ; jamais d'écrasement d'une donnée corrigée localement | `import.lot_applique` | IOP-02 |
| PRS-111 | Recevoir l'annuaire des personnels de l'AAF (public) | SYS | sous convention ; crée des agents et des identifiants de jointure ; aucun compte | `import.lot_applique` | IOP-L2 |
| PRS-112 | Importer un fichier CSV de reprise (personnels, fonctions, services) | SEC ★ | modèle de fichier fourni ; socle 28 | `import.lot_applique` | — |
| PRS-113 | Exporter la liste des personnels, des services, la répartition | DIR, SEC | D-022 : exports réservés aux administratifs ; tracés | `export.realise` | — |
| PRS-114 | Transmettre l'annuaire au GAR (enseignants, groupes, rattachements) | SYS | sous convention ; profils déduits des rôles (`INTEROPERABILITE_NATIONALE.md` §6 [S]) | `export.realise` | IOP-07 |
| PRS-115 | Paramétrer le domaine | Admin | §8.13, §11 | `personnels.parametres_modifies` | — |

Total : **86 actions** (dont 11 de l'agent pour lui-même, 4 pour les familles et les élèves, 7 automatiques).

### 10.11 Actions ajoutées par la passe personas [personas]

Source de chaque action : manque `M-xxx` de `PERSONAS_05-06.md`. Même format que ci-dessus.

| ID | Action | Acteur | Règles clés | Événement | Ancien |
|---|---|---|---|---|---|
| PRS-116 | Enregistrer la **prise de fonctions effective** (date réelle, procès-verbal d'installation [NV]) (M-173) | SEC, DIR | distincte de la date prévue ; trame du procès-verbal ; étape du parcours d'arrivée | `personnel.installe` | — |
| PRS-117 | **Compléter son dossier d'arrivée** : coordonnées, contact d'urgence, photo, pièces demandées selon la nature d'engagement (M-175, trou T-07) | AGT (avant ou après la date de début) | lien de l'invitation ; pièces au socle 7 ; pièces de paie de droit privé renvoyées au socle 49 | `personnel.dossier_soumis` | — |
| PRS-118 | **Valider le dossier d'arrivée**, pièce par pièce (M-175) | SEC | refus motivé (code) ; étape « fiche complète » cochée | `personnel.dossier_valide` | — |
| PRS-119 | **Prêter une clé de sécurité** à un agent, l'enrôler au guichet, la récupérer au départ (M-180) | SEC | parc inventorié (ou socle 44) ; étape de départ « matériel rendu » | `personnel.cle_pretee` / `_rendue` | — |
| PRS-120 | Ouvrir un **accès de secours local** à un agent de plusieurs établissements qui a perdu sa passkey ★ (M-181) | DIR | R-P16 ; clé de prêt valable dans cet établissement seulement, ≤ 48 h, pendant la récupération D-018 ; Q-P31 | `acces.secours_ouvert` | — |
| PRS-121 | **Renouveler un engagement** en un geste, sans rupture d'accès (M-179) | SEC, DIR | R-P9 ; services, missions et rôles prolongés avec lui (aperçu) | `personnel.engagement_modifie` | — |
| PRS-122 | Écrire, lire une **note de passation** attachée à un service (progression, évaluations prévues, points d'attention) (M-177) | titulaire, coordinateur, DIR ; lecture : intervenants successifs | R-P7 ; versionnée | `service.note_passation_ecrite` | — |
| PRS-123 | Voir le **dossier de l'arrivant** sur une page : emploi du temps, classes avec trombinoscope, PP et CPE de ses classes, élèves à aménagement (35), plan des salles, contacts, règlement, note de passation (M-176) | AGT | accessible dès la date de début (sans élèves avant, §8.4) ; imprimable (trame) | — | — |
| PRS-124 | Programmer une **intervention ponctuelle** d'un agent ou d'un partenaire sur des séances, sans service annuel (M-171, part socle 6 de T-15) | DIR, CPE, PP | portée = élèves de la séance, le jour de la séance ; référent obligatoire pour un partenaire | `intervention.programmee` | — |
| PRS-125 | Ouvrir un **accès de consultation temporaire** (inspection, formateur, tuteur extérieur) (M-195) | DIR ★ | R-P8 ; lectures choisies dans une liste fermée ; enseignant concerné informé ; aucune donnée de vie scolaire ni de santé | `acces.temporaire_ouvert` / `_clos` | — |
| PRS-126 | Préparer le **dossier d'un contrôle ou d'une inspection** : personnels (fonctions, titres déclarés), structure, effectifs, emplois du temps [NV] (M-196) | DIR | export tracé (D-022) ; trame ; aucune coordonnée personnelle | `export.realise` | — |
| PRS-127 | Recueillir les **vœux de service** des enseignants pour N+1 (niveaux, cours, options, heures supplémentaires acceptées) ; vœux d'emploi du temps transmis au socle 10 (M-140) | DIR ouvre ; AGT dépose | campagne datée ; chacun ne voit que ses vœux ; le coordinateur voit ceux de sa discipline | `service.voeux_deposes` | — |
| PRS-128 | **Proposer la répartition** d'une discipline sur les cours de N+1 ; la direction arbitre (M-158) | coordinateur de discipline (mission) ; DIR | brouillon distinct de la répartition retenue ; aucun effet sur les droits | `service.repartition_proposee` | — |
| PRS-129 | **Publier la répartition prévisionnelle** aux enseignants (leurs cours de N+1, sans élèves) (M-159) | DIR ★ | R-P17 ; mention « prévisionnelle » ; modifications ultérieures signalées à l'intéressé | `service.repartition_publiee` | — |
| PRS-130 | Définir les **composantes du service dû** (obligation, décharges, pondérations, compléments ailleurs) ; calculer l'écart et les heures supplémentaires année [NV] (M-155, M-212) | SEC, DIR | aucune donnée de grade ni de rémunération ; l'agent voit son calcul | `personnel.service_du_modifie` | — |
| PRS-131 | Éditer et faire signer l'**état de service** de chaque enseignant (trame), avec réserves possibles [NV] (M-156) | SEC prépare ; AGT signe ; DIR contresigne | version figée ; signature par le socle 7 ; une réserve n'empêche pas la signature | `personnel.etat_service_signe` | — |
| PRS-132 | Attacher à une mission une **rémunération indicative** (nature et volume, jamais de montant) ; état des missions rémunérées exportable [NV] (M-157) | DIR | R-P11 ; lu par le socle 49 et par l'agent comptable (export) | `mission.remuneration_modifiee` | — |
| PRS-133 | Définir des **postes de service** et le **planning de service** des personnels non enseignants : cycles, exceptions, compteur d'heures annualisées (M-185, M-213, M-228 ; trou T-09) | CPE (AED), gestionnaire (agents), DIR | R-P10 ; jours fermés et semaines lus au socle 5 ; absences au socle 15 ; Q-P30 | `planning.publie`, `planning.modifie` | — |
| PRS-134 | Demander un **échange de service** entre agents ; validation par le responsable (M-186) | AGT ; responsable fonctionnel | les deux agents d'accord ; compteurs recalculés | `planning.echange_valide` | — |
| PRS-135 | Voir **qui est de service maintenant** (poste, agent, moyen de joindre) (M-188) | accueil, vie scolaire, DIR ; port pour le socle 14 | d'après le planning, l'emploi du temps et les absences ; coordonnées professionnelles seules | — | — |
| PRS-136 | Voir les **intervenants et visiteurs attendus aujourd'hui** (M-189) | accueil, DIR | intervenants ponctuels, accès temporaires, engagements du jour ; référent affiché | — | — |
| PRS-137 | **Déclarer un lien personnel avec un élève** hors lien légal (M-236) | AGT ; DIR | R-P6 ; déport ; jamais visible des collègues | `personnel.lien_eleve_declare` (sans nature) | — |
| PRS-138 | Voir l'**organigramme fonctionnel** calculé (fonctions, missions), version réduite publiable aux familles (M-219, PER-03) | personnels ; FAM (réduite) | aucune coordonnée personnelle ; trame imprimable | — | — |
| PRS-139 | Désigner le **responsable fonctionnel** d'un agent (mission « encadrement », périmètre : agents) (M-220) | DIR | valideur des demandes des socles 15 et N1 ; un seul actif par agent et par nature de demande | `mission.attribuee` (type encadrement) | — |
| PRS-140 | **« Mes documents »** : recevoir les pièces qui le concernent (états de service, procès-verbal, arrêtés, attestations) ; dépôt en lot avec rattachement (M-218, PER-08) | SEC dépose ; AGT lit | moteur du socle 7 (rattachement, T-13) ; aucune pièce de santé | `document.depose` (7) | — |
| PRS-141 | **Partager ses créneaux occupés** d'un établissement ScolIA vers un autre, sans lieu ni nature (M-182) | AGT | D-013 : décision de l'agent seul, révocable ; l'autre établissement reçoit des indisponibilités | `personnel.indisponibilites` | — |
| PRS-142 | **Demander une attestation d'emploi**, délivrée selon une trame et signée par un habilité (M-217) | AGT demande ; SEC délivre | `peut_signer` (§8.9) ; données minimales (fonction, dates, quotité) | `document.emis` (7) | — |
| PRS-143 | Enregistrer une **vérification d'honorabilité** (date, auteur, résultat binaire, échéance) [NV] (M-214) | DIR | jamais le contenu ; alerte d'échéance ; blocage de planification paramétrable | `personnel.verification_enregistree` | — |
| PRS-144 | **Pouls d'humeur** anonyme du personnel (désactivé par défaut) (M-232, trou T-08) | Admin active ; AGT répond ; DIR lit les agrégats | R-P12 ; R-P24 ; Q-P32 | — | — |
| PRS-145 | **Relevé du service fait** par intervenant (séances assurées d'après les socles 10 et 12) (M-193) | SEC, comptabilité ; AGT (le sien) | export pour la paie (49) et l'état mensuel ; corrections tracées | `personnel.service_fait_arrete` | — |
| PRS-146 | **Changer la fonction** d'un agent en cours d'année (AED devenu enseignant) (M-222) | DIR, SEC ★ | engagement clos et nouveau, ou nouvelle fonction ; rôles proposés de nouveau, jamais conservés seuls | `personnel.fonctions_modifiees` | — |
| PRS-147 | **Suspendre un engagement** (disponibilité, détachement, congé parental) sans le clore, puis lever la suspension (M-221) | DIR, SEC ★ | R-P4 | `personnel.engagement_suspendu` / `_repris` | — |

Total après la passe : **118 actions** (86 + 32).

### 10.12 Règles ajoutées par la passe personas [personas]

| Id | Règle | Manque |
|---|---|---|
| R-P1 | **Engagements simultanés** permis s'ils diffèrent par l'employeur ou la nature (maître + salarié de l'OGEC ; AED + vacataire) ; l'exclusion porte sur (agent, employeur, nature) ; alerte si deux engagements simultanés ont le même employeur | M-191 |
| R-P2 | Toutes les dates métier dans le **fuseau de l'établissement** (profil du socle 5), Europe/Paris par défaut ; lot P0 inclus | M-215 |
| R-P3 | Une intervention commence par défaut à la **date de prise de poste** (installation ou début d'engagement), pas au premier cours | M-174 |
| R-P4 | **Suspension d'engagement** : accès suspendu par défaut (la casquette parent est conservée) ; services proposés en suppléance ou déclarés vacants ; missions à transférer (aperçu) ; la levée rétablit l'accès sans nouvelle invitation | M-221 |
| R-P5 | **Casquette élève et casquette personnel** sur la même personne : la portée de la casquette personnel **exclut** ses propres classes, groupes et cours, à toute date ; alerte à la direction à la création ; aucun rôle ne peut lever cette exclusion | M-187 |
| R-P6 | **Lien personnel déclaré** : déport proposé (conseil de classe, constitution, conseil de discipline, jury, commission) ; consultations du dossier de l'élève tracées à part, comme Q-P3 ; la nature du lien n'est lue que par l'agent et la direction | M-236 |
| R-P7 | La **note de passation** appartient au service : visible des intervenants actifs et suivants, du coordinateur, de la direction ; jamais des familles ni des élèves ; jamais exportée ; conservée le temps de l'année | M-177 |
| R-P8 | **Accès temporaire** : lecture seule, liste fermée de lectures, aucune donnée de vie scolaire, de santé ni de dossier administratif ; un jour, prolongeable à cinq ; enseignant concerné informé avant ; journal ; aucun compte permanent créé (casquette `intervenant` datée) | M-195 |
| R-P9 | **Fin d'engagement** : alertes à J-7 et J-1 (agent, direction) ; une prolongation saisie dans les 30 jours après la fermeture rouvre l'accès sans nouvelle invitation | M-179 |
| R-P10 | **Planning de service** : heures planifiées comparées aux heures dues de l'engagement (compteur), sans paie ; jours fermés et semaines A/B lus au socle 5 ; échanges validés par le responsable ; publication datée aux agents | M-185 |
| R-P11 | **Rémunération d'une mission** : nature et volume seulement, jamais de montant, de grade ni d'échelon ; le montant relève du socle 49 (privé) ou de l'agent comptable (public) | M-157 |
| R-P12 | **Pouls d'humeur** : désactivé par défaut ; aucune donnée nominative (pas d'`agent_id`), pas de texte libre ; agrégats affichés seulement à partir de 10 réponses par période et par famille de fonctions ; jamais croisé avec un autre indicateur | M-232 |
| R-P13 | **Motifs de fin** complétés (fin de période d'essai, rupture conventionnelle, licenciement, abandon de poste, détachement, radiation) [NV] ; toujours codés, jamais de texte | M-192 |
| R-P14 | **Qualifications** : codes ajoutés (sauveteur secouriste du travail, sécurité incendie, conduite d'engins, hygiène alimentaire, animation, surveillance de baignade) [NV] ; alerte d'expiration **avant les séances** qui l'exigent (atelier, piscine), pas seulement à date fixe | M-207, M-209 |
| R-P15 | Le **professeur principal** est visible des familles dès la publication des classes de N+1 (STR-78 du socle 5) ou, à défaut, dès l'ouverture de l'année | M-172 |
| R-P16 | **Accès de secours local** : cet établissement seulement, au plus 48 h, une fois par récupération ; alerte à la personne et dans chacun de ses établissements (D-018) ; clos à la fin de la récupération | M-181 |
| R-P17 | **Répartition prévisionnelle publiée** : accès aux cours de N+1 sans élèves (cahier, progression), marqué « prévisionnel » ; tout changement ultérieur est signalé à l'intéressé | M-159 |
| R-P18 | L'**invitation** peut partir sur une adresse personnelle quand l'adresse professionnelle n'existe pas encore ; l'adresse personnelle reste d'usage `perso` (jamais affichée) ; la bascule vers l'adresse professionnelle ne recrée rien | M-178 |
| R-P19 | **Prolonger une suppléance** décale la fin de l'intervention sans nouvelle annonce aux familles ni nouvelle note de passation ; seule la fin réelle est annoncée | M-223 |
| R-P20 | Les titulaires d'une **mission à périmètre d'élèves** (AESH, tuteur) reçoivent les changements de classe, de groupe et d'inclusion de ces élèves (événement `referentiel.composition_modifiee` du socle 5) | M-225 |
| R-P21 | L'**équipe pédagogique d'une classe** inclut les AESH dont un élève accompagné fait partie de la classe ou d'un cours de la classe à la date | M-226 |
| R-P22 | **Export du registre des entrées et sorties** des salariés de l'organisme (dates, nature, fonction), tracé, réservé à la comptabilité de l'OGEC et à la direction [NV] | M-194 |
| R-P23 | La **revue des accès** liste aussi les accès temporaires, les accès de secours et les clés prêtées | M-230 |
| R-P24 | Avant d'activer le planning de service, les indicateurs d'activité ou le pouls d'humeur, l'écran d'activation rappelle l'**information des personnels** et, dans le privé, la **consultation des représentants** [NV] ; l'activation enregistre la date de cette information | M-233 |
| R-P25 | **Contrat IA** (D-025) des actions ajoutées : commandes ★ (PRS-120, 125, 129, 146, 147) en confirmation renforcée avec aperçu serveur ; l'IA ne lit jamais les liens personnels, les vérifications d'honorabilité, le pouls ni les réserves d'un état de service | M-235 |
| R-P26 | Les **noms d'événements** consommés s'alignent sur ceux du socle 5 (`referentiel.annee_ouverte`, `referentiel.annee_cloturee`, `referentiel.classe_archivee`, `referentiel.groupe_ferme`) | M-234 |

---

## 11. Ce qui relève du logiciel, ce qui relève du paramétrage

### 11.1 Tableau

| Sujet | Codé par ScolIA (non paramétrable) | Paramétré par l'établissement |
|---|---|---|
| Natures d'engagement, types d'employeur | les **codes** (exports, règles de fin obligatoire) | libellés ; natures désactivées |
| Fonctions | familles de fonctions, fonctions fournies | libellés, fonctions créées, modèles de rôles proposés par fonction |
| Services | modèle (unité + interventions), règle « une saisie par jour », continuité | modalités utilisées, codes, heures, alertes de couverture, droits par nature d'intervention |
| Missions | moteur (périmètre, dates, cardinalité, rôle porté), types réservés aux socles 35 et 39 | types créés, libellés, cardinalité du PP, éligibilité (avertir / bloquer), modèle de rôle porté (anti-escalade) |
| Équipes | équipes calculées et leurs règles | types et membres des équipes constituées |
| Droits | catalogue d'actions, portées, anti-escalade, dernier administrateur, casquettes séparées | rôles et modèles, délais de grâce, lecture du titulaire pendant une suppléance |
| Accès | invitation sans mot de passe, fermeture à la date, révocation | durées d'invitation, accès réduit avant arrivée, intervenants avec ou sans compte |
| Parcours | moteur et étapes automatiques | étapes, responsables, natures et fonctions concernées |
| Confidentialité | jamais de coordonnée personnelle aux familles ; jamais de motif ; identité restreinte possible | ce que les familles et les élèves voient de l'équipe (fonctions affichées, photo) |
| Indicateurs d'activité | — | activés ou non (désactivés par défaut) |

### 11.2 Préréglages par profil d'établissement [R]

Mêmes profils que `03_INSCRIPTIONS.md` §4.6 et `04_ADMINISTRATIF.md` §10.2.

| Profil | Préréglages |
|---|---|
| **Collège public** | natures État + collectivité ; import STS-web et AAF activés ; PP : 1 par classe ; référents harcèlement, décrochage, numérique ; agents de la collectivité (département) sans compte par défaut, sauf accueil |
| **Lycée public (général, technologique, professionnel)** | idem, collectivité = région ; PP : 2 autorisés en terminale ; DDFPT en lycée professionnel [NV] ; co-intervention ; référent vie lycéenne |
| **Privé sous contrat** | maîtres (État) + salariés de l'organisme ; import STS-web pour les maîtres [H] ; animateur en pastorale ; chef d'établissement nommé par la tutelle ; parents bénévoles fréquents ; casquettes professeur-parent activées |
| **Privé hors contrat** | salariés et bénévoles ; aucun import national ; import CSV |
| **Ensemble scolaire** | selon la décision du socle 5 (§5.4 point 5) |

---

## 12. Échanges avec l'extérieur

| Flux | Sens | Public | Privé sous contrat | Hors contrat | Statut des spécifications | Règle |
|---|---|---|---|---|---|---|
| **STS-web** (`sts_emp_<UAI>.xml` : individus, structures, services [S `INTEROPERABILITE_NATIONALE.md` §6]) | entrant ; sortant si ScolIA fait l'emploi du temps | oui | maîtres [H] | non | formats en bascule (OMOGEN annoncé pour 2026-27 [S-tiers]) : **ne pas coder avant confirmation** [S §14 risques] | agents, disciplines, services `prevu` ; rapprochement par identifiant STS, puis nom + prénom + discipline, avec choix humain |
| **AAF** (`PersEducNat`) | entrant, quotidien | oui, sous convention DSI académique [S] | non en général [S] | non | **spécification à obtenir** | crée agents et jointures ; jamais de compte ; jamais d'écrasement d'une correction locale |
| **Guichet Agents** (fédération) | connexion | obligatoire [S] | maîtres : **[NV]** | non | protocole à confirmer (OIDC ou SAML) [S §4] | rattachement par jointure à un agent importé, jamais par e-mail ; exception à la passkey obligatoire à trancher (décision 1 de `INTEROPERABILITE_NATIONALE.md` §15) |
| **SSO de l'établissement** (Google, Microsoft) | connexion | possible | fréquent [H] | fréquent | existant `ent-oidc` [F] | même règle de rattachement : invitation d'abord, puis SSO accepté comme moyen de connexion du compte rattaché |
| **Annuaire d'entreprise** (Microsoft Entra ID, Google Workspace) | entrant, facultatif | rare | possible | possible | à étudier | **pas par défaut** (dépendance cloud) ; si activé : propositions seulement, jamais de création de droits |
| **Logiciels d'emploi du temps** (exports services et professeurs) | entrant / sortant | oui | oui | oui | formats propriétaires [H] ; import EDT existant [F `060`] | services et interventions par rapprochement ; socle 10 pour les créneaux |
| **GAR** (annuaire : enseignants, groupes, rattachements) | sortant | oui [S] | oui [S, arrêté de 2017 cité] | — | référentiel technique [S] | profils GAR déduits des rôles ; pseudonymes par service |
| **CSV de reprise** | entrant | reprise | reprise | voie principale | modèle ScolIA | socle 28 |
| **Organisme de gestion, tutelle** | sortant | — | listes de personnels | listes | — | export tracé, minimal |
| **Logiciel de paie** (privé) | sortant | — | futur | futur | socle 49 | engagements, jamais l'inverse |

Règle commune : **un import propose, l'établissement dispose** (socle 9, A18) ; une donnée corrigée à la main est marquée et jamais écrasée sans choix humain ; chaque lot est annulable.

---

## 13. Sécurité et données sensibles

### 13.1 Cloisonnement, intégrité, garde-fous

- Toutes les tables du socle portent `etablissement_id`, des clés composées et la **RLS** (permissive puis stricte, comme `100_rls_pilote.sql` [F]).
- Un agent n'existe que dans un établissement ; aucune requête ne joint deux établissements ; la vue « mes établissements » d'un agent passe par la couche identité (D-013), pour lui seul.
- **Dernier administrateur** : toute commande qui ferme ou suspend un accès, retire le profil Admin ou clôt un rôle de gestion vérifie qu'il reste au moins un accès Admin **ouvert et enrôlé** ; sinon refus avec message.
- **Anti-escalade** inchangée, étendue aux missions (le modèle de rôle d'un type de mission ne peut être choisi que par qui couvre ses droits) et aux délégations.
- **Séparation facultative des tâches** [R] : l'attribution du profil Admin ou d'un rôle contenant `roles.gerer` peut exiger la confirmation d'un second administrateur (paramètre, désactivé dans les petits établissements).

### 13.2 Comptes du personnel : la cible la plus précieuse

- **Passkey obligatoire en première vague** pour tous les comptes à casquette personnel (D-009) ; enrôlement à l'acceptation de l'invitation ; plusieurs passkeys ; **clés de sécurité matérielles** acceptées et recommandées pour les agents sans appareil personnel (cas P40).
- **Aucun secret transmis par l'établissement** : ni mot de passe temporaire, ni code par téléphone ; invitation à usage unique, jeton haché, expiration courte.
- **Postes partagés** (salle des professeurs, accueil, CDI) : sessions web courtes (D-011), connexion par passkey du téléphone (QR) ; **aucun compte partagé** (un agent d'accueil = un compte).
- **Départ** : fermeture à la date, sessions de l'établissement révoquées, jetons push supprimés ; l'appli mobile retire l'établissement à la prochaine synchronisation ; zéro compte personnel ouvert sans engagement actif (tuile de qualité, revue des accès).
- **Suspension conservatoire** : effective en moins d'une minute (droits lus à chaque requête [F] + révocation des sessions).
- **Récupération de passkey** : D-018 ; pour un agent d'un seul établissement, récupération par la direction possible, tracée.
- **Accès massifs** : détection (PRS-104) ; les professeurs n'exportent pas (D-022).

### 13.3 Groupes de lecture (minimisation)

| Groupe | Contenu | Qui [R] |
|---|---|---|
| **P0 Désignation** | civilité, nom d'usage, prénom (ou initiale si identité restreinte), fonctions, disciplines | personnels de l'établissement ; familles et élèves pour l'équipe de l'élève seulement, sous le nom affiché aux familles |
| **P1 Contact professionnel** | messagerie interne (toujours), e-mail professionnel, poste téléphonique de l'établissement, jours de présence, présence du jour (d'après l'emploi du temps) | personnels ; présence du jour : direction, secrétariat, vie scolaire, accueil |
| **P2 Organisation** | services, heures, missions, équipes, engagements (nature, dates, quotité), qualifications | direction, secrétariat ; l'agent pour lui-même ; le PP voit les services de sa classe |
| **P3 Personnel** | coordonnées personnelles (téléphone, e-mail), contact d'urgence | direction ; et collègues **seulement si l'agent l'a choisi** |
| **P4 Confidentiel** | motif de suspension, identité restreinte, motif de fin, indicateurs d'activité | chef d'établissement et adjoints |
| **P5 Accès** | compte, casquettes, rôles, passkeys (nombre, dates), sessions, journal | direction ; l'agent pour lui-même |

**Aucune coordonnée personnelle d'un agent n'est jamais visible d'une famille ou d'un élève**, quel que soit le paramétrage. Correctif immédiat de l'annuaire existant (§2.2 n° 12, lot P0) : `search_personnel` ne renvoie plus le téléphone.

### 13.4 Données qu'on ne collecte pas [R]

Adresse personnelle, date et lieu de naissance, nationalité, situation de famille, numéro de sécurité sociale, NUMEN, grade, échelon, rémunération, motif médical, appartenance syndicale. Si un flux national exige l'une d'elles (jointure STS ou AAF [NV]), elle est **reçue de l'import, chiffrée** (`champs_chiffres.rs` [F]), jamais affichée, et la question est posée au DPO avant (Q-P15). Le socle 49 (privé) collectera ce que la paie exige, dans son propre périmètre.

### 13.5 Les personnels comme personnes concernées (RGPD)

- Information des personnels sur les traitements les concernant (notice dédiée, à faire valider par le DPO) ; **droit d'accès** : l'agent voit sa fiche, ses droits, et qui (par fonction) a consulté sa fiche (PRS-106).
- **Indicateurs d'activité** : finalité (aider, pas sanctionner), désactivés par défaut, information préalable ; dans le privé, consultation des représentants du personnel probablement requise avant mise en œuvre [NV] ; jamais de classement ni d'affichage aux collègues (socle 47).
- **Droit à la déconnexion** [NV] : pause des notifications, heures calmes (socle 8), pas de notification pendant un congé.
- **Conservation** : fiche et engagements après le départ ; journal des accès ; invitations ; revues des accès : **durées à demander au DPO** (Q-P20), aucune durée codée avant. Repères sourcés à lui soumettre (VERIF R43-R45) : journaux d'accès « entre six mois et un an » **[V]** (CNIL, recommandation journalisation, délibération 2021-122) ; personnels de droit privé : référentiel CNIL « durées de conservation – gestion des ressources humaines » (avril 2026) **[V]**, qui exclut le dossier des agents publics ; bulletins de paie : double conservé 5 ans (C. trav. L3243-4) **[V]**. L'auteur des saisies (notes, appels) reste affiché tant que la saisie est conservée : c'est la preuve de l'acte, pas une donnée de profil.
- **Analyse d'impact** : le socle traite des droits d'accès à des données de mineurs à grande échelle : l'AIPD de l'ENT doit couvrir le moteur de droits et ce socle [H].

---
## 14. Droits (catalogue `ent_auth::Action`)

### 14.1 Existant [F]

Aucune action propre aux personnels, aux comptes, aux services, aux missions, aux équipes ni aux délégations [F `actions.rs:183-230`]. Les services et le PP passent par `referentiel.gerer` (Admin, Direction, Administratif) ; la lecture du référentiel par `referentiel.lire` (tout le personnel) ; la création des comptes du personnel par un contrôle de profil codé en dur (Admin, Direction) [F `service.rs:3182-3185`] ; les responsables de groupes et les rôles par `roles.gerer` (Admin, Direction) ; l'annuaire du personnel par `Role::is_staff` [F `routes.rs:1040-1046`].

### 14.2 Actions proposées [R]

| Action | Libellé | Portées admises | Modèle par défaut (= comportement actuel quand il existe) |
|---|---|---|---|
| `personnels.annuaire` | Annuaire interne (P0, P1) | établissement, sites désignés | tout le personnel, établissement (comportement actuel de `/personnel/search`, **sans le téléphone**) |
| `personnels.lire` | Fiche d'organisation des agents (P2) | établissement, sites désignés, `mon_perimetre` (agents désignés : tuteur) | Admin, Direction, Administratif [R] |
| `personnels.personnel.lire` | Coordonnées personnelles et contact d'urgence (P3) | établissement | Admin, Direction |
| `personnels.confidentiel.gerer` | Suspension, identité restreinte, motifs (P4) | établissement | Admin, Direction |
| `personnels.gerer` | Créer, modifier les agents, engagements, fonctions, qualifications ; parcours | établissement, sites désignés | Admin, Direction (= `provisionner_personnel` actuel) ; Administratif [R, nouveau, Q-P23] |
| `personnels.acces.gerer` | Inviter, ouvrir, suspendre, fermer l'accès ; révoquer les sessions ; réinitialiser les passkeys d'un agent | établissement | Admin, Direction (= `cible_compte_autorisee` actuel pour les identifiants [F `service.rs:2632`]) |
| `personnels.services.lire` | Répartition des services et missions | établissement, sites désignés, `mon_perimetre` | tout le personnel [R] (aujourd'hui `referentiel.lire`) |
| `services.gerer` | Services et interventions de l'année ouverte | établissement, sites désignés | Admin, Direction, Administratif (= `referentiel.gerer`) |
| `services.preparer` | Répartition de l'année en préparation, scénarios | établissement | Admin, Direction |
| `missions.gerer` | Nommer, terminer, transférer missions et responsabilités de groupes | établissement | Admin, Direction, Administratif (= `referentiel.gerer` pour le PP, `roles.gerer` pour les responsables de groupes) |
| `missions.types.gerer` | Types de missions de l'établissement | établissement | Admin |
| `equipes.gerer` | Équipes constituées | établissement, `mon_perimetre` (coordinateur) | Admin, Direction, Administratif, CPE [R] |
| `delegations.accorder` | Délégations datées de ses propres missions et rôles | `moi` | Admin, Direction [R] ; ouvrable aux PP et coordinateurs |
| `signature.habiliter` | Pouvoirs de signature, intérim de direction | établissement | Admin, Direction (chef d'établissement, Q-P24) |
| `acces.revue` | Revue des accès, explication des droits d'autrui, simulation | établissement | Admin, Direction |
| `personnels.indicateurs.lire` | Indicateurs d'activité par agent | établissement | **personne par défaut** ; activable pour Direction (Q-P21) |
| `personnels.importer` | Imports STS-web, AAF, CSV, logiciels d'emploi du temps | établissement | Admin, Direction, Administratif |
| `personnels.exporter` | Exports des personnels et des services | établissement | Admin, Direction, Administratif (D-022) |
| `personnels.parametrer` | Paramétrage du domaine | établissement | Admin |

Pour **soi-même** (portée `moi`, sans action à accorder) : voir sa fiche, ses services, ses missions, ses droits et leur explication ; accepter une invitation ; choisir sa photo, ses coordonnées partagées, son contact d'urgence ; mettre en pause ses notifications ; suspendre son accès pendant un congé ; déclarer ses indisponibilités ; consulter qui a vu sa fiche. Ces droits existent par construction et ne se retirent pas.

Règles [R] :
- **Compatibilité** : modèles par défaut qui reproduisent exactement les droits actuels (test oracle profil × action, comme `r2_modele_identique_aux_capacites_historiques` [F]) : `referentiel.gerer` détenu ⇒ `services.gerer` et `missions.gerer` accordés dans les modèles existants ; Admin et Direction ⇒ `personnels.gerer` et `personnels.acces.gerer`.
- **Inclusions explicites** : `personnels.gerer` inclut `personnels.lire` ; `services.gerer` inclut `personnels.services.lire` ; `personnels.confidentiel.gerer` inclut la lecture de P4. Aucune autre inclusion implicite.
- `personnels.acces.gerer`, `personnels.confidentiel.gerer`, `signature.habiliter`, `acces.revue` : portée `etablissement` seulement, et ne se donnent que par qui les détient (anti-escalade).
- **Nouvelles portées** (socle 1, lot P4) : `mon_perimetre`, `moi`, `sites_designes` ; redéfinition de `mes_classes` par les interventions (§9.1), libellé « mes élèves par l'enseignement », code inchangé.
- **Modèles de rôles fournis** [R], copiables et modifiables (D-017) : Professeur ; Professeur principal (porté par la mission) ; CPE ; CPE référent (porté par la mission) ; Assistant d'éducation ; Surveillant d'internat ; Accueil ; Secrétariat de scolarité ; Secrétariat de direction ; Comptabilité ; Direction ; Infirmerie ; Psychologue ; Assistant de service social ; Documentaliste ; Accompagnement AESH (porté par la mission du socle 35) ; Intervenant (appel de son groupe, cahier en lecture) ; Restauration ; Maintenance (aucune donnée d'élève).
- Les routes ne font qu'un pré-filtre ; le service refait le contrôle précis, comme en R2 [F].

### 14.3 Droits des actions ajoutées [personas]

| Action | Libellé | Portées admises | Modèle par défaut |
|---|---|---|---|
| `planning.gerer` | Postes et planning de service, validation des échanges (PRS-133, 134) | établissement, sites désignés, `mon_perimetre` (agents encadrés) | Admin, Direction ; CPE et gestionnaire en `mon_perimetre` [R] |
| `planning.lire` | Qui est de service maintenant, visiteurs attendus (PRS-135, 136) | établissement, sites désignés | Direction, vie scolaire, accueil |
| `acces.temporaire` | Accès de consultation temporaire et accès de secours (PRS-120, 125) | établissement | Admin, Direction ; anti-escalade |
| `services.voeux` | Ouvrir une campagne de vœux, lire les vœux, arbitrer les propositions (PRS-127 à 129) | établissement, `mon_perimetre` (discipline du coordinateur) | Direction ; coordinateur en `mon_perimetre` |
| `personnels.service_du.gerer` | Composantes du service dû, états de service, service fait (PRS-130, 131, 145) | établissement | Admin, Direction, Administratif |
| `personnels.verifications` | Vérifications d'honorabilité (PRS-143) | établissement | Direction seulement |

Les autres actions ajoutées utilisent `personnels.gerer`, `personnels.acces.gerer`, `missions.gerer`, `personnels.exporter` et la portée `moi` (§14.2). Le pouls (PRS-144) n'a **aucune** action de lecture nominative : ses agrégats passent par `personnels.indicateurs.lire`, désactivée par défaut.

---

## 15. Événements

**Charge utile : identifiants, codes, dates, noms de champs ; jamais de nom, de coordonnée, de motif détaillé.** Noms en style français (`personne.creee`, `lien.suspendu` [F]). Les événements prévus par `SOCLE2_DOMAINES.md` §4 (`referentiel.service_cree` / `_cloture`, `referentiel.pp_designe`, `delegation.accordee` / `_expiree`, `compte.provisionne`, `compte.role_change`, `compte.desactive`, `rh.fiche_modifiee`) sont **remplacés** par ceux-ci, avec correspondance dans le lot P1.

### 15.1 Émis

| Événement | Charge utile | Consommateurs [R] |
|---|---|---|
| `personnel.cree` | `agent_id`, `personne_id`, source | annuaire, recherche, 28 (reprise) |
| `personnel.engagement_ajoute` / `_modifie` | `agent_id`, `engagement_id`, nature, dates | 10 (jours travaillés), 49 (futur), 46 (corps électoral) |
| `personnel.fonctions_modifiees`, `personnel.disciplines_modifiees`, `personnel.qualification_modifiee` | `agent_id`, codes | 14 (secourisme), 42, 47 |
| `personnel.invite`, `personnel.acces_rattache` | `agent_id`, canal | 8 (envoi de l'invitation), parcours |
| `personnel.arrive` | `agent_id`, date | 8 (accueil), 10, 16 (listes de diffusion), 29 (à-faire de l'arrivant) |
| `personnel.depart_planifie` | `agent_id`, date | 10, 12, 20, 21, 23 (tâches en cours à signaler), 44 (matériel) |
| `personnel.parti` | `agent_id`, date, motif codé | 1 (sessions), 8 (annuler programmations ; aucun message si `deces`), 10 (séances futures orphelines), 16 (retrait des listes), 46, 47 |
| `acces.suspendu` / `acces.retabli` / `acces.ferme` | `user_id`, casquette, motif codé (sans détail) | 1, 11 (application mobile), 8 |
| `personnel.identite_restreinte` | `agent_id`, posée / levée | 7, 8, 16, portails (affichage) |
| `service.cree` / `_modifie` / `_clos` / `_annule` / `_remplace` | `service_id`, année, cible, matière, période | 10 (créneaux), 12, 20, 21, 23, 9 (GAR) |
| `service.intervenant_ajoute` / `_termine` / `intervention_corrigee` | `service_id`, `agent_id`, nature, dates | 1 (aucune action : droits calculés), 10 (professeur des créneaux), 12 (qui fait l'appel), 16, 9 (GAR) |
| `service.suppleance_ouverte` / `_close` | `service_id`, suppléant, titulaire, dates, `origine_ref` | 8 (information des familles, sans motif), 10, 15, 20 |
| `service.vacant` / `_pourvu` | `service_id`, depuis | 10 (cours non assurés), 8 (familles), 15, 29 (à-faire de la direction) |
| `service.prevu_cree`, `service.ouvert` (lot) | identifiants, année | 10 (préparation de l'emploi du temps) |
| `mission.attribuee` / `_terminee` | `mission_id`, type, `agent_id`, périmètre (identifiants), dates | 16 (« écrire au PP »), 19, 23 (conseils), 35, 39, portails (équipe de l'élève) |
| `groupe.responsable_ajoute` / `_termine` | `groupe_id`, `agent_id`, dates | 43, 16 |
| `equipe.creee` / `membres_modifies` / `close` | `equipe_id`, type | 16, 19 |
| `delegation.accordee` / `_revoquee` / `_expiree` | `delegation_id`, délégant, délégataire, objet, dates | 8 (prévenir les deux), 7 (signature) |
| `signature.habilitation_modifiee` | `agent_id`, natures de documents | 7 |
| `acces.revue_close` | `revue_id`, compteurs | 27 (preuve de conformité), 47 |
| `personnel.indisponibilites` | `agent_id`, période | 10, 19 |

### 15.2 Consommés

| Événement | Producteur | Réaction du socle 6 |
|---|---|---|
| `referentiel.annee_ouverte` | 5 | ouvrir les services `prevu` de l'année (si la direction l'a validé, PRS-52) ; clore les missions annuelles de l'année précédente |
| `referentiel.annee_cloturee` | 5 | clore services et missions annuelles restées ouvertes ; proposer la reconduction |
| `referentiel.classe_archivee`, `referentiel.groupe_ferme` | 5 | clore les services et responsabilités de la cible, alerte si saisies ouvertes |
| `personne.decedee` | 4 | si la personne a un agent : PRS-35 proposé à la direction (jamais automatique) |
| `personne.fusionnee` / `defusionnee` | 4 | rattacher l'agent à la personne conservée |
| `passkey.enrolee`, `compte.premiere_connexion` | 1 | étapes de parcours cochées |
| `document.signe` | 7 | étape « charte signée » |
| `rh.absence_validee` | 15 | **aucune écriture** : le socle 15 appelle `nommer_suppleant` s'il organise une suppléance |
| `materiel.restitue` | 44 | étape de départ |
| `import.lot_applique` | 9 | propositions à traiter (STS-web, AAF) |

**[personas]** R-P26 **appliquée au lot P1** : le tableau ci-dessus porte désormais les noms qu'émet le socle 5 (`referentiel.annee_ouverte`, `referentiel.annee_cloturee`, `referentiel.classe_archivee`, `referentiel.groupe_ferme`) ; les constantes sont dans `ent_events::types` (`REFERENTIEL_*`). Aucun consommateur n'est écrit au lot P1 (ils arrivent avec P3 et P8).

### 15.3 Événements ajoutés [personas]

| Événement | Sens | Charge utile | Consommateurs ou producteur |
|---|---|---|---|
| `personnel.installe`, `personnel.dossier_soumis`, `personnel.dossier_valide` | émis | `agent_id`, date | parcours, 49 (prise de fonctions) |
| `personnel.engagement_suspendu` / `_repris` | émis | `engagement_id`, dates | 1 (accès), 10, 15, 8 |
| `acces.secours_ouvert`, `acces.temporaire_ouvert` / `_clos` | émis | `user_id` ou `personne_id`, durée | 1, 8 (alerte à la personne), revue des accès |
| `service.note_passation_ecrite` | émis | `service_id` | 8 (intervenants du service) |
| `intervention.programmee` | émis | séances | 10, 12 (liste de la séance) |
| `service.voeux_deposes`, `service.repartition_proposee`, `service.repartition_publiee` | émis | `annee_id`, compteurs | 8, 10 (vœux d'emploi du temps) |
| `personnel.service_du_modifie`, `personnel.etat_service_signe`, `personnel.service_fait_arrete` | émis | `agent_id`, année ou période | 49, 47 |
| `mission.remuneration_modifiee` | émis | `mission_id`, nature | 49 |
| `planning.publie`, `planning.modifie`, `planning.echange_valide` | émis | postes, agents, dates | 14 (présents), 15, 8 |
| `personnel.lien_eleve_declare` | émis | `agent_id`, `eleve_id` (**sans nature**) | 5 (constitution), 13, 23 (déport) |
| `personnel.verification_enregistree` | émis | `agent_id`, échéance | parcours, planification |
| `referentiel.composition_modifiee` (socle 5, avec inclusions) | consommé | groupe ou cours, entrants, sortants | prévenir les titulaires de missions sur ces élèves (R-P20) |
| `referentiel.groupe_scinde` (socle 5) | consommé | groupes | service à pourvoir signalé à la direction |

---

## 16. Écrans clés

Leptos 0.8 (`docs/FRONTEND_LEPTOS.md`), actions **proposées par le serveur**, design system maison ; réutilise l'écran d'administration des rôles existant [F `modules/roles`].

| Écran | Route [R] | Contenu | Pour qui |
|---|---|---|---|
| **Annuaire des personnels** | `/web/personnels` | tableau dense et vue trombinoscope ; filtres fonction, discipline, site, nature, état (prévu, actif, parti), « présents aujourd'hui » ; recherche ; pastilles (sans passkey, invitation en attente, sans date de fin) ; actions en lot (inviter, relancer, exporter) | direction, secrétariat ; vue réduite P0–P1 pour tous |
| **Fiche agent** | `/web/personnels/{id}` | onglets : **Identité et contact** (socle 4, groupes de lecture) ; **Engagements** (frise des périodes) ; **Fonctions, disciplines, qualifications** ; **Services** (année, heures dues / attribuées, historique) ; **Missions et équipes** ; **Accès et droits** (casquettes, état, passkeys, sessions, rôles effectifs, « pourquoi il voit… », simulation) ; **Parcours** ; **Journal** | direction, secrétariat |
| **Arrivée guidée** | assistant | recherche de la personne (homonymes, « est-ce le père de Léa ? ») → nature, dates, employeur, quotité → fonctions et disciplines → **services** (vacants proposés, ou reprise d'un prédécesseur) → missions (PP ?) → rôles proposés selon la fonction, avec **simulation** → invitation (e-mail ou QR) → récapitulatif ★ | direction, secrétariat |
| **Rentrée des personnels** | `/web/personnels/rentree` | import STS-web ou CSV → rapprochement → invitations en lot → **séance d'enrôlement** : grand QR code par personne sur l'écran de la salle des professeurs, suivi en direct de qui a activé → relances | secrétariat |
| **Départ guidé** | assistant | date et motif codé → **services** (successeur, suppléant, vacant ; par service) → missions → délégations → **tâches en cours** (comptes par socle, lien vers chaque liste) → matériel → information des familles (texte proposé, envoi humain) → récapitulatif avant / après ★ | direction |
| **Répartition des services** | `/web/services` | matrice **classes × matières** (cellule = intervenants, heures, état) avec couverture (prévu socle 5 / attribué) ; vue **par professeur** (heures dues, attribuées, écart, disciplines, quotité) ; glisser un professeur sur une cellule ; groupes dépliables ; alertes (vacants, doublons, sur-service, hors discipline, créneaux sans service) ; bascule **année ouverte / année en préparation** ; scénarios ; comparaison N / N+1 ; exports | direction ; lecture pour le personnel |
| **Suppléance** | action sur un service ou depuis une absence (socle 15) | agent (recherche, disponibilité d'après l'emploi du temps et les indisponibilités) → dates, recouvrement → ce que le suppléant verra (simulation) → information des familles ★ | direction |
| **Missions et référents** | `/web/missions` | tableau **PP par classe** (trous en rouge, candidats proposés : professeurs de la classe, heures dans la classe), saisie en lot ; référents et coordinateurs par type ; types de l'établissement | direction |
| **Équipes** | `/web/equipes` | équipes calculées (aperçu à la date, par classe, niveau, discipline) ; équipes constituées (membres datés, diffusion, confidentialité) | direction, CPE, coordinateurs |
| **Délégations et intérim** | `/web/direction/delegations` | délégations en cours et à venir, expirations, intérim de direction, pouvoirs de signature | chef d'établissement |
| **Revue des accès** | `/web/securite/revue-acces` | une ligne par accès personnel : casquettes, rôles, dernière connexion, passkey, engagement, largeur (élèves visibles), constats ; décisions en lot ; clôture signée ; historique des revues | direction |
| **Qualité des personnels** | `/web/personnels/qualite` | tuiles PRS-100, chaque tuile ouvre la liste et l'action | direction, secrétariat |
| **Mon espace** (agent) | `/web/moi` | mes informations, mes établissements (D-013, moi seul), mes services et heures, mes missions et équipes, **mes droits expliqués**, photo et coordonnées partagées, pause des notifications, indisponibilités, qui a vu ma fiche ; bascule de casquette | tout agent |
| **L'équipe de mon enfant** | `/web/famille/equipe` + appli | par matière : professeur (nom affiché aux familles), suppléance en cours (« remplace M. A. jusqu'au 3 avril »), PP, CPE, direction, référents visibles ; bouton écrire (socle 16) | familles |
| **Mes professeurs** | `/web/eleve/professeurs` + appli | idem | élèves |
| **Paramétrage** | `/web/direction/parametrage/personnels` | profil d'établissement ; natures et fonctions ; modèles de rôles par fonction ; types de missions ; règles du PP ; droits par nature d'intervention ; délais de grâce ; parcours ; visibilité familles ; intervenants ; indicateurs | administrateur |
| **Mobile** (appli unique, D-003) | — | **agent** : acceptation de l'invitation et enrôlement de la passkey par QR, mes services, annuaire, équipe d'une classe, bascule de casquette, pause des notifications ; **vie scolaire** : « où est M. X maintenant » ; **famille, élève** : équipe | selon droits |

Critères d'ergonomie [R] : clavier complet, lecteur d'écran, 375 px ; aucune action sensible sans récapitulatif ; tout masquage expliqué ; arrivée guidée en moins de 3 minutes ; séance d'enrôlement de 30 personnes en moins de 20 minutes ; répartition de 60 classes fluide (tableau virtualisé, design system, D-010 préalable 5).

---

## 17. Plan de migration depuis l'existant

Règles : migrations **additives** ; reprise idempotente ; données imprévues rangées « à traiter » et signalées, jamais rejetées ; répétition sur base de labo et jeux anonymisés, **jamais** sur la base de production (interdite) ; ancienne interface `ui.rs` figée ; colonnes héritées conservées en projection (triggers) tant que des lecteurs existent. Numéros dans la plage **350–369**. Les clés composées et la RLS rejoignent le **lot transverse « clés et RLS »** proposé par `04_ADMINISTRATIF.md` Q-A20.

| N° | Étape | Contenu | Reprise / compatibilité | Vérification |
|---|---|---|---|---|
| 350 | **Accès : état et dates** (socle 1, lot P0) | `ent_membership.etat`, `etat_depuis`, `etat_motif`, `etat_par_user_id` ; contrôle de session étendu (`etat = 'ouvert'`) | tous les accès existants `ouvert` | test : accès fermé ⇒ connexion et session refusées ; dernier administrateur protégé |
| 351 | **Attributions datées** (socle 1, lot P0) | `ent_role_attribution.id`, contrainte d'exclusion datée, `source`, `source_ref`, `clos_le`, `perimetre` ; début futur permis ; fin de l'`ON CONFLICT` | une ligne existante = une attribution `manuelle` | historique conservé après réattribution |
| 352 | **Agents** | `ent_agent` ; `ent_agent_engagement` | un agent par `(etablissement_id, user_id)` dont le profil est du personnel ; **personne créée** par la même règle que `120` (nom affiché repris dans `nom`, signalé « à séparer ») quand elle n'existe pas, **réutilisée** quand le compte a déjà une personne (professeur-parent) ; engagement `a_preciser`, `du = membership.created_at` (date de Paris), `au = NULL` | un agent par compte du personnel ; aucune personne en double |
| 353 | **Fonctions, disciplines, qualifications** | `ent_fonction` (+ catalogue), `ent_agent_fonction`, `ent_agent_discipline`, `ent_agent_qualification`, `ent_agent_contact_urgence` | fonction déduite du profil (prof → enseignant, cpe → CPE, infirmerie → infirmier, psy → psychologue, compta → comptable, administratif → secrétariat, direction → direction, admin → `a_preciser`) | aucun agent sans fonction principale |
| 354 | **Services : unité et interventions** | colonnes du §8.5 sur `ent_service_enseignement` ; `ent_service_intervenant` ; `prof_user_id` rendu nullable et maintenu par trigger (intervenant en saisie) ; nouvel index d'unicité | une intervention `titulaire` par service (agent du `prof_user_id`, `du` = début de l'année) ; `etat = ouvert`, `source = reprise` | `prof_user_id` = projection pour tous les services ; `enseigne_a` inchangé sur les données reprises |
| 355 | **Missions** | `ent_mission_type` (+ types fournis), `ent_mission`, `ent_mission_perimetre` ; trigger qui maintient `professeur_principal` ; suppression de `ent_service_pp_uq` remplacé par la cardinalité | chaque `professeur_principal = true` → mission PP (classe, `du` = début de l'année) | un PP par classe après reprise, identique |
| 356 | **Responsabilités de groupes datées** | `ent_groupe_responsable.id`, `agent_id`, `nature`, `du`, `au` | `du = created_at` (date de Paris), `nature = responsable` | portée `mes_groupes` inchangée sur les données reprises |
| 357 | **Casquettes et invitations** (socle 1 + 6) | `ent_membership_casquette`, `ent_session.casquette`, `ent_agent_invitation` | une casquette par accès existant, depuis `role` ; ajout d'une casquette `parent` aux comptes du personnel qui ont un lien actif (professeurs-parents) **sans l'ouvrir** tant que la bascule n'est pas livrée | aucune session existante invalidée |
| 358 | **Équipes** | `ent_equipe_type`, `ent_equipe`, `ent_equipe_membre` ; fonctions SQL des équipes calculées | — | équipe pédagogique de la 4B du labo = attendu |
| 359 | **Délégations et signature** | `ent_delegation`, `ent_habilitation_signature` | chef d'établissement : habilitation complète par fonction | — |
| 360 | **Parcours** | `ent_parcours_modele` (+ modèles fournis), `ent_parcours_agent`, `ent_parcours_etape` | aucun parcours pour les agents repris | — |
| 361 | **Identifiants externes** | `ent_agent_identifiant` | — | — |
| 362 | **Agent comme sujet des autres tables** | `agent_id` sur `ent_edt_creneau`, `ent_edt_seance` (+ `remplacant_agent_id`), `ent_absence_rh` ; triggers depuis les `*_user_id` | rempli depuis l'agent du compte | aucune ligne sans `agent_id` quand le compte a un agent |
| 363 | **Revue des accès** | `ent_revue_acces`, `ent_revue_acces_ligne` | — | — |
| 364 | **Paramétrage** | `ent_param_personnels` | valeurs par défaut = comportement actuel (aucun délai de grâce, titulaire en lecture, PP unique) | oracle de comportement |
| 365 | RLS permissive | toutes les tables du socle | dépôts en `TenantTx` | tests sous rôle non propriétaire |
| 366 | RLS stricte | idem | — | idem |
| 367 | **Bascule du périmètre** | `eleves_suivis_par_prof` et `prof_suit_eleve_le` lisent les interventions ; fenêtre de ±14 jours remplacée par la séance ; repli classe texte derrière un drapeau par établissement | drapeau `perimetre_strict = false` pour les établissements existants ; `true` à la création | tests D-022 : LV2 dans une classe mixte, suppléance, co-enseignement, remplacement d'une séance, fin de service, délai de grâce |
| 368–369 | **Retrait des projections** | `ent_service_enseignement.prof_user_id`, `professeur_principal` ; `ent_groupe_responsable.user_id` ; repli classe texte | **après** bascule de tous les lecteurs (grep à zéro) et validation | migrations réservées |

---

## 18. Découpage en lots de code (équipe Sonnet)

Chaque lot livre : migrations, dépôt (`ent-db`), service en transaction unique, routes (`ent-api`) et types (`ent-types`), tests Postgres (isolation entre établissements, matrice de droits, cas du §4 concernés), écrans quand le lot en a, documentation (ce document, `ROLES_ET_DROITS.md`, `PROJECT_STATUS.md`). Revue `review-sonnet` sur chaque lot ; **revue sécurité** (skill `security-review`) sur P0, P2, P3, P4, P6, P10. Travaux parallèles en sessions ou worktrees séparés (F-007), quatre compilations au plus en même temps (F-006). Effort : **S** ≤ 3 j, **M** 1–2 sem., **L** > 2 sem. [H].

| Lot | Contenu | Dépend de | Critère de fin | Effort |
|---|---|---|---|---|
| **P0 Correctifs de sécurité** | 350, 351 ; commandes `acces::fermer/suspendre` ; **garde-fou du dernier administrateur** ; fin du mot de passe temporaire (invitation e-mail minimale sur le schéma actuel) ; attribution à début futur et historique ; annuaire sans téléphone ; dates en Europe/Paris dans `perimetre.rs` | — | un accès fermé ne permet plus de se connecter (test) ; un établissement ne peut pas perdre son dernier administrateur (test) ; aucun secret dans une réponse HTTP (test) ; test de bord de journée | M |
| **P1 Agents et engagements** | 352, 353, 361 ; PRS-01 à 13 ; ports `agent_du_compte`, `annuaire` ; actions `personnels.*` et oracle | P0 ; socle 4 A0 (`correspondances`, création de personne) | un agent par compte du personnel ; professeur-parent = une personne ; matrice de droits identique à l'existant | M |
| **P2 Invitation, accueil, casquettes** | 357 ; PRS-20 à 29 ; QR code ; accès réduit avant arrivée ; bascule de casquette ; passkey obligatoire pour la casquette personnel | P1 ; socle 1 (D-009, enrôlement existant) | invitation → identité existante rattachée après confirmation ; professeur-parent agit comme parent dans sa casquette (test sur les 4 routes famille de `service.rs` citées) ; aucune union de droits | L |
| **P3 Services et interventions** | 354, 362 ; PRS-40 à 49, 54 ; ports `intervenants`, `en_saisie`, `de_l_eleve`, `nommer_suppleant` ; indicateur « créneaux sans service » | P1 ; socle 5 (cibles) | suppléance : une saisie par jour ; titulaire de retour retrouve tout ; correction d'intervention sans perte d'auteur | L |
| **P4 Périmètre, missions, portées** | 355, 367 ; PRS-60 à 67 ; portées `mon_perimetre`, `moi`, `sites_designes` ; modèle « Professeur principal » ; explication et simulation (PRS-101, 102) | P3 | **tests D-022** (LV2 mixte, co-enseignement, suppléance, remplacement d'une séance, délai de grâce, deux PP) ; PP sans service sur la classe ; repli texte coupé sur un établissement de labo complet | L |
| **P5 Groupes et équipes** | 356, 358 ; PRS-66, 70 à 75 ; ports d'équipes | P3, P4 | équipe pédagogique calculée juste sur le jeu de labo ; aucune équipe n'ouvre de droit (test) | M |
| **P6 Délégations et signature** | 359 ; PRS-80 à 84 ; port `peut_signer` pour le socle 7 | P4 | délégation expirée ⇒ droits retirés le soir (test) ; anti-escalade du délégant (test) | M |
| **P7 Parcours d'arrivée et de départ** | 360 ; PRS-27, 28, 30 à 37 ; tâches en cours lues aux socles existants (dégradé : compteurs disponibles seulement) | P2, P3, P4 | départ guidé complet sur le labo : accès fermé à la date, services vacants, missions transférées ; décès : aucun message automatique (test) | M |
| **P8 Répartition et préparation de N+1** | PRS-50 à 53, 55 ; contrôles de couverture | P3 ; socle 5 (année en préparation, heures prévues) | reconduction N → N+1 en un geste ; ouverture à la rentrée sans effet sur les droits avant la date | L |
| **P9 Imports et exports** | PRS-110 à 115 ; CSV d'abord ; STS-web et AAF **après** spécifications (socle 9) ; GAR | P1, P3 ; socle 9 (IOP-L1) | CSV de reprise rejouable et annulable ; fichier d'exemple officiel importé sans perte quand disponible | M (CSV) + M (nationaux) |
| **P10 Revue des accès et détection** | 363 ; PRS-100, 103, 104, 106 | P2, P4 | revue complète sur le labo ; « agents partis avec accès ouvert » = 0 | M |
| **P11 Écrans** | sous-lots : P11a annuaire + fiche ; P11b arrivée guidée + rentrée (QR) ; P11c départ guidé ; P11d répartition ; P11e missions + équipes ; P11f mon espace + équipe de mon enfant + mes professeurs ; P11g revue des accès + qualité ; P11h paramétrage | lots back | rendu et parcours contre le vrai backend ; clavier, 375 px, axe-core | L (total) |
| **P12 Mobile** | enrôlement par QR, mes services, annuaire, équipe, bascule de casquette, « où est-il maintenant », équipe de mon enfant | P2, P3, P5 ; socle 11 | contrat JSON testé ; parité des droits | M |
| **P13 RLS, conservation** | 364 à 366 ; purges après décision DPO | P1–P6 ; DPO | RLS stricte testée | M |
| **P14 Retrait des projections** | 368, 369 | tous ; socles 10, 12, 15, 20, 21 lisant `agent_id` | grep des lectures à zéro | S |

**[personas] Lots ajoutés et changements** (`PERSONAS_05-06.md`) :

| Lot | Contenu | Dépend de | Critère de fin | Effort |
|---|---|---|---|---|
| **P0** (changé) | dates dans le **fuseau de l'établissement** au lieu d'Europe/Paris (R-P2) | socle 5 S2 (fuseau au profil) ou valeur par défaut | test de bord de journée avec deux fuseaux | — |
| **P1** (changé) | exclusion des engagements par (agent, employeur, nature) (R-P1) ; suspension d'engagement (R-P4) ; motifs de fin (R-P13) ; noms d'événements alignés (R-P26) | — | maître + salarié OGEC enregistrés (test) ; disponibilité sans clôture (test) | — |
| **P3** (changé) | **après Q-P28** : interventions posées sur le cours du socle 5 ; intervention dès la prise de poste (R-P3) | socle 5 S4 | une seule table du cours (test de garde) | — |
| **P4** (changé) | **après Q-S21** : aucune fonction de périmètre propre ; le socle 6 fournit les interventions à `structure::perimetre` ; R-P5 (élève + personnel) dans les tests D-022 | socle 5 S1 | l'AED étudiante ne voit aucun camarade (test) | — |
| **P15 Planning de service** | PRS-133 à 136 ; `planning.*` ; compteurs ; port « de service maintenant » pour le socle 14 | P1 ; socle 5 S3 (jours, semaines) | planning de 15 AED sur deux semaines tournantes avec exceptions et échanges ; présents à une heure donnée exacts | L |
| **P16 Arrivée enrichie** | PRS-116 à 123, 140, 142 ; trames (procès-verbal, attestation, dossier de l'arrivant) | P2, P7 ; socle 7 | arrivée d'une contractuelle à 8 h, appel à 10 h (parcours de labo) ; renouvellement sans rupture (test) | M |
| P8 (complété) | PRS-127 à 132 (vœux, propositions, publication, service dû, état de service, rémunérations des missions) | P3 | état de service signé sur le labo ; publication prévisionnelle sans droit sur les élèves (test) | +M |
| P10 (complété) | PRS-120, 125, 126 (accès temporaires et de secours) ; R-P23 | P2 | accès d'inspection clos seul après un jour (test) | +S |
| P7 (complété) | PRS-121, 137, 143, 145 à 147 | P3 | — | +M |
| P11 (complété) | sous-lots P11i planning, P11j arrivée enrichie et « mes documents » | lots back | — | +M |

PRS-144 (pouls) n'entre dans aucun lot tant que Q-P32 n'est pas tranchée. Migrations : numéros à attribuer hors de la plage 350–369, déjà pleine.

Ordre recommandé : **P0** (indépendant, immédiatement utile) → P1 → (P2, P3 en parallèle) → P4 → (P5, P6, P7) → P8 → P9, P10 → P11 au fil de l'eau → P12, P13 → P14. Parcours minimal vendable (« rentrée des personnels d'un établissement ») : P0, P1, P2, P3, P4, P7 + P11a, b, c, d, f. **Avant le socle 10** (emploi du temps) : P0 à P4 au minimum, puisque le créneau se rattache au service et l'appel à l'intervention.

---

## 19. Questions à trancher par le porteur du projet

| # | Question | Options | Recommandation |
|---|---|---|---|
| **Q-P1** | L'agent est une **personne du socle 4** avec une fiche d'agent à côté (une seule personne pour le professeur-parent) | personne + agent / identité propre au socle 6 / compte seul | **personne + agent** (§6.2) |
| **Q-P2** | Un compte à plusieurs **casquettes** : une casquette active par session, bascule en un geste | casquette par session / union de tous les droits | **casquette par session** : pas d'union accidentelle, journal clair |
| **Q-P3** | Professeur-parent : accès à son propre enfant avec ses droits de professeur | interdit / permis et tracé / permis | **permis, tracé à part, visible de la direction** ; alerte si un de ses services contient son enfant ; noter son enfant permis sauf paramétrage contraire |
| **Q-P4** | Service = unité stable ; les professeurs sont des **interventions** datées (le titulaire reste titulaire pendant une suppléance) | unité + interventions / un service par professeur (actuel) | **unité + interventions** |
| **Q-P5** | Pendant une suppléance, le titulaire garde-t-il la lecture ? | lecture / rien / tout | **lecture conservée, saisie au suppléant, notifications coupées** ; paramétrable |
| **Q-P6** | Délai de grâce après la fin d'une intervention ou d'un engagement | 0 / 7 / 15 jours | **7 jours**, lecture et saisie de ses propres évaluations de la période seulement ; paramétrable de 0 à 15 |
| **Q-P7** | Un professeur garde-t-il la lecture de ses propres saisies des années passées ? | oui / non | **oui, ses saisies seulement**, pendant la durée de conservation |
| **Q-P8** | Nouvelles portées `mon_perimetre` (périmètre porté par l'attribution), `moi`, `sites_designes` ; `mes_classes` redéfinie par les interventions | oui / non | **oui** (sans elles : un rôle par classe) |
| **Q-P9** | Les équipes ne donnent aucun droit (seuls services, missions, responsabilités, rôles) | aucun / équipes porteuses de rôles | **aucun** |
| **Q-P10** | Intervenants extérieurs : compte ? | jamais / compte invité limité / comme un personnel | **sans compte par défaut** ; compte invité sur décision de la direction, fin obligatoire ≤ 1 an, droits limités à son groupe |
| **Q-P11** | Fin d'accès : le soir de la date de fin, suspension immédiate possible, identité conservée | — | **oui** ; toute fermeture vérifie le dernier administrateur |
| **Q-P12** | Coordonnées personnelles des agents : invisibles des familles toujours, des collègues sauf choix de l'agent ; contact d'urgence facultatif | — | **oui** |
| **Q-P13** | Ce que les familles voient de l'équipe | équipe de l'enfant / tout l'annuaire | **équipe de l'enfant** + direction, vie scolaire, secrétariat ; photo désactivée par défaut |
| **Q-P14** | Nom affiché aux familles : civilité + nom par défaut ; identité restreinte pour un agent menacé | — | **oui**, posée par la direction seulement ; vérifier une éventuelle obligation d'identification des agents [NV] |
| **Q-P15** | Ne jamais collecter le NUMEN ni la date de naissance ; si un flux national les exige, les recevoir chiffrés et ne jamais les afficher | — | **oui**, après vérification des spécifications STS-web et AAF [NV] et avis du DPO. **Exception légale [V]** (C. éduc. L442-2 II ; VERIF R50) : un établissement **hors contrat** transmet chaque année à l'autorité académique les noms des personnels et les pièces attestant « leur identité, leur âge, leur nationalité et, pour les enseignants, leurs titres » ; le DPO dit si ces pièces sont tenues dans ScolIA (domaine `rh` chiffré) ou hors ScolIA |
| **Q-P16** | Congé long : accès conservé, notifications en pause par défaut | conservé / suspendu | **conservé, pause des notifications** ; l'agent peut suspendre lui-même |
| **Q-P17** | Revue des accès : obligatoire ? | trimestrielle proposée / obligatoire bloquante | **proposée chaque trimestre**, alerte si non faite, non bloquante |
| **Q-P18** | Passkey des agents sans appareil personnel : clés de sécurité fournies par l'établissement | oui / non | **oui** ; vérifier si un employeur peut imposer l'usage d'un appareil personnel [NV] ; coût à chiffrer |
| **Q-P19** | Public : le guichet Agents dispense-t-il de la passkey ? | — | dépend de la décision 1 de `INTEROPERABILITE_NATIONALE.md` §15 ; **recommandation de ce document : oui pour les connexions fédérées nationales** |
| **Q-P20** | Durées de conservation (agents partis, engagements, invitations, revues, journal des accès) | — | **à demander au DPO** ; aucune durée codée avant |
| **Q-P21** | Indicateurs d'activité par agent (appels non faits, évaluations non publiées) | désactivés / activables / actifs | **désactivés par défaut**, activables par l'établissement après information des personnels (consultation des représentants dans le privé [NV]) ; jamais de classement |
| **Q-P22** | Stocker les heures dues d'un enseignant dans l'établissement (sans grade ni échelon) | oui / non | **oui**, facultatif, pour la couverture seulement |
| **Q-P23** | Le secrétariat (`administratif`) peut-il créer des agents ? Aujourd'hui : Admin et Direction seulement | oui / non | **non par défaut** (comportement actuel conservé) ; l'établissement peut l'ouvrir par un rôle |
| **Q-P24** | Intérim de direction : l'adjoint faisant fonction reçoit-il `roles.gerer` et `securite.politique` ? | oui / non / paramétrable | **paramétrable, non par défaut** |
| **Q-P25** | Préparation de la répartition de N+1 dans ce socle (heures prévues portées par le socle 5) | 6 / 5 / 10 | **6**, à confirmer avec la conception du socle 5 (§5.4) |
| **Q-P26** | Vérifications réglementaires avant les lots P3, P4, P7 : natures d'engagement et employeurs, PP (nombre, terminale), obligations de service, AED et AESH (durées de contrat), maîtres du privé sous contrat (statut, guichet Agents, STS-web), personnels des collectivités, référents obligatoires, délégation de signature du chef d'établissement, droit à la déconnexion, information et consultation des personnels sur les traitements les concernant | — | **lecture des textes** par une session dédiée (code de l'éducation, statuts particuliers, circulaires), sources consignées dans ce document |
| **Q-P27** | Préfixe des actions métier `PRS-` (le socle 4 a repris `ADM-`, déjà utilisé par D15 dans `SOCLE2_DOMAINES.md`) | — | **`PRS-`** ; signaler la collision `ADM-` au socle 4 |
| **Q-P28** [personas] | **Service d'enseignement (ici) et cours (socle 5) : un seul objet** (même question que Q-S20) | une table, création par le socle 5, interventions ici / deux tables | **une table** ; le socle 6 garde interventions, heures dues, missions ; PRS-40 appelle la création du cours (STR-57). **Bloquant avant P3** |

> **[S4, code, 2026-11-09]** Le socle 5 a livré l'objet « cours » (`ent_cours`, migration 341, `05_STRUCTURE.md` §24). **Tranché en code : deux objets, un seul cours** — le cours (ce qui est enseigné à qui) est au socle 5 ; `ent_service_enseignement` reste la table des services (qui l'enseigne) jusqu'au socle 6, et pointe vers son cours par `cours_id`. Contrat pour le lot P3 : poser les services / interventions **sur** un cours (`ent_db::structure_cours::cours::poser_service`, qui recopie la cible du cours), lire `cours::{lire, services_du_cours, vacants, public}` ; un cours sans service est un poste vacant ; ne pas recalculer le périmètre (Q-S21 : `ent_public_cours_jour`). La reprise de M7 est faite (n° 354 de ce document : plus à faire pour les cours).
| **Q-P29** [personas] | **Engagements simultanés** (maître + salarié de l'OGEC, AED + vacataire) | exclusion par agent / par (agent, employeur, nature) | **par (agent, employeur, nature)** (R-P1). **Bloquant avant P1** |
| Q-P30 [personas] | **Planning de service** des personnels non enseignants : socle 6 (lot P15) ou socle 15 ? | 6 / 15 / nouveau socle | **socle 6** (c'est l'organisation du travail, comme le service d'enseignement) ; le socle 15 y lit les postes à remplacer |
| Q-P31 [personas] | **Accès de secours local** (PRS-120) pour un agent de plusieurs établissements, pendant la récupération D-018 | non / oui, borné | **oui, borné** (48 h, cet établissement, alertes) ; à valider au regard de D-018 et de la revue de sécurité |
| Q-P32 [personas] | **Pouls d'humeur** anonyme (trou T-08) | l'écarter / le concevoir désactivé / l'activer | **le concevoir désactivé**, et ne le coder qu'à la demande d'un pilote ; risque de perception de surveillance |
| Q-P33 [personas] | **Service des enseignants** : jusqu'où aller (composantes du service dû, heures supplémentaires année, état de service, natures de rémunération des missions) sans faire la paie ? [NV] | minimal / complet sans montants | **complet sans montants** (R-P11) ; vérifier les textes avant P8 (Q-P26) |
| Q-P34 [personas] | **Accès d'inspection** (PRS-125) : l'accord de l'enseignant est-il requis, ou seulement son information ? [NV] | accord / information | **information préalable** ; vérifier le cadre de l'inspection |
| Q-P35 [personas] | **Vérification d'honorabilité** (PRS-143) : quelles personnes, quelles obligations dans chaque secteur ? [NV ; au 2026-11-09, aucun texte en vigueur propre aux écoles trouvé, une proposition de loi est en navette (VERIF R51) : ne pas présenter le module comme une obligation ; données d'infractions (RGPD art. 10) : avis du juriste et du DPO] | — | **vérifier** avant P7 ; ne jamais stocker le contenu d'une vérification |

---

## 20. Récapitulatif : faits, hypothèses, contradictions

**Faits vérifiés [F]** : identité unique `ent_user` et rattachement `ent_membership (user_id, etablissement_id, role)` sans état ni date (`005:4-20`), aucune autre migration ne la modifie, aucune suppression dans le code ; validité de session par simple jointure sur `ent_membership` (`auth_repo.rs:820-833`) ; révocation de toutes les sessions d'un compte, tous établissements (`auth_repo.rs:1139`) ; provisionnement du personnel réservé aux profils Admin et Direction, avec mot de passe temporaire renvoyé dans la réponse (`service.rs:3175-3290`, `routes.rs:1073-1101`), silencieux si l'adresse existe ailleurs ; droits famille dépendant du profil de session (`service.rs:4394`, 76 occurrences de `Role::Parent`) ; annuaire du personnel avec e-mail et téléphone ouvert à tout le personnel (`routes.rs:755-761, 1035-1053`) ; catalogue de 47 actions et 4 portées, sans action propre aux personnels (`actions.rs:13-230`), portée `moi` absente ; modèles par défaut codés (`actions.rs:381-540`) ; attributions de rôle à clé `(role_id, user_id)`, début forcé à aujourd'hui, écrasement par `ON CONFLICT` (`098:32-42`, `droits_repo.rs:474-478`) ; anti-escalade avec exemption du profil Admin (`droits_repo.rs:633-659`) ; garde-fou du dernier administrateur absent ; responsables de groupes sans dates, remplacés en bloc (`098:48-57`, `droits_repo.rs:580-589`) ; service d'enseignement à clé `prof_user_id`, sans dates ni heures, PP booléen unique par classe (`092:87-116`) ; PP mis à jour dans `referentiel_structure.rs:681-691` ; seule route de service : `POST /referentiel/services` ; périmètre du professeur par `enseigne_a`, repli classe texte et remplacement ±14 jours, en UTC (`perimetre.rs`) ; absences des personnels, créneaux, séances et remplaçants rattachés aux comptes (`080`, `021`, `110`) ; aucune table de personnel, d'équipe, de référent, de quotité, aucun import STS-web dans le code. Next.js [F Next] : table `personnel` distincte du compte, tables RH filles, 17 rôles d'intranet, affectation professeur ↔ classe par clé texte, nom d'enseignant dupliqué en texte, contextes multiples par compte.

**Non relu ligne à ligne** : écrans Leptos `modules/roles` et `referentiel_admin` ; `securite_repo.rs` (cité depuis le relevé) ; tests R1 / R2.

**Hypothèses [H] / non vérifié [NV]** : natures d'engagement et employeurs par type d'établissement ; statut des maîtres du privé sous contrat, leur présence dans STS-web et leur accès au guichet Agents ; agents des collectivités ; deux PP en terminale ; obligations de service ; durées de contrat des AED et AESH ; organisation des AESH en pôles ; référents (harcèlement, vie lycéenne…) ; DDFPT ; association sportive ; délégation de signature du chef d'établissement ; droit à la déconnexion ; consultation des représentants du personnel ; obligation d'identification des agents ; usage imposé d'appareils personnels ; formats STS-web (bascule OMOGEN), AAF, GAR ; protocole du guichet Agents ; ce que vivent les acteurs et la comparaison avec les logiciels du marché ; efforts des lots.

**Contradictions signalées** :
- `ROLES_ET_DROITS.md` annonce une portée `moi` et le garde-fou « administrateur indélébile » : ni l'une ni l'autre n'existe dans le code (Q-P8, lot P0).
- `ROLES_ET_DROITS.md` dit « attribution datée (début, fin facultative) » : le début est forcé à aujourd'hui (`droits_repo.rs:476`).
- `ROLES_ET_DROITS.md` parle de « modèles fournis par ScolIA, copiables » : ce sont des profils calculés en Rust, aucun rôle modèle n'est inséré (`ent_role.modele` est informatif).
- D-022 (professeur limité à son groupe) est contredit par le repli « classe texte » et la fenêtre de remplacement de `perimetre.rs` (§9.2, lot P4).
- `SOCLE2_DOMAINES.md` ADM-05 (« désactiver un compte : sessions révoquées », état P) : il n'existe pas de désactivation, seulement la révocation des sessions, qui n'empêche pas de se reconnecter.
- `SOCLE2_DOMAINES.md` REF-13 (« un PP par classe ») : figé dans un index unique, alors que le besoin est paramétrable (cas P17).
- Préfixes d'actions métier : `ADM-` désigne D15 dans `SOCLE2_DOMAINES.md` et le dossier administratif dans `04_ADMINISTRATIF.md` ; `PER-` désigne le périscolaire ; ce document utilise `PRS-` (Q-P27).
- La conception du socle 5 n'était pas consultable : les points §5.4 sont à accorder avant les lots P3 et P8.
- **[personas]** Lue depuis (`05_STRUCTURE.md`), elle répond à §5.4 : groupes créés avant tout service (point 1, oui) ; heures prévues portées par la maquette du socle 5 (point 2, oui) ; année en préparation (point 3, oui) ; ensemble scolaire = un établissement à plusieurs sites et UAI (point 5) ; matières du socle 5 (point 6) ; sites désignés (point 7). Mais elle **possède aussi** « ce qui est enseigné à qui » (`ent_cours`) et la **seule** fonction de périmètre (`structure::perimetre`) : contradiction avec §5.3, §8.5 et §9 de ce document (Q-P28, Q-S21) ; et ses noms d'événements diffèrent (R-P26).
- **[personas]** La contrainte d'exclusion des engagements (§8.2) contredit les doubles employeurs réels (R-P1) ; le cas P10 parle d'« engagement suspendu » sans modèle (R-P4) ; « Europe/Paris » (§7, lot P0) contredit le fuseau du profil du socle 5 (R-P2).


---

## 21. État du lot P0 (2026-11-08, branche `ent-socle-personnels-p0`)

**Niveau : implémenté + testé en local ; non validé CI GitHub ni Florian.** Migrations 350 et 351.

| Exigence du lot | Réalisation | Test |
|---|---|---|
| `acces::fermer` / `suspendre` | `ent-db/src/acces.rs` (fermer, suspendre, rouvrir, programmer une fin) ; routes `POST /etablissement/personnel/{id}/acces/{fermer,suspendre,rouvrir}`, `PUT …/acces/fin` ; Admin/Direction, jamais soi-même, Direction ne touche pas Admin/Direction ; personnels seulement ; sessions et appareils push **de cet établissement** supprimés ; journalisé | `acces_ferme_refuse_connexion_session_et_jeton_…`, `acces_ferme_ou_suspendu_http_…` |
| Connexion/session/jeton refusés | `ent_acces_ouvert` (350) lu à la connexion, à `issue_session` (tous chemins : mot de passe, passkey, SSO) et à chaque requête (`resolve_bearer`) ; réponse de connexion identique à un mauvais mot de passe | idem |
| Dernier administrateur | contrôle service (message clair) + déclencheur SQL `ent_membership_garde_dernier_admin` (fermeture, suspension, date de fin, changement de profil, suppression ; verrou consultatif contre deux retraits simultanés) | `dernier_administrateur_protege_…`, `dernier_administrateur_409_…` |
| Fin du mot de passe temporaire | provisionnement : compte à secret inconnu + invitation par e-mail (lien 7 jours) ; réinitialisation du personnel : lien (30 min) ; `EnvoiImpossible` avant toute création si messagerie absente ; plus aucun secret dans les réponses | `provisionnement_et_reinitialisation_sans_secret_…`, `l3_reinitialisation_par_lien_…` |
| Attribution à début futur et historique | `attribuer_role_periode_for_actor` ; ligne identifiée, jamais écrasée ; retrait = clôture ; modification de fin = clôture + nouvelle ligne ; exclusion de chevauchement | `attributions_datees_…` |
| Annuaire sans téléphone | `StaffMember` sans téléphone ; accès non ouverts exclus | `annuaire_du_personnel_sans_telephone…` |
| Jour de l'établissement | `PgEnt::aujourdhui_etablissement` (fuseau 265, Europe/Paris par défaut) dans `perimetre.rs` et `droits_repo.rs` ; logique D-022 bis inchangée | `bord_de_journee_deux_fuseaux_…`, `perimetre_prof_suit_eleve_…` |

**Écarts à connaître (à valider par Florian).**
1. Invitation d'un **compte élève** : renvoie encore un mot de passe temporaire (socles 2-3, élèves sans boîte mail probable) — non traité.
2. Colonne `perimetre` de l'attribution (§17, 351) non créée : sens non défini.
3. Supprimer un rôle libre supprime ses attributions (historique perdu) ; archivage à prévoir.
4. `delegation.rs` et `edt_*` calculent encore la date en UTC.
5. Provisionnement d'une adresse connue ailleurs : réponse identique, mais l'envoi du mail peut s'entendre au temps de réponse.
6. Un accès à date de fin n'est pas un « administrateur durable » : un établissement garde toujours au moins un administrateur sans fin.
7. Les listes de destinataires (notifications, etc.) lisant `ent_membership` ne filtrent pas encore l'état : à reprendre au lot P1/P2.

**Revue de sécurité du lot P0 (skill `security-review`, auto-revue Sonnet, 2026-11-08 ; pas une certification).** Examiné : connexion mot de passe / passkey / SSO, ouverture et lecture de session, jetons mobiles (mêmes sessions), commandes d'accès, provisionnement et réinitialisation, attributions de rôle, annuaire, jour d'établissement. Constats : (1) aucun chemin d'ouverture de session ne contourne `ent_acces_ouvert` (`issue_session` le revérifie) ; (2) la lecture de session relit l'état à chaque requête, donc une fermeture agit sans attendre la révocation ; (3) réponse de connexion identique pour accès suspendu et mauvais mot de passe (test) ; (4) cloisonnement : cible cherchée par (établissement, personne), refus uniforme 403, révocation limitée aux sessions de l'établissement (test avec second établissement) ; (5) aucun secret dans les réponses ni le journal (tests) ; (6) déclencheur du dernier administrateur sérialisé par verrou consultatif (test de course). Résidus acceptés : le réglage SQL `ent.purge_etablissement` désactive le garde-fou (jamais posé par l'application, accessible seulement à qui a déjà l'accès SQL) ; lectures `ent_membership` des listes de destinataires sans filtre d'état (point 7 ci-dessus) ; appareils push d'un accès échu par date non purgés (accès refusé de toute façon). Vérifications : tests ent-db du lot (6), ent-api du lot (4), `l3_*`, `api_*` (21), suite `separation` (42), gardes statiques d'isolation ; clippy sans erreur, sans avertissement nouveau sur les fichiers du lot.

## 22. État du lot P1 « agents et engagements » (2026-11-08, branche `ent-socle-personnels-p1`, base `ent-socle-personnels-p0`)

**Niveau : implémenté + testé en local ; non validé par la CI GitHub ni par Florian.** Migrations **352** (`352_agents.sql`), **353** (`353_agents_fonctions.sql`), **361** (`361_agent_identifiants.sql`) ; `MIGRATION_TAG` reste `370_edt_creneau_groupe` (plus haute du répertoire).

| Exigence du lot | Réalisation | Test |
|---|---|---|
| Un agent par compte du personnel | déclencheur `ent_membership_agent_sync` (tout accès de profil non parent/élève reçoit personne + agent + engagement `a_preciser` + fonction principale déduite du profil, quel que soit le chemin de création) ; reprise idempotente dans 352/353 | `un_agent_par_compte_du_personnel_avec_fonction_principale`, `reprise_de_l_existant_…` |
| Professeur-parent = une seule personne (socle 4) | la personne d'un compte est réutilisée (reprise et déclencheur) ; PRS-01 accepte une personne existante (responsable, contact) ; la recherche de correspondances est faite avant toute création | `professeur_parent_une_seule_personne`, `creation_cherche_les_correspondances_…` |
| Maître sous contrat + salarié OGEC | exclusion de chevauchement par (agent, employeur, organisme, nature) (R-P1) ; alerte de qualité `meme_employeur_simultane` | `maitre_sous_contrat_et_salarie_ogec_…` |
| Disponibilité / congé sans clôture | `ent_agent_engagement_suspension` (R-P4) ; accès coupé par `ent_agent_acces_suspendu` (tous les engagements courants suspendus avec accès coupé) aux quatre points de contrôle (`ent_acces_ouvert`, connexion, session, appartenance) ; levée sans nouvelle invitation | `disponibilite_ou_conge_…`, `suspension_d_un_engagement_le_maitre_garde_l_acces_…`, HTTP |
| Ports | `personnels::agent_du_compte` (`PgEnt::agent_du_compte`, `agent_du_compte_tx`) ; `personnels::annuaire` (`annuaire_tx`, `annuaire_for_actor`) à trois niveaux de lecteur (interne, famille, élève) | `annuaire_parite_avec_l_existant_et_vue_des_familles` |
| Actions `personnels.*`, moteur et oracle | `personnels.annuaire`, `.lire`, `.personnel.lire`, `.confidentiel.gerer`, `.gerer`, `.acces.gerer` ; oracle `Role::can_gerer_personnels` ; `provisionner_personnel` et `cible_compte_autorisee` lisent maintenant l'action (hiérarchie conservée) | `p1_modele_identique_aux_capacites_historiques` (ent-auth), `matrice_de_droits_serveur_et_roles_parametres` |
| Cloisonnement, RLS, journal | clés étrangères composées, `TenantTx` pour toute commande, politique RLS sur les 9 tables, rôle non propriétaire testé ; journal et événements sans nom, sans coordonnée, sans motif | `isolation_entre_etablissements_…_rls`, `journal_et_evenements_sans_donnee_personnelle` |

**Actions PRS couvertes.** PRS-01 (création, avec correspondances d'abord), 02 (identité, `personnels.gerer` **ou** `personnes.gerer`), 03 et 04 (ajout, modification, prolongation, fin avec motif codé R-P13, correction par annulation), 05 (fonctions, catalogue fourni + fonctions propres à l'établissement), 06 (disciplines), 07 (qualifications, codes R-P14), 08 (contact d'urgence, agent seul), 09 (visibilité de la photo et consentement daté), 10 (pause des notifications), 11 (identité restreinte), 12 (personnel mineur), 13 (fusion d'agents), PRS-147 (suspension d'engagement, R-P4). Identifiants externes (361) : ajout, clôture, recherche d'un agent par identifiant.

**Écarts avec la conception, à valider par Florian.**
1. **Contact d'urgence** : porté en clair sur la ligne de l'agent (nom, lien, téléphone), **pas** comme `ent_personne`. Une personne du socle 4 serait apparue dans la recherche de personnes du secrétariat, contraire à « visible de la direction seule ». Le téléphone n'est pas chiffré (le mécanisme `champs_chiffres` n'est pas branché) ; lecture par un tiers journalisée.
2. **`modeles_roles_proposes UUID[]`** remplacé par `profil_propose` (profil de base) : les modèles de rôles propres à un établissement n'existent qu'au lot P4.
3. **Photo (PRS-09)** : consentement daté et visibilité (collègues / élèves / familles) seulement ; le dépôt du fichier attend le socle 7. Aucune clé étrangère vers un document.
4. **`ent_agent.etat` n'est pas stocké** : calculé (`ent_agent_etat(établissement, agent, jour)` : `prevu`, `actif`, `suspendu`, `parti`) ; `archive` est réservé au lot P13. Un accès fermé avec engagement courant se lit `parti`.
5. **`employeur_organisme_id`** sans clé étrangère : `ent_organisme` (socle 4) n'existe pas encore.
6. **Fusion d'agents (PRS-13)** : la fusion de personnes du socle 4 refuse désormais deux agents (« fusionner les agents ») ; un seul agent côté source suit la personne conservée et **revient à la défusion**. La défusion d'une fusion **d'agents** est refusée côté personnes (« liée à une fusion d'agents ») et **n'est pas livrée** au lot P1 (`agent_origine_id` garde la trace des engagements déplacés).
7. **Reprise** : le téléphone du compte n'est **pas** copié sur la personne créée (minimisation §13.4) ; l'e-mail du compte l'est, en minuscules, sans usage `pro`/`perso` (qui n'existe pas encore au socle 4) : l'annuaire interne rend donc l'e-mail du compte, comme `/personnel/search`. L'engagement repris commence le jour de création de l'accès (jour de l'établissement) ; un accès fermé donne une fin à la date de fermeture, un accès à fin datée donne cette fin (jamais avant le début).
8. **Secrétariat** : n'a PAS `personnels.gerer` par défaut (Q-P23 : comportement actuel conservé) ; il a `personnels.lire` et corrige l'identité par `personnes.gerer`. Un rôle paramétré peut lui ouvrir la gestion (testé).
9. **Familles/élèves** : l'annuaire de leur niveau existe comme port (`NiveauAnnuaire::Famille` / `Eleve`, identité restreinte masquée, personnel mineur exclu, jamais d'e-mail ni de compte) ; aucune route ne l'expose encore (lot P11f, contrôle du lien actif).
10. **Pas encore faits (lots suivants)** : fermeture automatique à la date de fin d'engagement (tâche quotidienne, P2/P7), réouverture sous 30 jours (R-P9), invitation (P2), casquette parent (P2) : un professeur-parent est une seule personne, mais la bascule de casquette n'est pas livrée. Les listes de destinataires qui lisent `ent_membership` sans filtre d'état (point 7 du lot P0) ne sont pas reprises ici.

**Points de vigilance.** (a) Les personnes du personnel apparaissent désormais dans la recherche de correspondances du socle 4 (c'est voulu : « est-ce M. Martin, père de Léa ? »), mais une déclaration de famille dont le nom ou l'e-mail ressemble à un agent demande maintenant un rapprochement. (b) Le déclencheur est silencieux sur le profil `parent`/`eleve`. (c) `ent_acces_ouvert` et les quatre requêtes d'accès appellent `ent_agent_acces_suspendu` : une jointure supplémentaire par requête authentifiée (index `ent_personne_compte_uq`).

**Vérifications locales du lot P1 (2026-11-08).** `ent-auth` 42/42 ; `ent-db` (suite complète, `labo`) 417/420 après correction du décompte RLS de `tests_rls` (9 tables du lot) ; `ent-api` (suite complète, `labo`) lib 105/105, `separation` 42/42 après ajout de `personnels.rs` à l'inventaire des routes, `referentiel`, `fichiers_*`, `span_requete`, `tenant_hote` verts ; clippy workspace sans avertissement dans les fichiers du lot, avec et sans `labo`. **Trois échecs d'`ent-db` hors lot, identiques sur la base `ent-socle-personnels-p0` (vérifié dans un worktree)** : `tests_eleve_identite::reprise_de_l_existant_idempotente` (date de naissance de Harry Potter non reportée sur la version d'identité du labo), `tests_evaluations::isolation_entre_etablissements` et `tests_finances::droits_et_isolation` (événement forgé inter-établissements refusé par la clé étrangère `ent_metier_event_eleve_etab_fkey`) : tests à rendre conformes par leurs lots (E2/E3, S2-7/S2-9).


## 23. État du lot P2 « invitation, accueil, casquettes » (2026-11-08, branche `ent-socle-personnels-p2`, base `ent-socle-personnels-p1`)

**Niveau : implémenté + testé en local ; non validé par la CI GitHub ni par Florian ; aucun écran (§18).** Migration **357** (`357_casquettes_invitations.sql`) ; `MIGRATION_TAG` reste `370_edt_creneau_groupe` (plus haute du répertoire).

| Exigence du lot | Réalisation | Test |
|---|---|---|
| Invitation à usage unique, durée courte, jeton haché, jamais renvoyé (PRS-20, 21) | `ent_agent_invitation` : empreinte SHA-256 seule (le jeton porte l'identifiant de l'établissement, 96 caractères hexadécimaux) ; e-mail 7 jours, QR 10 minutes ; une seule invitation en attente par agent (index unique partiel) ; réponse d'émission par e-mail sans jeton ni adresse ; **le lien du QR n'est rendu que dans la réponse d'émission ou de relance** | `invitation_nouvelle_identite_passkey_obligatoire_et_aucun_secret`, `invitation_matrice_de_droits_unicite_et_jeton_jamais_stocke` |
| Expirée, annulée, relancée, déjà utilisée : refusée | une seule réponse 400 pour jeton inconnu, expiré, annulé, utilisé ou de forme invalide (aucun oracle) ; relancer = annuler + émettre | `invitation_expiree_annulee_relancee_ou_utilisee_refusee_et_qr` |
| Identité existante rattachée après confirmation (PRS-24) | session du **bon compte** + `confirmer_rattachement: true` ; sans connexion 401, autre compte 401, sans confirmation 409, rien n'est écrit ; deux fiches « personne » pour un compte : 409 « fusionner d'abord » (socle 4), jamais de fusion silencieuse | `identite_existante_rattachee_…`, `deux_fiches_pour_un_compte_exigent_la_fusion_…` |
| Professeur-parent : une casquette à la fois, aucune union (PRS-29) | `ent_session.casquette` (`principale` / `parent`) ; `resolve_bearer` donne à la session le rôle de la casquette ; casquette parent fermée = session invalide ; bascule journalisée (`casquette_changee`, événement `session.casquette_changee`) ; en casquette parent, mêmes statuts et mêmes enfants qu'un parent pur sur `/famille/enfants`, `/eleves/{id}/dossier`, `/dossier/eleves/{id}/fiche|responsables` ; en casquette professeur, ces routes ne s'ouvrent pas par le lien de parenté ; justification d'absence refusée à un professeur | idem |
| Passkey obligatoire pour la casquette personnel | `passkey_requise` posée à l'acceptation ; `passkey_obligatoire_compte` (politique OU casquette) aux trois chemins d'ouverture de session et dans `/auth/me` ; session d'enrôlement jusqu'à la première passkey, bascule refusée avant | `invitation_nouvelle_identite_…`, `identite_existante_…` |
| Accès réduit avant la date d'arrivée (PRS-25) | `ent_agent_avant_arrivee` ; portée `arrivee` **calculée à chaque requête** (jamais stockée) : routes ouvertes = identité, passkeys, casquettes, annuaire des collègues, `/personnels/moi/*` ; toute donnée d'élève refusée (`arrivee_pas_commencee`) ; ouverture complète le matin de la date, sans reconnexion | `acces_reduit_avant_la_date_d_arrivee_puis_ouverture_complete` |
| Arrivée du jour (PRS-26) | `signaler_arrivees_du_jour(etab)` idempotent (événement `personnel.arrive`, une fois par invitation) ; **non branché sur un ordonnanceur** | idem |
| Aucun secret dans une réponse HTTP | jeton, adresse, mot de passe absents des réponses, du journal et des événements ; aucune session ouverte par l'acceptation | `invitation_nouvelle_identite_…` |
| Cloisonnement, RLS, journal | `TenantTx` partout ; RLS sur `ent_membership_casquette` et `ent_agent_invitation` (rôle non propriétaire testé) ; 404 uniforme hors établissement ; jeton d'un autre établissement falsifié : 400 | `invitations_cloisonnees_par_etablissement_et_lot`, `rls_invitations_et_casquettes_isolees_entre_etablissements` |

**Actions PRS couvertes.** PRS-20, 21 (invitation e-mail et QR), 22 (lot : aperçu sans écriture, adresses manquantes listées, un refus n'arrête pas le lot), 23 (liste, annulation, relance), 24 (acceptation, nouvelle identité ou rattachement), 25 (`GET /personnels/moi/arrivee`, sans donnée d'élève), 26 (fonction idempotente), 29 (`GET /auth/casquettes`, `POST /auth/casquette`). **Non faits ici :** PRS-27 et 28 (parcours et arrivée guidée : lot P7, migration 360). Action de droit ajoutée : `personnels.inviter` (admin, direction, administratif).

**Hiérarchie des profils invitables.** `admin` et `direction` : par un administrateur ; `administratif`, `compta`, `infirmerie`, `psy` : par qui détient `personnels.gerer` ; `prof`, `cpe`, `accueil` : par tout détenteur de `personnels.inviter` (secrétariat compris). Personne ne s'invite soi-même.

**Écarts avec la conception, à valider par Florian.**
1. **Le lien du QR code figure dans la réponse d'émission** (et de relance) : c'est le contenu du QR, affiché une fois à l'écran authentifié du secrétariat. Seule exception à « jamais renvoyé » ; aucune lecture ultérieure ne le rend. Les écrans (P11b) ne doivent ni le journaliser ni le mettre en cache.
2. **L'acceptation n'ouvre aucune session** : la personne se connecte ensuite (mot de passe choisi, puis enrôlement de la passkey). Plus simple à sécuriser qu'une session émise sur un lien ; coûte une connexion.
3. **Passkey obligatoire par invitation, pas par politique** : les accès du personnel existants restent soumis à la politique de l'établissement (aucun test historique cassé) ; seule la casquette ouverte par une invitation porte `passkey_requise`. Généraliser à tous les comptes du personnel est une décision D-009 à prendre.
4. **Choix de la casquette à la connexion non livré** : la session démarre toujours sur la casquette principale (le profil de l'accès) ; un professeur-parent bascule ensuite (`POST /auth/casquette`). La bascule vers `principale` ne redemande pas la passkey : une session complète sur un accès du personnel l'a déjà passée.
5. **Casquette parent créée à l'acceptation si le compte était parent ou a un lien actif, ou à la première bascule** pour un compte du personnel déjà responsable d'un élève ; jamais pour `eleve` ni `intervenant` (non livrés). Pas de casquette élève (cas P48) dans ce lot.
6. **Accès réduit** : la portée `arrivee` n'ouvre pas encore l'emploi du temps, le cahier ni la progression des services (ils dépendent du lot P3) ; seul le contenu « mes dates et mes collègues » existe.
7. **Sessions de l'accès révoquées à l'acceptation** : un parent devenu professeur doit se reconnecter (son profil change).
8. **Reste dû au lot P7/P10** : relance automatique, tableau « qui a activé son accès », fermeture automatique à la date de fin, réouverture sous 30 jours (R-P9), invitation d'un agent dont l'accès est fermé.

**Revue de sécurité du lot P2 (skill `security-review`, auto-revue Sonnet, 2026-11-08 ; pas une certification).** Examiné : émission, relance, annulation, lot ; acceptation publique ; rattachement d'identité ; résolution du rôle de session par casquette ; accès réduit ; passkey obligatoire ; journal et événements ; cloisonnement. Constats :
- **Corrigé pendant la revue (gravité moyenne)** : `GET /personnels/moi/arrivee` ne contrôlait pas la casquette ; la session « parent » d'un professeur-parent aurait lu l'annuaire des collègues (union de droits). Il exige maintenant la casquette principale et un profil du personnel (test : refus en casquette parent et pour un parent pur).
- Droits effectifs : `droits_effectifs` sort tôt pour un rôle non personnel, donc les rôles libres et délégations du professeur ne suivent pas la session en casquette parent. Les lectures de `ent_membership.role` par identifiant de compte inspectées (rôles libres, cibles de délégation, remplaçants, référentiel) portent sur des **cibles**, pas sur l'acteur.
- Rattachement : seul le compte désigné par la fiche (ou l'adresse mémorisée) peut accepter ; le détenteur du jeton seul ne peut pas annexer un compte existant ; deux fiches pour un compte : conflit.
- Jeton : 256 bits aléatoires, empreinte en base, forme validée avant toute lecture, réponse uniforme ; l'aperçu révèle « une identité existe » au seul détenteur du jeton.
- **Résidus acceptés / à suivre.** (a) Invitation par QR sans adresse mémorisée : quiconque détient le QR (10 minutes, devant le secrétariat) choisit l'adresse du nouveau compte ; protégé par la présence physique et la durée. (b) Les routes `/personnels/moi/*` de P1 (contact d'urgence, photo, pause) ne contrôlent pas la casquette : elles ne touchent que les données de l'agent lui-même ; à aligner au lot P4. (c) `ent_agent_avant_arrivee` et `ent_casquette_ouverte` sont lues par le pool sans variable d'établissement : en mode RLS strict la casquette parent se ferme (sûr) mais l'accès réduit s'ouvre (comme `ent_agent_acces_suspendu` en P1) ; à traiter avec le lot P13. (d) Aucune limite de débit propre aux routes publiques d'invitation (jeton de 256 bits : force brute sans objet) ; le plafond général s'applique. (e) La tâche d'arrivée n'est pas planifiée.

**Vérifications locales du lot P2 (2026-11-08, base `ent_ci_test`).** Avec la feature `labo` : tests du lot `ent-api` 6/6 et `ent-db` 2/2 ; `ent-api` lib 111/111 ; `ent-db` `tests_rls` 9/9 (décompte des tables à RLS porté à +2), `tests_personnels_p0` 6/6, `tests_personnels_p1` 16/16, `tests_l6` 8/8, `tests_socle1` 5/5, `tests_roles` 9/9 ; `ent-auth` 42/42 (matrice de `personnels.inviter`). Sans la feature : `ent-auth` 42, `ent-events` 2, `ent-types` 12, `ent-db` migrations 1/1, `ent-api` lib 25/25. Clippy workspace avec et sans `labo` : aucun avertissement sur les fichiers du lot (restent les avertissements existants de `auth_repo.rs` : `set_password` inutilisé, type complexe ; de `actions.rs` test d'oracle ; d'`inscriptions_reactions.rs`). Non relancés : la suite complète d'`ent-db` (trop longue pour le délai de la session, lancée par modules) et les trois échecs d'`ent-db` hors lot connus du §22. Formatage : `rustfmt` sur les trois fichiers créés seulement.
