# Passe « personas » sur le socle 36 — Suivi social, psychologique et protection de l'enfance

**Date :** 2026-11-11 — **Statut :** proposition, non décidée ; recommandations appliquées par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§6.2). Ajouts versés au **§30 de `36_SUIVI_SOCIAL.md`**, qui l'emporte.
**Branche :** `ent-socle-35-36-conception`. Aucun code écrit ; lecture ciblée de Next.js (`app/lib/bien-etre-*.ts`, `app/api/bien-etre/*`) en lecture seule.
**Personas fictifs** ; aucune situation réelle. Les situations de danger décrites sont des cas d'école.

---

## 0. À lire en premier : cinq bloquants et leur recommandation

1. **Une préoccupation qui n'est prise par personne** est pire que pas d'outil : l'adulte croit avoir transmis. **R-SOC35** (escalade automatique au délégué puis au chef, S-2728) et **SOC-305** (l'émetteur sait qui a pris). *Appliqué.*
2. **Le chef d'établissement peut être empêché ou mis en cause** : la cellule doit pouvoir transmettre sans lui (CSOC-02, S-2725). *Appliqué.*
3. **La nuit, l'internat, les vacances** : le circuit ne s'arrête pas (SOC-301, SOC-311, CSOC-39, CSOC-48). *Appliqué.*
4. **L'agent conversationnel Next.js** : la passe confirme qu'il ne faut pas le reprendre ; **soumis à Florian** (Q-SOC1, Q-SOC2), avec l'urgence de le restreindre en production.
5. **La preuve numérique** (capture d'un message) finit aujourd'hui dans la galerie du téléphone personnel d'un professeur : **R-SOC34** (capture dans l'appli, chiffrée). *Appliqué.*

---

## 1. Méthode
Parcours d'un an ou d'une crise, confrontés aux §10, §14, §16 de `36_SUIVI_SOCIAL.md` ; manques M-xxx → actions SOC-300 et suivantes, règles R-SOC33 et suivantes, cas CSOC-39 et suivants ; deux attaquants et un auditeur RGPD.

## 2. Les personas en un tableau
| # | Persona | Situation |
|---|---|---|
| P01 | Mme Leroy, psychologue de l'EN (EDO), trois établissements | agenda éclaté ; un élève suivi en crise |
| P02 | M. Benali, assistant de service social, deux collèges | fonds social, absentéisme, deux IP par an |
| P03 | Mme Duval, cheffe d'établissement | valide les IP ; préside la commission ; elle-même mise en cause une fois (cas d'école) |
| P04 | Mme Caron, CPE | membre de la cellule protection et du GPDS |
| P05 | M. Haddad, professeur de français | une élève lui écrit une révélation dans une copie |
| P06 | Yanis, AED et surveillant de nuit à l'internat | révélation à 2 h du matin |
| P07 | Mme Vidal, infirmière | traces de coups lors d'un passage |
| P08 | Chloé, 14 ans | harcelée, n'ose pas en parler à ses parents |
| P09 | Hugo, 18 ans | demande de l'aide au fonds social sans ses parents |
| P10 | Famille Martin | demande une aide pour le voyage scolaire ; craint d'être reconnue |
| P11 | M. Martin, père visé par une IP (cas d'école) | se présente à l'accueil |
| P12 | Mme Roux, gestionnaire | applique les remises |
| P13 | M. Blanc, DPO (auditeur RGPD) | AIPD, durées, droits |
| P14 | Attaquant 1 : un membre du personnel mis en cause | veut savoir qui l'a signalé |
| P15 | Attaquant 2 : parent qui usurpe le compte de son enfant | veut lire les demandes de l'enfant |
| P16 | Florian, avec le module Next.js « bien-être » | arbitrer la bascule |

---

## 3. Parcours et manques

### P01 — Mme Leroy, psychologue
**Parcours** : lundi collège A, mardi lycée B (autre organisation cliente), jeudi collège C ; demandes d'élèves, bilans pour une orientation, ESS (35), conseil en orientation (37).
**Ce qui marche** : demandes sans motif (SOC-150, SOC-153), billet neutre (SOC-155), communications dans l'ENT (SOC-174), cabinet en N-2.
**Manques** : M-01 **agenda consolidé** sur ses trois établissements, sans nom d'élève d'un autre établissement dans la casquette active (SOC-313, CSOC-10) ; M-02 **créneaux bloqués** sans élève (SOC-314) ; M-03 **rendez-vous rapide** sur permanence (SOC-312) ; M-04 demandes jamais confirmées par l'élève (SOC-316) ; M-05 le **remplaçant** qui sort un élève de cours n'a pas de compte : billet imprimé (SOC-315) ; M-06 bureau partagé : « lieu confidentiel » (CSOC-44) ; M-07 elle reprend les **signalements Next.js** (SOC-329, SOC-330).
**Contradiction** : l'existant lui donne `dossier.lire@etablissement` ; la conception réduit (R-SOC32). Elle a pourtant besoin, pour un bilan d'orientation, des **résultats** d'un élève suivi : lecture des résultats **de l'élève suivi** seulement, sur demande tracée — ajouté à R-SOC32 (portée « élèves suivis »).

### P02 — M. Benali, assistant de service social
**Manques** : M-08 **repas d'urgence** pour un élève qui n'a pas mangé depuis la veille : aide en nature immédiate, sans facture ni libellé (SOC-317, R-SOC37) ; M-09 commission sans **quorum** (CSOC-45) ; M-10 aide votée, famille partie (CSOC-46) ; M-11 **PV anonyme** pour le CA et la comptabilité (SOC-319) ; M-12 **consommation par nature** (SOC-320).
**Point** : un élève placé à l'**ASE** : le secrétariat doit savoir **qui appeler** (référent ASE, famille d'accueil) sans savoir pourquoi (SOC-309, SOC-310, R-SOC36).

### P03 — Mme Duval, cheffe d'établissement
**Parcours** : vendredi 16 h 50, la CPE lui apporte une préoccupation (révélation de violences) ; elle valide l'IP par passkey sur son téléphone, décide de ne pas informer les parents (intérêt contraire), transmet à la CRIP par le courriel sécurisé du département.
**Manques** : M-13 un **guide** pour l'entretien avec les titulaires quand ils sont informés (SOC-307) ; M-14 la **copie remise** aux titulaires (SOC-308) ; M-15 la CRIP appelle pour des compléments (CSOC-41, SOC-142).
**Cas d'école** : elle est mise en cause par une famille ; la cellule transmet **sans elle** (S-2725), ses casquettes au 36 sont suspendues (SOC-250), le journal de ce dossier lui est invisible (CSOC-02).

### P04 — Mme Caron, CPE
**Manques** : M-16 savoir qu'**un suivi existe** pour un élève absentéiste, sans savoir lequel (R-SOC13, avec l'accord de l'élève ou en cellule de veille) — confirmé ; M-17 **revue des situations** de veille anciennes (SOC-322) ; M-18 **bilan annuel** du GPDS pour le CA (SOC-321).
**Contradiction** : `12_ASSIDUITE.md` RA-21 renvoie l'information préoccupante au « socle 3 » (coquille, à lire « 36 ») : **signalé**.

### P05 — M. Haddad, professeur
**Parcours** : une élève écrit dans une rédaction qu'elle est frappée.
**Manques** : M-19 **comment réagir** : guide « écouter, ne pas questionner, noter les mots exacts, ne rien promettre » (SOC-300, R-SOC33) ; M-20 photographier la copie : **dans l'appli**, chiffrée, jamais dans la galerie (SOC-302, R-SOC34) ; M-21 après l'envoi, il veut savoir **qui a pris** et ne plus porter seul (SOC-305) ; il ne saura pas la suite (CSOC-43) ; M-22 un temps d'échange pour lui (SOC-306).

### P06 — Yanis, surveillant de nuit
**Manques** : M-23 tablette de l'internat, révélation à 2 h : **15 / 17** si danger, **astreinte** de direction, préoccupation enregistrée, IP le matin (SOC-301, CSOC-39) ; M-24 réseau faible : **brouillon chiffré** hors ligne effacé à l'envoi ou après 24 h (SOC-303, S-2729).

### P07 — Mme Vidal, infirmière
**Ce qui marche** : pli du 34 vers le 36 (SOC-260), le passage reste au 34.
**Manque** : M-25 elle veut que l'**IP reprenne** les constats faits au passage sans recopier le dossier de soins : l'IP contient le texte qu'**elle** rédige ; aucun champ ne tire le passage (confirmé, R-SOC10).

### P08 — Chloé, 14 ans, harcelée
**Parcours** : elle n'ose pas le dire ; elle ouvre « Parler à quelqu'un », choisit l'infirmière, lit ce qui restera confidentiel, envoie sans motif ; elle reçoit un rendez-vous ; son professeur voit « sortie autorisée ».
**Manques** : M-26 elle veut **retirer** sa demande avant qu'elle soit lue (SOC-328) ; M-27 voir **l'historique** de ses demandes (SOC-327) ; M-28 pendant les **vacances** (SOC-311, CSOC-48) ; M-29 le harcèlement lui-même est traité au **13** (pHARe) ; le 36 n'en garde rien sauf si un danger appelle une IP — confirmé.
**Cas limite** : sa mère consulte son téléphone : notifications **sans contenu** (« un rendez-vous t'est proposé ») ; aucune notification aux parents (R-SOC6).

### P09 — Hugo, 18 ans
**Ce qui marche** : demande d'aide en son nom (SOC-193), aucune information aux parents (§11.5).
**Manque** : M-30 son foyer fiscal reste celui des parents : instruction sur **sa** déclaration de situation, l'ASS juge [H].

### P10 — Famille Martin
**Manques** : M-31 la commission comprend un parent d'élève qui les connaît : **déclaration de lien** et retrait (SOC-318, CSOC-15) ; M-32 sur la facture du voyage, l'organisateur ne doit voir qu'une **participation réduite** (SOC-202, confirmé).

### P11 — M. Martin, père visé par une IP (cas d'école)
**Parcours** : il se présente à l'accueil, exige son fils, demande « ce qui se passe ».
**Ce qui marche** : rien n'est confirmé (CSOC-04) ; remise selon le 4.
**Manques** : M-33 s'il n'existe **aucune** interdiction, il peut repartir avec l'enfant : la cellule est **informée** immédiatement par l'accueil (CSOC-40) ; M-34 il demande un **changement d'adresse** de l'enfant au secrétariat le lendemain : alerte à la cellule (SOC-331, R-SOC40), sans bloquer le 4 ni lui révéler l'IP.

### P12 — Mme Roux, gestionnaire
**Manque** : M-35 la remise appliquée doit porter un **numéro de décision** (preuve comptable) sans motif (confirmé, `ent_fs_application`) ; durée comptable anonyme (§18).

### P13 — M. Blanc, DPO (auditeur RGPD)
1. *Base légale* : obligation légale (L226-2-1, CPP 40) pour la protection ; mission d'intérêt public pour la coordination et le fonds social ; art. 9.2.h et 9.3 (secret) pour le cabinet ; art. 10 pour les signalements d'infractions (traitement sous l'autorité publique ou autorisé par le droit [H]) → **registre** (Q-SOC3, Q-SOC4).
2. *AIPD* : obligatoire (délibération 2018-327) ; dossier SOC-324.
3. *Minimisation* : trois cercles, aucune vue transversale, commission anonymisée, événements muets — tests §26.
4. *Durées* : copie d'IP jusqu'à la majorité + 1 an : à valider avec un juriste (Q-SOC7) ; effet des sauvegardes : destructions rejouées.
5. *Droits* : accès sous réserve des droits d'autrui ; **IP** : refus motivé au titulaire mis en cause, orientation vers la CRIP (Q-SOC11).
6. *Sous-traitants* : aucun fournisseur d'IA ; courriel en enveloppe neutre ; **aucune** pièce jointe.
7. *Traçabilité de ses propres accès* : le DPO lit le journal, et **sa** consultation est tracée et visible du titulaire du cercle (SOC-325).
8. *Next.js* : NXB-01 à NXB-10 sont des non-conformités en production (information trompeuse d'un mineur, sous-traitant IA, JSON non chiffré, lecteurs non habilités) → **à traiter par Florian** (Q-SOC1), avec notification au DPO de l'établissement.
**Manques** : M-36 SOC-324 ; M-37 SOC-325 ; M-38 revue mensuelle des membres (SOC-323).

### P14 — Attaquant 1 : membre du personnel mis en cause
**Tentatives** : (a) lire le journal pour savoir qui a ouvert le dossier : `journal.lire` générique n'expose rien (R-SOC17) ; (b) demander à l'IA : refus (R-SOC20) ; (c) se faire inscrire dans la cellule : suspension de casquettes (SOC-250) et la composition est journalisée et visible du DPO ; (d) déduire par l'agenda d'un collègue : « occupé » sans élève (19) ; (e) déduire par les événements : bus muet, file réservée (R-SOC16) ; (f) demander ses données (RGPD) pour obtenir le nom du signalant : les données d'un tiers ne sont pas communiquées [H, DPO].
**Manque** : M-39 lorsqu'un membre de la cellule est lui-même mis en cause, **toutes ses sessions** doivent être révoquées (SOC-326, R-SOC38).

### P15 — Attaquant 2 : parent qui utilise le compte de son enfant
**Tentatives** : (a) ouvrir « mes demandes » sur le téléphone de l'enfant : le compte élève exige la passkey (D-009) ; si elle est partagée, la demande n'affiche **pas** le contenu de la réponse de l'adulte (SOC-327) ; (b) effacer une demande : seulement si non lue, et la trace « retirée » reste pour le professionnel (SOC-328) ; (c) envoyer une fausse demande au nom de l'enfant : le professionnel rencontre l'enfant, pas un message.
**Manque** : M-40 un **mode discret** pour l'élève (icône neutre de l'espace « Parler à quelqu'un », aucun aperçu sur écran verrouillé) : ajouté au §21 (mobile).

### P16 — Florian et le module Next.js « bien-être »
**Constat** : NXB-01 à NXB-10. **Options** : (a) arrêter l'agent conversationnel tout de suite et garder le bouton « signaler » vers le psychologue seul ; (b) attendre SO2–SO4 (V1) puis basculer ; (c) garder tel quel jusqu'en 2028. **Recommandation** : (a) maintenant par une action de Florian sur Next.js (le pilote n'y touche pas), puis bascule vers SO2–SO4 à la V1 ; reprise des signalements par le psychologue (SO12). **Soumis à Florian.**

---

## 4. Manques transverses
- **Mobile** : brouillon chiffré, capture dans l'appli, mode discret de l'élève, validation passkey.
- **Autres socles** : 4 (contact ASE sans motif ; alerte au 36 sur changement de responsable ou d'adresse), 12 (coquille RA-21 « socle 3 » → 36), 13 (pas de questionnaires nominatifs au 36, R-SOC39), 14 (information de la cellule à la remise d'un enfant concerné), 19 (« occupé »), 24 (remise numérotée sans motif), 40 (repas d'urgence), 41 (surveillant de nuit).
- **IA** : aucune (R-SOC20) ; la passe n'a trouvé aucun cas où l'IA serait indispensable.

## 5. Vérification réglementaire (2026-11-11)
Reprise du §4 de `36_SUIVI_SOCIAL.md`. Sources consultées (extraits) : https://www.legifrance.gouv.fr/codes/id/LEGIARTI000045137055/2022-02-09 · https://eduscol.education.gouv.fr/4974/enfants-en-danger-comment-les-reperer-et-que-faire · https://solidarites.gouv.fr/sites/solidarite/files/2023-02/cadre_national_de_reference_-_livret_1.pdf · https://www.education.gouv.fr/bo/17/Hebdo12/MENE1709191C.htm · https://www.legifrance.gouv.fr/loda/id/JORFTEXT000034517664 · https://www.education.gouv.fr/bo/2023/Hebdo42/MENE2320218C · https://www.education.gouv.fr/phare-un-programme-de-lutte-contre-le-harcelement-l-ecole-323435 · https://sites.ac-nancy-metz.fr/lio/mlds/documents/prevention/GPDS.pdf · https://eduscol.education.fr/document/71146/download. Points à relire sur le texte entier : §28 de la conception.

## 6. Questions
### 6.1 Appliquées par défaut (D-031)
| # | Question | Recommandation |
|---|---|---|
| Q-SOC21 | Escalade automatique d'une préoccupation non prise | **Oui** |
| Q-SOC22 | Passkey et sessions courtes pour les membres de la cellule | **Oui** |
| Q-SOC23 | Psychologue : résultats des seuls élèves suivis, sur demande tracée | **Oui** |
| Q-SOC24 | Aucun questionnaire nominatif de repérage dans le 36 | **Oui** |
### 6.2 Soumises à Florian
Q-SOC1 et Q-SOC2 (Next.js « bien-être »), Q-SOC3, Q-SOC4, Q-SOC7, Q-SOC11 (DPO, juriste), Q-SOC5 (CRIP et parquet réels), Q-SOC6 (référents), Q-SOC9 (juriste : secret du psychologue).

## 7. Bilan
40 manques (M-01 à M-40) → 32 actions (SOC-300 à SOC-331), 8 règles (R-SOC33 à R-SOC40), 12 cas limites (CSOC-39 à CSOC-50), 4 réglages (S-2728 à S-2731), 4 questions (Q-SOC21 à Q-SOC24), 3 contradictions relevées (portée du psychologue, coquille 12 RA-21, M18 de T0).
