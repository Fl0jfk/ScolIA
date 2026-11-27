# Passe « personas » sur le socle 27 — Conformité RGPD, conservation et archives

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; recommandations appliquées par défaut (D-031), sauf ce qui engage l'établissement ou l'éditeur à l'extérieur (§7.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un service de production.
**Branche :** `ent-socle-27-28-conception`, partie de `origin/ent-mc0hoe`.
**Éprouve :** `docs/socles/27_RGPD_ARCHIVES.md` (lu en entier), avec `CLAUDE.md`, D-031, D-034, D-035, D-036, D-037 à D-046, `PRA_SAUVEGARDES.md` §11 et §18.4, `PORTABILITE_EXPORTS.md`, `registre_donnees.rs`, `MISE_EN_SERVICE_DONNEES.md`, `VERIF_REGLEMENTAIRE.md` ; forme reprise de `PERSONAS_42.md` et `PERSONAS_10.md`.
**Ajouts :** faits **directement** dans `27_RGPD_ARCHIVES.md` au **§26** (« ajout personas »), qui l'emporte en cas de contradiction ; rien n'est supprimé.
**Numérotation :** manques **M-2700 à M-2735** ; actions **RGP-300 à RGP-319** ; règles **R-RG41 à R-RG50** ; cas limites **CL27-63 à CL27-76** ; questions **Q-RG12 à Q-RG14** ; réglage **S-554**.

---

## 0. À lire en premier : cinq bloquants et leur recommandation (appliquée par défaut, D-031)

La conception tient : un référentiel commun, des socles qui exécutent, un registre généré, des preuves. Seize personas la mettent à l'épreuve ; cinq points bloqueraient une mise en service réelle.

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut) | Lot |
|---|---|---|---|---|
| **A** | **Le journal en ajout seul empêche d'anonymiser ce qu'il contient** [F : `091_journal_ajout_seul.sql` refuse tout `UPDATE`/`DELETE`, y compris au rôle applicatif ; `ent_journal_modif.payload` garde des instantanés avant/après qui contiennent noms, adresses, motifs]. CL27-37 promet que le journal est « anonymisé avec l'objet » : c'est impossible tel quel. | Un dossier détruit ou anonymisé **survit en clair** dans le journal pendant des décennies : c'est exactement ce qu'un contrôle relèverait (P02). | **Procédure dédiée** : fonction `SECURITY DEFINER` appartenant au propriétaire du schéma, appelable **seulement** par l'exécuteur de conservation, sous un drapeau de transaction `ent.conservation_lot = <id du lot>` vérifié par le déclencheur 091 (qui autorise alors **un seul** `UPDATE` : remplacement de `payload` et `libelle` par une version neutre, et `eleve_id` mis à `NULL`) ; chaque ligne traitée est comptée dans `cons_execution` ; tout autre `UPDATE` reste refusé. Même principe pour les traces d'accès des sujets anonymisés (R-RG41). | G8 (migration 2555) |
| **B** | **Usurpation par le droit d'accès** : une demande arrive par courriel « je suis la mère de Léa, envoyez-moi son dossier ». La conception vérifie l'identité, mais ne dit pas **où** l'extrait est envoyé. | Un attaquant (ex-conjoint, P04 ; inconnu, P11) obtient tout le dossier en donnant **sa** propre adresse. | **Remise uniquement** à un canal déjà connu du dossier (compte de la personne, adresse électronique vérifiée au socle 4, ou remise en main propre contre pièce), **jamais** à une adresse ou un numéro fournis dans la demande (R-RG42) ; une adresse nouvelle passe par la vérification de coordonnées du socle 4 avant toute remise. | G3 |
| **C** | **L'année de doublon crée un traitement que personne n'a déclaré** : ScolIA recopie les données de Charlemagne pour évaluer le logiciel (D-034). La finalité « évaluer un nouvel outil » n'est pas celle de la scolarité, et si le doublon échoue, que deviennent les copies ? | Registre faux ; AIPD incomplète ; données gardées sans fin si la bascule de 2028 n'a pas lieu (P01, P02). | Traitement **« essai en parallèle »** au registre (finalité, base, durée **jusqu'à la décision de bascule + 3 mois**), catégorie `doublon.copie` ; **procédure de fin d'essai** : si la bascule est abandonnée, purge de toutes les données de l'établissement selon la fin de contrat (§8.10), attestation ; si elle a lieu, les données deviennent celles du traitement normal à la date de bascule, avec la mention (R-RG43). | G2, G10 |
| **D** | **Des données de santé sont déjà collectées alors que le module santé est désactivé** [F : `registre_donnees.rs` classe `ent_absence_declaration` en `.sante()` et `fichiers(Reference)` : les justificatifs d'absence, dont des certificats médicaux, sont déposés au socle 12]. `MISE_EN_SERVICE_DONNEES.md` A7 interdit les données de santé sans hébergeur HDS. | Contradiction directe avec A7 ; un certificat médical stocké sans HDS (P12). | Tant que HDS est faux : **le certificat médical n'est pas conservé** — la vie scolaire coche « justificatif médical présenté le … » (motif « santé » sans détail) et rend ou détruit la pièce ; le dépôt en ligne d'un justificatif refuse la catégorie « certificat médical » et explique pourquoi (R-RG44) ; **signalé au socle 12** (correctif à porter dans son lot AS0) et au juriste (B9). | G0 (signalement), socle 12 |
| **E** | **Le masquage ne couvre que les lignes, pas les pièces ; et le journal peut révéler une protection** : une facture PDF de la mère, un certificat de scolarité portant son adresse, ou la trace « Secrétariat — consultation des **données protégées** » vue par le père, révèlent ce qu'on cache. | Fuite d'une adresse protégée par le canal même qui sert à protéger les droits (P04). | (1) Une **pièce** qui contient des données d'un tiers n'est **pas caviardée automatiquement** : elle est **exclue** de l'extrait et **listée** (« 2 documents existent qui concernent aussi une autre personne ; ils ne vous sont pas remis ; motif ») ; caviardage manuel possible, relu (R-RG45). (2) Les traces sur des sections protégées (F4, interdictions) apparaissent au journal des familles sous un **code de section générique** (« coordonnées ») et **jamais** au parent visé par la protection (R-RG46). | G3, G4 |

---

## 1. Méthode
Seize personas réalistes, dans l'établissement pilote (lycée privé sous contrat, OGEC) et dans un collège public (EPLE), y compris un **attaquant** (deux profils) et deux **auditeurs** (DPO externe ; contrôleur de la CNIL). Pour chacune : sa situation, ce qu'elle fait avec le 27, ses irritants et peurs, ce qui la ravirait, les **manques** (M-27xx), les **contradictions** et les **cas limites**. Chaque manque est classé : **B** bloquant, **I** important, **C** confort. Les vérifications de code citées ont été refaites sur `ent-mc0hoe` le 2026-11-11.

## 2. Les personas en un tableau

| N° | Persona | Contexte | Ce qu'elle éprouve |
|---|---|---|---|
| P01 | **Mme Duval**, cheffe d'établissement | lycée pilote (privé sous contrat) | responsabilité de traitement, doublon, AIPD, signature |
| P02 | **M. Blanc**, DPO externe de l'OGEC (auditeur RGPD) | 6 établissements | registre, durées, preuves, journal, sauvegardes |
| P03 | **Mme Martin**, secrétaire de direction | lycée pilote | demandes papier, fratries, délais, modèles |
| P04 | **M. Garnier**, père séparé en conflit (attaquant interne à la famille) | lycée pilote | droit d'accès détourné, adresse protégée de la mère |
| P05 | **Léa**, 15 ans, seconde | lycée pilote | ses données, ses droits, ses parents |
| P06 | **Hugo**, 24 ans, ancien élève (sorti en 2028) | — | bulletins, effacement, sans compte |
| P07 | **M. Roux**, archiviste départemental | collège public | bordereaux, visa partiel, versement |
| P08 | **Mme Lefèvre**, professeure remplaçante | lycée pilote | motifs hors périmètre, journal visible |
| P09 | **M. Benali**, CPE | lycée pilote | consultations en rafale, alertes |
| P10 | **Florian**, éditeur (sous-traitant et responsable de la couche compte) | plateforme | violation de plateforme à 2 h du matin |
| P11 | **Les attaquants** : **Yanis**, AED curieux ; **« Mme X »**, inconnue qui se dit mère | lycée pilote | consultation abusive, export, usurpation par courriel |
| P12 | **Mme Petit**, infirmière | lycée pilote | justificatifs médicaux, module santé désactivé |
| P13 | **Mme Lambert**, comptable de l'OGEC | lycée pilote | factures, mandats, effacement refusé |
| P14 | **M. Faure**, contrôleur de la CNIL | contrôle sur place | preuves en 15 minutes |
| P15 | **M. Arnaud**, principal d'un collège public | EPLE | CADA, journaliste, archives publiques |
| P16 | **Mme Diallo**, AESH partie en juin | lycée pilote | ses données de personnel, ses traces |

---

## 3. Parcours et manques, persona par persona

### P01 — Mme Duval, cheffe d'établissement
**Situation.** Septembre 2027 : ScolIA en doublon de Charlemagne, modules de l'add-on en usage réel. Elle doit signer l'AIPD et le registre avant les premières données réelles (porte `donnees_reelles`, socle 28).
**Irritants, peurs.** Signer un registre de 40 pages qu'elle ne comprend pas ; découvrir que l'OGEC pensait être responsable ; « et si on arrête ScolIA en juin 2028 ? ».
**Ce qui la ravirait.** Une page « en une phrase » par traitement ; une liste « ce que je dois décider » ; savoir ce qui se passe si l'essai échoue.
**Manques.**
- **M-2700 (B)** : traitement « essai en parallèle » absent → bloquant **C**.
- **M-2701 (I)** : le registre généré n'a pas de **résumé en une phrase** par traitement : ajouter une phrase générée et relue (« Nous tenons le dossier de chaque élève pour l'inscrire, le suivre et délivrer ses documents ; il est conservé 50 ans… ») (RGP-300).
- **M-2702 (I)** : la répartition établissement / OGEC (Q-RG1) n'a pas d'écran : chaque traitement a un **responsable** désigné, et la fiche exporte la répartition comme annexe à signer (RGP-301).
**Cas limite.** CL27-63 : la cheffe change en cours d'année — les validations restent valables (elles engagent l'établissement), mais le registre propose au nouveau chef une **relecture** (point ouvert).

### P02 — M. Blanc, DPO externe (auditeur RGPD)
**Situation.** Il suit six établissements de l'OGEC ; il audite le 27 avant d'émettre son avis.
**Ce qu'il vérifie et ce qu'il trouve.**
1. **Registre généré** : bon point, les destinataires sont les rôles réels. Mais il demande la **preuve** que les durées sont appliquées : la conservation intermédiaire et les lots le montrent, sauf pour le **journal** → **bloquant A** (M-2703, B).
2. **Sauvegardes** : 35 jours + 12 mensuelles (D-043) ; une donnée effacée sur demande vit encore douze mois dans des sauvegardes chiffrées. Il l'accepte **si** c'est écrit dans les mentions et si la restauration rejoue les effacements (R-RG22) : vérifié.
3. **Traces d'accès** : 12 mois, partitionnées : bien. Il demande que les **traces du support** de l'éditeur soient conservées aussi longtemps que les autres et soient **dans** le journal de l'établissement, pas seulement chez l'éditeur : déjà prévu (socle 28, R-MSE16).
4. **Pseudonymisation pour le pilotage et l'IA (RGP-294)** : il rappelle qu'un jeu pseudonymisé **reste** une donnée personnelle (CNIL, §4.10) : le registre doit le déclarer comme traitement (M-2704, I).
5. **Tableau consolidé** de ses six établissements : il le veut, sans nominatif : prévu (§12.2) ; il demande d'y voir aussi les **demandes en retard** et les **violations ouvertes** : prévu (RGP-238).
6. **Comptes inactifs** : S-551 (12 mois) contredit le SDET 2024, qui demande la suppression **trois mois** après la fin du compte (§4.10) → **M-2705 (I)** : S-551 passe à 3 mois après la fin du dernier rattachement, avec un avis préalable et l'offre de télécharger le coffre.
**Contradiction relevée.** CL27-37 (« journal anonymisé avec l'objet ») et 091 (ajout seul) : tranchée par **A**.

### P03 — Mme Martin, secrétaire de direction
**Situation.** Lundi de rentrée de novembre : trois courriers recommandés arrivés pendant les vacances, dont un père qui demande « toutes les données de mes trois enfants ».
**Irritants.** Trois demandes à saisir pour une seule lettre ; le délai a couru pendant les vacances ; elle ne sait pas quoi masquer.
**Manques.**
- **M-2706 (I)** : une demande ne porte qu'**un** sujet (`rgp_demande.sujet_id`) : il faut une demande à **plusieurs sujets** (fratrie) → table `rgp_demande_sujet` (RGP-302), un extrait par sujet, une réponse commune.
- **M-2707 (I)** : la date de réception d'un courrier est la date de **dépôt dans la boîte de l'établissement**, pas celle de l'ouverture : champ « reçu le » distinct de « saisi le » (RGP-303) ; alerte si l'écart dépasse 5 jours.
- **M-2708 (C)** : **modèles de réponse** par cas fréquent (accès simple, effacement refusé, demande d'un parent séparé, demande d'un élève mineur, CADA) : liste fournie, éditable (RGP-304).
**Cas limite.** CL27-64 : demande reçue le 20 juillet, établissement fermé jusqu'au 25 août : le délai court ; **un accusé automatique** est impossible pour un courrier ; la procédure écrite (G0) prévoit une **relève du courrier** pendant l'été ou l'information du DPO.

### P04 — M. Garnier, père séparé en conflit (attaquant interne à la famille)
**Situation.** Ordonnance de protection : l'adresse de la mère est protégée (socle 4). Il fait une demande d'accès « complète » pour ses deux enfants, puis consulte le journal des accès.
**Ce qu'il tente.** Obtenir l'adresse de la mère par (1) les factures de la cantine dont elle est payeuse, (2) le certificat de scolarité qui porte l'adresse de résidence, (3) les métadonnées d'un PDF, (4) le journal (« quelqu'un a consulté les données protégées » prouve qu'elles existent), (5) les échanges de la messagerie entre la mère et le CPE.
**Ce qui le bloque déjà.** R-RG38 et le masquage relu (R-RG15).
**Manques.**
- **M-2709 (B)** : pièces et journal → **bloquant E**.
- **M-2710 (I)** : les **documents émis** (certificat de scolarité, attestation) sont regénérés pour l'extrait **depuis les données**, avec les champs du tiers masqués, plutôt que remis tels quels (RGP-305) ; les métadonnées des fichiers remis sont nettoyées (auteur, chemin, coordonnées) (R-RG47).
- **M-2711 (I)** : la **messagerie** : seuls les messages dont il est **auteur ou destinataire** entrent dans son extrait (CL27-18 précisé) ; les fils de la mère avec l'établissement n'en font jamais partie.
**Cas limite.** CL27-65 : il dépose une demande **au nom de l'enfant** de 16 ans (« c'est elle qui demande ») depuis son propre compte → la qualité « élève » exige le compte de l'élève ou sa présence ; sinon la demande est celle du père.

### P05 — Léa, 15 ans
**Situation.** Elle veut savoir ce que le lycée sait d'elle, faire retirer une photo publiée, et ne veut pas que ses parents le sachent.
**Ce qui marche.** Page « Tes données » ; demande depuis son compte ; photo retirée sur retrait du consentement (socle 18) — mais le consentement à l'image est donné par **les parents** (VERIF R15) : elle ne peut pas le retirer seule [J].
**Manques.**
- **M-2712 (I)** : l'extrait remis à un **élève demandeur** ne contient pas les données propres de ses parents (situation financière, payeurs, coordonnées détaillées), seulement leur identité de responsables (R-RG48).
- **M-2713 (I)** : demande de retrait d'une photo par l'élève mineure : reçue, transmise aux titulaires pour décision **et** retirée provisoirement des publications futures en attendant (limitation, RGP-160) — l'avis de l'enfant compte (C. civ. 372-1, VERIF R15) (RGP-306).
- **M-2714 (C)** : le texte de « Tes données » existe en deux niveaux (collège, lycée) ; ajouter une **version audio** courte pour les élèves en difficulté de lecture (accessibilité) [R].
**Q-RG3** reste au DPO (information des parents).

### P06 — Hugo, 24 ans, ancien élève
**Situation.** Sorti en 2028, son compte a été anonymisé (S-551). Il veut ses bulletins et demande l'effacement de « tout ».
**Ce qui marche.** Formulaire sans compte (RGP-142) ; vérification par pièce ; bulletins en conservation intermédiaire délivrés par le socle 23.
**Manques.**
- **M-2715 (I)** : la vérification d'identité d'un ancien élève sans compte s'appuie sur des données qu'il connaît et que le dossier garde (date de naissance, classe, année de sortie) **en plus** de la pièce : grille fournie (RGP-307).
- **M-2716 (C)** : réponse à « effacez tout » : un tableau clair « ce qui est effacé / ce qui est gardé, jusqu'à quand, pourquoi » généré depuis le référentiel (déjà RGP-157, à rendre lisible).
**Cas limite.** CL27-66 : Hugo demande ses données d'un établissement qui a **quitté ScolIA** : ScolIA n'en a plus (attestation de suppression) ; il est renvoyé vers l'établissement.

### P07 — M. Roux, archiviste départemental (collège public)
**Situation.** Le collège lui envoie son premier bordereau d'élimination généré par ScolIA.
**Ce qu'il exige.** Service versant, références de la ligne du tableau de tri (instruction de 2005), **dates extrêmes**, **volume** (nombre d'objets et taille pour du numérique), mode de destruction, nom et qualité du signataire ; il vise **une partie seulement** (il garde les dossiers des élèves nés une année donnée pour échantillon).
**Manques.**
- **M-2717 (I)** : champs du bordereau complétés (RGP-308) : référence de la ligne de l'instruction de tri (portée par la catégorie, `cons_categorie.source_ligne`), dates extrêmes, volume, mode de destruction.
- **M-2718 (B pour le public)** : **visa partiel** : l'archiviste retire des objets → `cons_visa_exclusion` (objets exclus par le visa, motif), les objets exclus passent au **versement** ou restent en intermédiaire (RGP-309).
- **M-2719 (I)** : pour le versement numérique, il veut savoir **ce que ScolIA détruit encore après** (copies de sauvegarde, index) : l'attestation de versement le mentionne.

### P08 — Mme Lefèvre, professeure remplaçante
**Situation.** Elle remplace un collègue trois semaines (socle 15) et ouvre les fiches des élèves de ses classes temporaires.
**Irritant.** On lui demande un motif à chaque fiche, alors qu'elle fait son travail ; peur que les familles la croient indiscrète.
**Manques.**
- **M-2720 (I)** : le **périmètre** pris en compte pour « hors périmètre » doit inclure les périmètres **temporaires** déjà connus par les autres socles : remplacement (15), fenêtre du conseil de classe (21, D-032), sortie encadrée (42), permanence (12) (R-RG49, RGP-310).
- **M-2721 (C)** : le journal des familles montre « Professeur — remplacement » plutôt que « Professeur, hors de ses classes ».

### P09 — M. Benali, CPE
**Situation.** Une bagarre à la sortie : il ouvre 25 fiches en dix minutes.
**Constat.** Son périmètre est l'établissement (rôle CPE) : rien n'est « hors périmètre » ; l'alerte S-543 ne se déclenche pas. Bon. Mais un **AED** dans la même situation déclenche l'alerte.
**Manques.**
- **M-2722 (C)** : seuils d'alerte **par rôle** (S-554) ; l'alerte se ferme d'un clic par la direction avec « justifié : incident du … » (déjà RGP-196, préciser).

### P10 — Florian, éditeur, violation de plateforme à 2 h du matin
**Situation.** Un défaut de cloisonnement a exposé, pendant 40 minutes, des noms d'élèves de trois établissements à des comptes d'un quatrième.
**Ce qui marche.** RGP-254 : déclaration une fois, propagation à chaque établissement avec ses éléments.
**Manques.**
- **M-2723 (B)** : la couche **compte** (D-013) est sous la responsabilité de ScolIA : si la violation touche des comptes (adresses de connexion), **ScolIA notifie lui-même la CNIL** pour cette partie, en plus d'informer les établissements (R-RG50, RGP-311) ; cela suppose Q-RG6 tranchée.
- **M-2724 (I)** : modèle de **message de sous-traitant** aux établissements (33.2) avec les éléments de l'art. 33.3 déjà remplis par la plateforme (RGP-312).
- **M-2725 (I)** : **liste des personnes concernées par établissement** calculée par l'extraction (fermeture des références, PRA §11.3), jamais transmise d'un établissement à l'autre.

### P11 — Les attaquants
**Yanis, AED curieux.** Il veut voir la fiche d'un élève d'une autre classe (le petit frère d'un ami). (1) Motif « autre » + texte : la trace est visible de la famille ; (2) il tente un export : refusé (pas le droit) ; (3) il demande à un collègue de lui créer un rôle : la création de rôle ne permet pas d'octroyer `droits.traiter` ou `acces.consulter_traces` sans passkey récente d'un détenteur (★) : vérifié dans la conception.
**« Mme X »**, inconnue, écrit par courriel : « je suis la mère de Léa, envoyez-moi son dossier, voici ma nouvelle adresse ». → **bloquant B** (M-2726, B).
**Manques supplémentaires.**
- **M-2727 (I)** : une demande dont la **qualité** est incertaine (pas de lien au socle 4) n'est jamais traitée comme celle d'un titulaire ; la réponse dit seulement « nous ne pouvons pas donner suite sans justificatif », sans confirmer que l'enfant est scolarisé ici (**ne pas révéler la scolarisation** à un inconnu, R-RG42 précisée).
- **M-2728 (I)** : limite de fréquence des demandes par adresse et par sujet (anti-harcèlement), sans refus automatique (R-RG17).
**Cas limite.** CL27-67 : un compte de parent volé fait une demande d'accès : remise sur le compte → l'attaquant l'obtient. Parade : la remise d'un extrait exige une **passkey récente** du demandeur (ou le guichet) — ajoutée à RGP-154 (R-RG42).

### P12 — Mme Petit, infirmière
**Situation.** Le module santé est désactivé (A7) ; elle garde son outil. Mais la vie scolaire dépose dans ScolIA des **certificats médicaux** d'absence.
**Manque.** **M-2729 (B)** → **bloquant D**.
**Cas limite.** CL27-68 : un parent écrit dans le motif libre d'absence « angine, sous antibiotiques » : texte de santé dans un champ libre ; repérage proposé (§10.1) et **suggestion** de remplacer par « raison médicale » (jamais de modification silencieuse).

### P13 — Mme Lambert, comptable de l'OGEC
**Situation.** Une famille partie demande l'effacement de ses coordonnées bancaires et de ses factures.
**Ce qui marche.** Refus motivé (L123-22, 10 ans) pour les factures ; mandat SEPA révoqué ; IBAN : la durée `fin.iban` est [NV].
**Manques.**
- **M-2730 (I)** : distinguer **IBAN d'un mandat actif**, **IBAN d'un mandat révoqué** (preuve en cas de contestation d'un prélèvement : la durée de contestation d'un prélèvement non autorisé est de 13 mois [NV, à vérifier par l'expert-comptable]) et **IBAN sans mandat** (à détruire) : trois catégories au lieu d'une (RGP-313).
- **M-2731 (C)** : une facture **contestée** gèle automatiquement (CL27-22) ; la levée du gel est faite par la comptabilité, avec motif.

### P14 — M. Faure, contrôleur de la CNIL (contrôle sur place)
**Situation.** Contrôle inopiné au lycée pilote ; il demande en 15 minutes : registre, AIPD, procédures, registre des violations, liste des habilitations et leur revue, preuve d'une destruction, preuve d'une réponse à une demande, journal des accès d'un dossier tiré au hasard, contrats de sous-traitance.
**Ce qui marche.** Tout existe, dispersé.
**Manques.**
- **M-2732 (I)** : **dossier de contrôle** en un clic : archive datée (registre, AIPD, mentions, procédures, sous-traitants et accords, revue des habilitations, liste des lots et certificats, liste anonymisée des demandes et délais, registre des violations), sans données d'élèves ; les traces d'un dossier ne sont produites que sur sa demande explicite et sont tracées (RGP-314).
- **M-2733 (I)** : la **revue des habilitations** (A12) doit produire une **liste datée** des personnes et de leurs actions sensibles, signée par la direction (RGP-315).

### P15 — M. Arnaud, principal d'un collège public
**Situation.** Un journaliste demande « la liste des sanctions de l'année » au titre de la CADA ; une mère demande le dossier de son fils au titre du RGPD.
**Manques.**
- **M-2734 (I)** : qualification CADA **par un tiers** : documents communicables après **occultation** des mentions nominatives (CRPA L311-6, §4.10) ; ScolIA produit un document **agrégé** ou occulté, jamais une extraction nominative (RGP-316) ; délai d'un mois, refus implicite au-delà (affiché).
- **M-2735 (C)** : dans le public, la qualification par défaut d'une demande de « dossier » par un parent est **RGPD et CADA à la fois** : la réponse cite les deux fondements (modèle).

### P16 — Mme Diallo, AESH partie en juin
**Situation.** Elle demande ses données et « qui a consulté mon dossier ».
**Ce qui marche.** §11.4 : son dossier RH et ses consultations ; ses **propres** actions (traces de ce qu'elle a consulté) lui sont communicables, **sans** les données des élèves consultés.
**Cas limite.** CL27-69 : son affectation à un élève (AESH, `ent_aesh_affectation` classée **santé** au registre [F]) : la donnée concerne l'élève (besoin d'accompagnement) autant qu'elle ; son extrait dit « affectée à l'accompagnement d'un élève de 2ᵈᵉ » sans nom.

### Manques transverses : mobile, IA, tests
- Mobile : la déclaration d'un incident en un geste est prévue ; ajouter la **réception** d'une demande de droit au guichet sur tablette (photo de la pièce, détruite à la clôture) (CL27-70).
- IA : le repérage de textes de santé (P12) et de tiers (P04) doit être évalué sur un **jeu fictif** avant activation (taux de rappel mesuré) ; jamais de masquage automatique.
- Tests : ajouter les cas des bloquants A à E au §22 du 27.

---

## 4. Vérification réglementaire complémentaire (2026-11-11)
Voir `27_RGPD_ARCHIVES.md` §4.10 (sous-agent, sources officielles) : confirme les délais (12.3), l'inapplicabilité de la portabilité à la mission d'intérêt public (20.3), le contenu du registre (30) et de la notification (33.3), les délais de communicabilité (L213-2), le **SDET 2024** (journaux 12 mois au plus, suppression des comptes 3 mois après leur fin, cahier de textes 7 ans), l'**instruction de tri de 2005** (le dossier scolaire « ne doit pas être remis à l'élève » : l'extrait est une copie), la modification de **R511-13** par le décret 2026-637 (à utiliser par le socle 13), et l'**agrément** exigé pour un dépositaire d'archives publiques (L212-4). Non trouvé : délai de contestation d'un prélèvement SEPA (M-2730), exercice des droits par le mineur seul (Q-RG3).

---

## 5. Manques qui relèvent d'autres socles
| Manque | Socle | Action |
|---|---|---|
| M-2729 certificats médicaux sans HDS | 12 (et 34) | correctif AS0 : pas de conservation du certificat tant que HDS faux |
| Périmètres temporaires (M-2720) | 15, 21, 42, 12 | exposer un port « périmètre temporaire » lu par le 27 |
| Version en vigueur de R511-13 (décret 2026-637) | 13 | relire les paliers dans `13_DISCIPLINE.md` |
| Consentement à l'image et avis de l'enfant (M-2713) | 18 | retrait provisoire sur demande de l'élève |
| Vérification d'une adresse nouvelle avant remise (bloquant B) | 4 | réutiliser `lien_confirmation_voie` (268) |
| Traces du support dans le journal de l'établissement | 28 | déjà R-MSE16 |
| Responsabilité de la couche compte (M-2723) | 1, 9 | Q-RG6 |

---

## 6. Synthèse

### 6.1 Manques par gravité
- **Bloquants (5 points, 7 manques)** : M-2700, M-2703, M-2709, M-2718 (public), M-2723, M-2726, M-2729.
- **Importants (21)** : M-2701, M-2702, M-2704, M-2705, M-2706, M-2707, M-2710, M-2711, M-2712, M-2713, M-2715, M-2717, M-2719, M-2720, M-2724, M-2725, M-2727, M-2728, M-2730, M-2732, M-2733, M-2734.
- **Confort (8)** : M-2708, M-2714, M-2716, M-2721, M-2722, M-2731, M-2735, et le texte audio.

### 6.2 Ajouts dans la conception (`27_RGPD_ARCHIVES.md` §26)
Actions RGP-300 à RGP-316 ; règles R-RG41 à R-RG50 ; cas limites CL27-63 à CL27-70 ; tables `rgp_demande_sujet`, `cons_visa_exclusion`, colonne `cons_categorie.source_ligne`, distinction des catégories IBAN ; migrations 2555 (procédure d'anonymisation du journal), 2556 (sujets multiples, visa partiel), 2557 (réserve) ; S-551 ramené à 3 mois ; S-554.

### 6.3 Les dix ajouts les plus importants
1. Anonymisation du journal par procédure dédiée (A).
2. Remise de l'extrait seulement sur un canal déjà connu, passkey récente (B).
3. Traitement « essai en parallèle » et fin d'essai (C).
4. Pas de certificat médical conservé sans HDS (D).
5. Pièces de tiers exclues et listées ; journal qui ne révèle pas une protection (E).
6. Demandes à plusieurs sujets (fratries).
7. Visa partiel et bordereau complet.
8. Périmètres temporaires pour ne pas demander de motif à tort.
9. Dossier de contrôle en un clic, revue des habilitations datée.
10. Couche compte : ScolIA notifie pour sa part.

### 6.4 Ce que la passe a montré sur la conception
La conception pense bien les **données** et moins les **canaux** : c'est par la remise, les pièces jointes et le journal que les fuites passeraient. Elle supposait aussi que le journal en ajout seul se plierait aux sorts finaux : il faut une procédure explicite. Enfin, le doublon de 2027-2028 est un traitement à part entière.

---

## 7. Décisions

### 7.1 Appliquées par défaut (D-031)
Les cinq bloquants A–E et leurs recommandations ; M-2705 (comptes : 3 mois, aligné sur le SDET) ; M-2706 (sujets multiples) ; M-2718 (visa partiel) ; M-2720 (périmètres temporaires) ; M-2730 (trois catégories d'IBAN, durée de la deuxième à confirmer) ; M-2732 (dossier de contrôle) ; M-2734 (CADA par un tiers : agrégé ou occulté).

### 7.2 Soumis à Florian
| N° | Question | Recommandation |
|---|---|---|
| Q-RG12 | Le traitement « essai en parallèle » (C) : à déclarer par l'établissement dans son registre et son AIPD **avant** les premières données réelles ; accord de la direction et avis du DPO | oui ; durée : jusqu'à la décision de bascule + 3 mois |
| Q-RG13 | Certificats médicaux (D) : faire valider par le juriste et le DPO qu'un justificatif d'absence médical **non conservé** (simple mention) suffit, en attendant l'HDS | oui |
| Q-RG14 | Notification CNIL par ScolIA pour la couche compte (M-2723) : ScolIA a-t-il un référent pour cela (cf. Q-RG6, Q-RG8) ? | Florian au départ |

### 7.3 Réglage signalé
S-554 : seuils d'alerte de consultations par rôle (défaut : AED 15 dossiers hors périmètre en 10 min ; personnels de direction et CPE : sans objet).

---

## 8. Faits, hypothèses, contradictions
**Faits [F]** : `091_journal_ajout_seul.sql` refuse `UPDATE`/`DELETE` ; `ent_journal_modif.payload` est un instantané JSONB ; `registre_donnees.rs` classe `ent_absence_declaration` et `ent_aesh_affectation` en santé. **Hypothèses [H]** : comportement des personas ; délai de contestation SEPA. **Contradictions** : CL27-37 et 091 (tranchée par A) ; A7 et les justificatifs médicaux (tranchée par D, à porter au socle 12) ; S-551 (12 mois) et le SDET (3 mois) (corrigée).
