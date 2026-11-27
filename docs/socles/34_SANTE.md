# Socle 34 — Santé scolaire et infirmerie

**Date :** 2026-11-10 — **Statut :** conception de fond, **non décidée** (à relire par Florian), rédigée **après** le code (lot S2-11 : migrations 210–216, `sante_acc.rs`, `accompagnement.rs` ; lot 7 et antérieurs : migrations 040, 045, 052, 053, `infirmerie_repo.rs`, `fiche_sante*.rs`, `pai.rs`) pour le rattraper (`docs/PILOTAGE.md` §4, « rattrapage des domaines codés avant la méthode »). Le module santé est **désactivé par défaut** (migration 216) faute d'hébergeur certifié HDS. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à l'application Next.js.
**Branche :** `ent-socle-sante-conception`, partie d'`origin/ENT` (`ee7706d3`).
**Passe personas (2026-11-10, ajout) :** `PERSONAS_34.md` ; ajouts au **§30**, qui l'emporte en cas de contradiction (§28, points 11 à 16).
**À lire avec :** `CLAUDE.md` (données d'élèves, cloisonnement, autorisation côté serveur, minimisation, audit ; l'IA n'a jamais plus de droits que l'utilisateur) ; `docs/DECISIONS.md` (D-009, D-017, D-019, D-021, D-022, D-023, D-026, D-027, D-028 ; D-029 à D-033 sur les branches de conception) ; `docs/SOCLES.md` (socles 34, 35, 36, 12, 14, 40, 41, 42) ; `docs/SOCLE2_DOMAINES.md` (SAN-01 à SAN-08, ACC-01 à ACC-05) ; `docs/MATRICE_CHARLEMAGNE.md` §3.12 ; `02_ELEVE.md` (pastilles, bris de glace R-E11) ; `04_ADMINISTRATIF.md` (contacts d'urgence, remise, interdictions, majorité) ; `07_DOCUMENTS.md` (domaine `sante`, C3, signature S2 du PAI) ; `08_NOTIFICATIONS.md` (natures `sante.*`, défaut E6) ; `16_MESSAGERIE.md` (boîtes C3, Q-M21) ; `10_EMPLOI_DU_TEMPS.md` (AESH, CL-33, CL-34) ; `21_EVALUATIONS.md` (dispenses et aménagements sans motif) ; `IA_COEUR.md` (C3, classe « sensible », Q-IA5) ; `docs/VERIF_REGLEMENTAIRE.md` R23–R27 (branche `ent-verif-reglementaire`, non fusionnée) ; modèles de forme : `10_EMPLOI_DU_TEMPS.md`, `16_MESSAGERIE.md`, `21_EVALUATIONS.md`, `24_FINANCES.md`.
**Conventions.** **[F]** fait vérifié dans le dépôt (chemin et ligne cités) ; **[V]** règle lue sur une source officielle (URL citée, §5) ; **[Ve]** extrait officiel vu sans lecture du texte entier ; **[NV]** règle non vérifiée ; **[H]** hypothèse (pratique du terrain, interprétation) ; **[R]** recommandation (non décidée). Numérotation propre : actions **SAN-100** et suivantes (SAN-01 à SAN-08 sont pris par `SOCLE2_DOMAINES.md`, et la matrice emploie SAN-05 à SAN-12 avec un autre sens, §28) ; défauts **SD-01**… ; règles **R-S1**… ; cas limites **CS-01**… ; questions **Q-S1**… ; réglages signalés **S-180**… (après S-178 du socle 24).
**Plages de migrations :** **6400–6499** si la règle « 3000 + 100 × numéro de socle » (Q-B2 d'`ARCHITECTURE_BRIQUES.md`) est adoptée, **repli 580–599** sinon. Vérifié libre sur **toutes les branches distantes** le 2026-11-10 (`git ls-tree` de chaque `origin/*` : plus haute migration trouvée **480**, socle 16 M0a ; plus haute réservation **560–579**, socle 26). Q-B2 n'étant pas adoptée (les socles 21 et 24 ont pris leur repli), **le lot S0 part en 580**. Registre : `MIGRATIONS.md`.
**Méthode.** Lecture des décisions, de la carte, des conceptions 2, 4, 7, 8, 10, 16, 21, 24 et IA ; lecture directe des migrations 040, 045, 052, 053, 210 à 216, de `module_sante.rs`, de `ent-api/src/sante.rs` (garde du module), du catalogue d'actions (`ent-auth/src/actions.rs`) ; un audit indépendant du code en lecture seule (~80 lectures), dont **SD-01, SD-02, SD-04, SD-06 et SD-23 ont été relus ligne à ligne** par le concepteur (`roles.rs:172-176`, `actions.rs:969-972`, `sante_acc.rs:1264-1280` et `:1450-1458`, `reactions.rs:120-131` et `:168-194`, `service.rs:15998-16031`, `module_sante.rs:33-68`) ; recherche réglementaire (sous-agent, ~90 lectures) sur Légifrance (codes, JORF, LODA), BO (PDF), cnil.fr, curia.europa.eu, assemblee-nationale.fr, senat.fr ; esante.gouv.fr (liste des hébergeurs certifiés) et les pages HTML d'education.gouv.fr ont refusé l'accès automatisé (points marqués [Ve] ou [NV]).

Sommaire : §0 En bref · §1 Thèse · §2 Existant · §3 Acteurs · §4 L'année vue de l'infirmerie · §5 Réglementaire · §6 Trois niveaux d'activation et l'hébergement · §7 Frontières · §8 Concepts · §9 Secret et partage minimal · §10 Les domaines un par un · §11 Familles, mineur, majeur · §12 Cas limites · §13 IA · §14 Actions SAN · §15 Règles R-S · §16 Droits · §17 Chiffrement et journal des accès · §18 Conservation · §19 Événements · §20 Notifications · §21 Écrans et mobile · §22 Modèle de données · §23 Plan de migration · §24 Lots et migrations · §25 Tests et recette · §26 Réglages signalés · §27 Questions · §28 Faits, hypothèses, contradictions · §29 Les cinq décisions prioritaires.

---

## 0. En bref

1. **Un module prudent mais percé.** L'existant (S2-11) a de bonnes idées — catalogue fermé de contraintes sans texte libre, fiche d'urgence à deux niveaux, champs chiffrés AES-GCM, événements sans donnée médicale, module désactivé par défaut — mais **le secret médical n'est pas tenu** : la **direction et l'administrateur ont `sante.lire` et `sante.saisir` par défaut** et lisent soins, notes confidentielles, PAI et ordonnances (SD-01) ; le **renvoi au domicile ignore les interdictions de remise** (SD-02) ; la garde « module désactivé » ne tient qu'au **forme du chemin HTTP** et laisse passer le dossier élève, les dispenses et le journal (SD-03) ; le **professeur lit le motif médical et le commentaire** des dispenses du jour (SD-04) ; les **avis d'infirmerie n'arrivent pas** aux parents sans droit « vie scolaire » (SD-06) ; les **ordonnances ne sont pas chiffrées** dans le magasin d'objets (SD-09). Un **lot S0** les ferme **avant toute activation, même en démonstration**.
2. **La thèse (§1).** La santé à l'école n'est pas un dossier que l'établissement possède : c'est **un secret confié à un professionnel** (infirmier, médecin), dont l'établissement n'a le droit de connaître que **les conséquences utiles** (« peut manger à la cantine avec un panier repas », « trousse d'urgence : sac rouge, salle des professeurs », « est à l'infirmerie depuis 10 h 12 »). ScolIA bâtit donc deux mondes étanches : **le cabinet** (secret, chiffré, hébergé HDS, lu des seuls soignants, chaque lecture tracée) et **la vie de l'établissement** (consignes fonctionnelles codées, jamais un diagnostic, jamais un soin).
3. **Le réglementaire tranche huit points [V]** (§5) : (a) le **secret** s'impose aux soignants ; le partage hors équipe de soins exige l'**accord** de la personne, et ni la direction, ni le CPE, ni les professeurs ne sont dans la liste des non-soignants autorisés (CSP L1110-4, R1110-2) ; (b) l'arrêté « SAGESSE » (2001) limite ce que reçoivent chef, CPE et professeurs au **nom et prénom** de l'élève (mouvements à l'infirmerie, dispenses), et rend l'**infirmier seul responsable** des données couvertes par le secret ; (c) l'**hébergement par un tiers** de données de santé exige un **certificat HDS** — et un éditeur SaaS comme ScolIA **est ce tiers**, pas seulement Scaleway (CSP L1111-8, R1111-8-8, R1111-9) ; stockage dans l'UE/EEE depuis le 26/09/2026 (décret 2026-209) ; (d) une indication **même indirecte** (« repas sans arachide ») est une donnée de santé au sens du RGPD (CJUE C-184/20) : l'option « sans donnée de santé » n'existe pas, seule existe une option **sans dossier de soins** ; (e) la **contraception d'urgence** de la mineure est couverte par le **secret** vis-à-vis des parents (CSP L5134-1, D5134-5 et s.), compte rendu dans le « cahier de l'infirmière », état chiffré annuel ; (f) le **PAI** (circulaire du 10/02/2021) : rédigé par le médecin, signé avec la famille et le chef d'établissement, ne révèle pas le diagnostic aux personnels, renouvelé à chaque rentrée, trousse emportée à chaque sortie ; (g) l'**inaptitude en EPS** se décrit en **incapacités fonctionnelles** sans diagnostic, valable l'année au plus, jamais rétroactive (R312-2, D312-4, arrêté du 13/09/1989) ; (h) le protocole national de 2000 impose un **registre** des soins et urgences et une **fiche d'urgence non confidentielle** annuelle, les informations confidentielles passant « sous enveloppe fermée ».
4. **Trois niveaux d'activation (§6)** au lieu d'un booléen : **N-0 inactif** (aujourd'hui) ; **N-1 « sécurité et organisation »** (contraintes fonctionnelles codées, fiche d'urgence, protocoles génériques, trousses, mouvements à l'infirmerie sans motif ni soin, inaptitudes en incapacités fonctionnelles, registre des PAI sans contenu médical) — données de santé au sens du RGPD mais **sans dossier de soins**, activable **sans HDS seulement après avis écrit du DPO** (point juridique ouvert, R23) ; **N-2 « cabinet infirmier »** (passages complets, soins, traitements, contraception d'urgence, vaccinations détaillées, dépistages, contenu du PAI, ordonnances) — **uniquement avec un hébergement certifié HDS de bout en bout**, dans une base séparée, avec ses propres clés.
5. **Public et privé ne se servent pas du module de la même façon (D-019).** Dans le **public**, les infirmiers et médecins de l'Éducation nationale tracent leurs actes dans les applications nationales **LIEN** (infirmiers) et **Esculape** (médecins) [V, rapport AN n° 1228 ; circulaire 2015-119] : ScolIA ne doit **pas** devenir un second dossier de soins (double saisie, deux secrets) ; il apporte N-1 et la logistique. Dans le **privé sous contrat**, l'infirmière est souvent salariée de l'OGEC, sans outil national [H] : N-2 y a toute sa place, sous HDS.
6. **Ce que ScolIA apporte de plus que le marché [H]** : un professeur sait **en temps réel** qu'un élève est à l'infirmerie (l'appel le montre, ce n'est pas une absence) ; l'infirmière renvoie un enfant au domicile **avec la photo et les interdictions de remise sous les yeux**, et l'absence se crée seule ; le PAI se signe en ligne **par chaque titulaire**, chaque version est figée et prouvée, la trousse a une date de péremption qui alerte ; un encadrant de voyage a la **fiche d'urgence hors ligne** de ses élèves, effacée au retour ; une mineure peut venir pour une contraception d'urgence **sans qu'aucun avis, aucune absence motivée, aucune ligne de journal visible ne la trahisse** ; l'infirmière voit **qui a lu quoi** dans ses dossiers.
7. **Le plan.** **160 actions** (SAN-100 à SAN-295, numérotation à trous), **58 cas limites** (CS-01 à CS-58), **34 règles** (R-S1 à R-S34), **14 réglages signalés** (S-180 à S-193), **16 lots** : **S0** (correctifs urgents de l'existant), **S1** (droits, cercles, niveaux, journal), **S2 à S4** (N-1 : sécurité et organisation, PAI, inaptitudes), **S5 à S9** (N-2 : cabinet, vaccinations et dépistages, sorties, crises, conservation et droits d'accès), **S10 à S15** (messagerie de l'infirmerie, mobile, accompagnement avec le 35, articulation LIEN / Esculape, IA, RLS stricte et retraits). **Parcours minimal d'un pilote** : S0, S1, S2, S3, S4 en **N-1** — après l'avis du DPO ; N-2 attend l'hébergement HDS.
8. **Décisions appliquées par défaut (D-031)** : 28 questions (Q-S1 à Q-S28) tranchées selon la recommandation ; **soumis à Florian** : **hébergeur HDS et coût** (certification de ScolIA comme hébergeur ou infogérance certifiée, contrat Scaleway HDS), **DPO** (avis sur N-1 sans HDS, AIPD, durées de conservation), **infirmier et médecin référents** du pilote (relecture métier, protocoles, PAI).
9. **Ajout personas (§30)** : **N-0 devient « sans donnée de santé »** au sens strict (statut neutre « sortie autorisée », sans lieu) ; **mouvement confidentiel dès N-1** ; audience « qui a l'élève maintenant » ; accès d'urgence de la vie scolaire ; **mode sans infirmière** ; **IA exclue de toute donnée du 34** ; destructions rejouées après restauration, cabinet sauvegardé en HDS sur chaque destination, export santé chiffré pour un destinataire nommé ; **jalon N-1 avant la bascule des voyages (16 avril 2027)** ; risque de production du module santé de Next.js signalé (Q-S34). 33 actions (SAN-300 à SAN-332), 18 règles (R-S35 à R-S52), 12 cas limites (CS-59 à CS-70).

---

## 1. Thèse : un secret confié, des conséquences partagées

1. **Le secret appartient à l'élève, pas à l'établissement.** Ce que l'élève ou sa famille confie à l'infirmière ou au médecin reste chez eux. Ni le rôle « direction », ni l'administrateur technique, ni le support ScolIA n'y ont accès par construction (base séparée, clés distinctes, aucun droit par défaut, R-S1).
2. **On partage une conséquence, jamais une cause.** L'établissement reçoit des **consignes fonctionnelles** tirées d'un catalogue fermé (« ne pas faire courir », « accès libre aux toilettes », « panier repas », « trousse d'urgence : lieu »), datées, publiées par un soignant pour une **audience** précise (restauration, professeurs de l'élève, encadrants d'une sortie) ; jamais un diagnostic, un médicament ni un texte libre (R-S3).
3. **Savoir où est l'enfant n'est pas un secret.** « Léa est à l'infirmerie depuis 10 h 12 », « Léa est rentrée chez elle à 11 h, récupérée par sa mère » : c'est ce que l'arrêté SAGESSE autorise à transmettre au chef, au CPE et aux professeurs (nom, prénom, mouvement) [V] ; ScolIA le rend **immédiat** (l'appel, la vie scolaire) et **sans motif**.
4. **Un soin se trace pour le soignant.** Le registre des soins et urgences [V, protocole 2000] et le « cahier de l'infirmière » [V, D5134] sont des outils professionnels : chiffrés, horodatés, non modifiables (on complète, on ne réécrit pas), lisibles du seul cercle de soins.
5. **Le mineur a des secrets que ses parents n'ont pas le droit de lever.** Contraception d'urgence, santé sexuelle, opposition du mineur (L1111-5, L1111-5-1, L5134-1) [V] : ces actes sont **invisibles par construction** de toutes les vues familiales, de toutes les notifications, de tout motif d'absence ; un garde en base refuse leur publication (R-S12).
6. **L'élève majeur décide seul.** À 18 ans, les parents perdent l'accès à sa santé ; il signe son PAI ; il peut leur rouvrir un accès, révocable (R-S13).
7. **Pas de donnée de soins hors d'un hébergement certifié.** Le cabinet (N-2) n'existe qu'en HDS ; ce qui n'est pas HDS ne reçoit jamais un soin, une ordonnance, un compte rendu (R-S2).
8. **Chaque lecture laisse une trace que le soignant peut lire.** Le journal des accès est écrit **dans la même transaction** que la lecture (pas de trace, pas de donnée), sans contenu médical, et l'infirmière titulaire voit qui a ouvert ses dossiers (R-S8).
9. **Le temps efface.** Les données du cabinet suivent la scolarité dans l'établissement, puis sont transmises (avec accord) ou détruites ; rien n'est gardé « au cas où » (R-S25, §18).
10. **L'IA ne lit pas la santé.** Aucune capacité IA n'a accès au cabinet (N-2) ; les consignes fonctionnelles (N-1) ne sont lues par l'IA que comme le ferait la personne, et jamais pour en déduire une cause (R-S30, `IA_COEUR.md` Q-IA5).

---

## 2. Évaluation franche de l'existant

Code lu sur `origin/ENT` (`ee7706d3`), chemins depuis `ent-rust/`. Un audit indépendant en lecture seule ; **SD-01, SD-02, SD-04, SD-06 et SD-23 relus ligne à ligne**.

### 2.1 Ce qu'il y a [F]

| Élément | Où | Ce que ça fait |
|---|---|---|
| Fiche santé | `045_fiche_sante.sql`, `domain-eleve/src/fiche_sante.rs`, `service.rs:16178-16290` | une par élève : allergies, traitements, contre-indications, médecin, téléphone d'urgence, note confidentielle (**chiffrés**, `champs_chiffres.rs:31-41`), `pai_actif` ; saisie par la famille **et** par l'infirmerie, sans historique |
| Passages | `040`, `213`, `sante_acc.rs:1100-1390` | jour, heures, motif (5 valeurs, **en clair**), note de soins et soins (**chiffrés**), issue (retour en classe, repos, famille appelée, renvoi au domicile, secours), information de la famille (`non` / `a_informer` / `informee`), personne qui récupère |
| PAI | `052`, `053`, `210`, `pai.rs`, `sante_acc.rs:504-856` | une ligne par élève (motif, mesures d'accueil, protocole d'urgence, note interne **chiffrés**) ; `version` incrémentée à chaque écriture ; signatures par partie et par version (famille, médecin, chef d'établissement, infirmier, collectivité) avec contrôle de version `FOR UPDATE` ; révision annuelle ; trousse (présence, lieu, péremption) ; ordonnances en pièces jointes (anciennement `BYTEA` 256 Kio, puis magasin d'objets avec lien pré-signé contrôlé et tracé, `service.rs:17890-17935`) |
| Contraintes publiées | `211`, `sante_acc.rs:92-110`, `:856-1100` | **catalogue fermé de 17 codes** (repas sans arachide / fruits à coque / lait / œuf / gluten / poisson, panier repas, PAI alimentaire, trousse d'urgence, ne pas courir, pas d'effort intense, accès libre à l'eau / aux toilettes, pauses, éviter la chaleur, prévenir l'infirmerie, ascenseur) ; audiences restauration / enseignants / sorties ; source (`pai`, `pap`, `pps`, `manuel`) **masquée** hors personnel de santé (`sante_acc.rs:1090`) ; synchronisation depuis le PAI |
| Fiche d'urgence | `sante_acc.rs:1392-1600` | niveau **minimal** (encadrant : contacts, contraintes non alimentaires, trousse et lieu, consigne générale) ou **complet** (santé : + protocole d'urgence, médecin, téléphone) |
| Protocoles | `214` | textes génériques de l'établissement, versionnés, jamais liés à un élève |
| Accompagnement | `212`, `accompagnement.rs` | dispositifs (PAP, PPS, PAI, GEVA-Sco, PPRE, ULIS) avec statut et révision ; **catalogue fermé de 14 aménagements** (tiers-temps, police adaptée, secrétaire, lecteur, salle séparée…) portée cours / examen ; AESH saisie par son nom ; réunions d'équipe éducative avec compte rendu **chiffré** |
| Dispenses | `dispense_vs.rs`, `routes.rs:463` | dispenses (EPS…) avec motif (dont `medical`) et commentaire **en clair**, lues par l'appel |
| Module | `216`, `module_sante.rs`, `ent-api/src/sante.rs:39-107` | `ent_module_etablissement` (un booléen) **fermé par défaut** ; couche HTTP `exiger_module` qui répond 403 sur les chemins de santé tant que le module est inactif |
| Droits | `ent-auth/src/actions.rs:940-975`, `roles.rs:172-176` | `sante.lire`, `sante.saisir` (Admin, **Direction**, Infirmerie) ; `contraintes.lire`, `sante.urgence_lire` (Admin, Direction, Administratif, CPE, Infirmerie) ; `amenagements.lire` (+ Psy ; professeurs pour leurs élèves par la route dédiée) ; `accompagnement.gerer` (Admin, Direction) ; `documents.lire_sante` |
| Événements | `sante_acc.rs`, `reactions.rs:115-131` | identifiants et états seulement (test sentinelle `tests_sante.rs:312`) ; `infirmerie.famille_informee` transformé en avis aux responsables |
| RLS | `100:29`, `215` | ENABLE + FORCE sur `ent_fiche_sante`, `ent_pai`, `ent_pai_piece`, `ent_infirmerie_passage` et les six tables de 210–214 (mode `off` par défaut) |
| PDF | `fiche_sante_pdf.rs` | résumé de fiche santé régénéré à chaque appel, non stocké |
| Tests | `ent-db/src/tests_sante.rs`, `ent-api/src/tests_sante_module.rs` | PAI et signatures, chiffrement en base, minimisation des événements, contraintes par audience, information de la famille, renvoi au domicile, aménagements vus du professeur, niveaux de fiche d'urgence, isolation et RLS, protocoles ; forme des chemins gardés |

### 2.2 Défauts trouvés (à corriger avant toute activation)

| # | Gravité | Défaut | Conséquence | Où [F] | Lot |
|---|---|---|---|---|---|
| **SD-01** | **Critique** | **La direction et l'administrateur ont `sante.lire` et `sante.saisir`** par défaut (`can_saisir_infirmerie` = Admin, Direction, Infirmerie) ; même schéma pour `psy.lire` | le chef d'établissement, son adjoint et l'administrateur technique lisent notes confidentielles, soins, PAI complets, ordonnances ; fiche d'urgence complète ; **violation du secret** (L1110-4, R1110-2 : la direction n'est pas un professionnel autorisé [V]) | `roles.rs:172-176`, `actions.rs:969-972` ; test qui fige ce comportement `actions.rs:1240` | S0 |
| **SD-02** | **Critique** | **Renvoi au domicile et fiche d'urgence ignorent les interdictions de remise** : tout responsable au lien actif est « habilité à récupérer » (`peut_recuperer: true` en dur) ; `ent_eleve_interdiction` (socle 4, 271) n'est jamais lue | l'infirmière remet un enfant malade **au parent visé par une interdiction judiciaire** ; la fiche d'urgence d'un voyage le présente comme personne à appeler pour venir le chercher | `sante_acc.rs:1264-1280`, `:1450-1458` | S0 |
| **SD-03** | Élevée | **Garde du module seulement HTTP et seulement sur la forme du chemin** : aucune fonction de service ne vérifie le module ; routes non couvertes : `/eleves/{id}/dossier` (section `dispositifs` : PAP, PPS, PAI, libellé libre), `/vs/dispenses` et `/eleves/{id}/dispenses-vs` (motif médical), `/eleves/{id}/journal`, documents de confidentialité santé ; un test affirme que `/dossier` n'est pas gardé | données de santé servies alors que le module est « désactivé faute d'HDS » ; tout appelant non HTTP (rendu serveur Leptos, tâches, labo) contourne la garde | `ent-api/src/sante.rs:48-107`, `lib.rs:167-173`, `service.rs:4655-4732`, `tests_sante_module.rs:83` | S0 |
| **SD-04** | Élevée | **Liste du jour `/vs/dispenses` : motif `medical` et commentaire libre servis au professeur** (masquage présent seulement sur la liste par élève) ; le secrétariat voit aussi le commentaire | « asthme sévère, Ventoline » sous les yeux d'un professeur ou d'un AED | `service.rs:15998-16031` (masquage `:15975-15985`) | S0 |
| **SD-05** | Élevée | **Le secrétariat lit les documents de confidentialité « santé »** (`can_read_sante_docs` inclut Administratif) | certificats et pièces médicales lisibles hors cercle de soins | `roles.rs:164-169`, `service.rs:9932-9943` | S0 |
| **SD-06** | Élevée | **Avis d'infirmerie perdus hors de l'ENT** : destinataires choisis sans `peut_voir_vs`, mais envois e-mail / SMS / push portant `droit_requis = peut_voir_vs`, annulés à l'expédition (`droit_retire`) | le parent sans droit « vie scolaire » (séparé, par exemple) n'apprend **jamais** que son enfant a été renvoyé au domicile, sauf s'il ouvre l'ENT (défaut E6 du socle 8) | `reactions.rs:168-194`, `notifications.rs:851-865` | S0 (avec N0b du socle 8) |
| **SD-07** | Moyenne | **Le texte de l'avis révèle le passage à l'infirmerie** au prestataire SMS / e-mail (« s'est présenté(e) à l'infirmerie ») | donnée de santé chez un tiers non HDS ; lisible sur l'écran verrouillé d'un téléphone partagé | `reactions.rs:120-131` | S0 |
| **SD-08** | Élevée | **Familles : ni autorité parentale, ni majorité, ni confidentialité du mineur.** Fiche, PAI, ordonnances et passages ouverts à tout lien parent actif ; une seule signature « famille » par version ; aucun drapeau « secret vis-à-vis des titulaires » (seul `visible_famille = false` par défaut protège) | un parent sans autorité lit le PAI ; les parents d'un élève **majeur** lisent sa santé ; impossible d'enregistrer une contraception d'urgence sans risque d'exposition | `service.rs:16257-16264`, `:16313-16323`, `sante_acc.rs:626-631`, `:1327-1328`, `210:31` | S0 (fermeture), S5 (modèle) |
| **SD-09** | Élevée | **Champs de santé en clair** : ordonnances dans le magasin d'objets (aucun chiffrement applicatif, `ent-object/src/lib.rs:185-199`) et `BYTEA` hérité de 053 ; `motif`, `issue`, `recupere_par_nom` du passage ; `trousse_lieu` ; nom du médecin signataire ; commentaire de dispense ; libellé de dispositif ; nom d'AESH | une sauvegarde, un accès au stockage ou une requête directe livre des ordonnances et des motifs | `champs_chiffres.rs:27-65`, `service.rs:16409-16413` | S0 (pièces, commentaire, libellé), S5 |
| **SD-10** | Moyenne | **RLS absente** sur `ent_dispense_vs` et `ent_eleve_dispositif` ; RLS permissive hors mode strict | isolation des établissements par le seul code applicatif pour ces deux tables | `100:29`, `215:9-10` | S0 |
| **SD-11** | Moyenne | **Trace des accès « fail-open » et incomplète** : si l'écriture de la trace échoue, la donnée est rendue ; PDF de fiche santé, listes de contraintes et d'aménagements, dossier élève, dispenses non tracés ; `audit.lire` (admin, direction, **secrétariat**) révèle « Consultation du PAI » pour un élève, donc l'existence d'un PAI | lecture sans preuve ; fuite par le journal | `service.rs:3903-3917`, `:16505-16568`, `:13265-13273`, `actions.rs:992` | S0 (fail-closed), S1 |
| **SD-12** | Élevée | **PAI sans versions conservées** : une ligne écrasée, `version` incrémentée, contenu signé précédent perdu | impossible de prouver **ce qui a été signé** en cas d'accident (un parent conteste la conduite à tenir) | `infirmerie_repo.rs:296-313`, `052:18` | S0 (instantané), S3 |
| **SD-13** | Moyenne | **Pièces du PAI** : plafond 256 Kio (un scan ne passe pas), écriture dans le magasin **avant** l'insertion en base (orphelins), toutes les pièces visibles de tout parent | ordonnance perdue ou orpheline dans le stockage ; ordonnance vue par un parent sans autorité | `053:14`, `pai.rs:12`, `service.rs:16411-16414` | S3 |
| **SD-14** | Moyenne | **Passage modifiable sans trace** : la clôture se rejoue et écrase `soins` et `issue` (`COALESCE`) ; aucune correction tracée | le registre, pièce de preuve d'un accident, peut être réécrit | `sante_acc.rs:1167-1241`, `:1207` | S0 |
| **SD-15** | Moyenne | **Fiche santé écrasable par un parent** (allergies, traitements saisis par l'infirmière) sans historique | une allergie validée disparaît d'une saisie familiale | `service.rs:16178-16240`, `infirmerie_repo.rs:149-206` | S0 (historique), S5 (propositions) |
| **SD-16** | Moyenne | **Fusion d'élèves incohérente** : fiche et PAI uniques restent sur l'absorbé, mais pièces, signatures et contraintes « déplacées » vers le conservé tout en pointant le PAI de l'absorbé ; dispenses classées « Discipline », contraintes « Emploi du temps » dans l'aperçu | ordonnances d'un enfant visibles par le PAI d'un autre ; aperçu de fusion trompeur | `domain-eleve/src/fusion.rs:78-248` | S0 |
| **SD-17** | Moyenne | **`contraintes.lire` et `sante.urgence_lire` à l'échelle de l'établissement** pour direction, secrétariat, CPE ; le **code** révèle un diagnostic (« repas sans arachide », « trousse d'urgence » publiée automatiquement aux professeurs et sorties) | chacun peut lister « tous les élèves qui ont une trousse d'urgence » sans besoin d'en connaître | `actions.rs:955-966`, `sante_acc.rs:92-110` | S0 (portées), S2 (catalogue) |
| **SD-18** | Moyenne | **Aucune durée de conservation ni purge** sur les tables de santé | données gardées sans limite, contraire à SAGESSE (destruction en fin de scolarité [V]) | — | S9 |
| **SD-19** | Moyenne | **Aucun rôle médecin**, aucun rôle « infirmier remplaçant » ; un seul rôle « infirmerie » pour tout | le médecin scolaire (souvent partagé entre établissements) n'a pas de place ; le remplaçant reçoit tout ou rien | `roles.rs` | S1 |
| **SD-20** | Moyenne | **Aucun lien avec l'appel** : l'élève à l'infirmerie est noté absent ou rien ; le renvoi au domicile **ne crée pas d'absence** (`MATRICE_CHARLEMAGNE.md` l. 1238) | absence signalée aux parents pendant que l'enfant est à l'infirmerie ; enfant rentré chez lui sans trace à la vie scolaire | `sante_acc.rs:1167-1241` | S2 |
| **SD-21** | Moyenne | **Le texte libre `libelle` d'un dispositif** et les codes PAP / PPS sont servis dans `/eleves/{id}/dossier` à six profils | « PPS — troubles autistiques » lisible du secrétariat et du CPE | `service.rs:4655-4671` | S0 |
| **SD-22** | Faible | **Données de démonstration médicales réalistes** (« Asthme », « Ventoline », « Arachides ») écrites sans passer par la garde du module | habitue l'équipe et les démonstrations à des écrans qui affichent un diagnostic | `seed.rs:1212-1242`, `:4371-4373`, `ent-lab/src/main.rs:8484-8490` | S0 |
| **SD-23** | Élevée | **Activation du module en un clic** par tout détenteur de `securite.politique`, sans attestation (contrat HDS, AIPD, avis du DPO), sans confirmation renforcée ; journal écrit **après** et hors transaction ; un seul niveau (tout ou rien) | un administrateur active le module « pour essayer » et l'établissement héberge des soins hors HDS | `module_sante.rs:33-68` | S0 (N3 + attestation), S1 (niveaux) |
| SD-24 | Faible | **PDF de fiche santé non stocké** et téléchargeable par les applis mobiles [H : stockage local non vérifié] | copie médicale sur un téléphone, sans effacement | `fiche_sante_pdf.rs`, `routes.rs:670` | S0 (route fermée), S11 |

**Ajout personas** : SD-25 (signatures du PAI en cascade, FK non composite, `210:22`), SD-26 (tableaux `TEXT[]`), SD-27 (clé unique `ENT_SECRETS_KEY` pour IBAN, santé, psychologie), SD-28 (N-0 garde « envoyé à l'infirmerie ») : `PERSONAS_34.md` §3.2, lots au §30.10.

### 2.3 Verdict : garder, corriger, remplacer

- **Garder** : le **catalogue fermé** de contraintes et d'aménagements (sans texte libre, la base le garantit) ; la **fiche d'urgence à deux niveaux** ; la **signature du PAI par partie et par version** avec contrôle de version ; le chiffrement AES-GCM par champ avec AAD par ligne ; les **événements sans donnée médicale** et leur test sentinelle ; le lien pré-signé tracé ; le module **fermé par défaut**.
- **Corriger tout de suite (S0)** : SD-01 à SD-17, SD-21 à SD-24 — **avant toute activation, même en démonstration** : retirer la santé à la direction et à l'administrateur, lire les interdictions de remise, garder le module dans le service, masquer motifs et commentaires, fermer la trace, figer les versions du PAI, rendre le registre non réécrivable.
- **Remplacer** progressivement : le booléen du module par **trois niveaux** (§6) ; la fiche santé « famille + infirmerie » par un **dossier infirmier** (N-2) alimenté par des **propositions** de la famille ; la ligne unique de PAI par des **versions immuables** ; les catalogues par des **consignes fonctionnelles** mieux formulées (« sans arachide » devient « repas : éviction de l'arachide » — c'est la même donnée, mais §9.4 explique ce qu'on gagne à la rédiger comme une consigne) ; le rôle « infirmerie » par des **cercles** (§9).
- **Transférer au socle 35** (inclusion) ce qui est pédagogique dans 212 (dispositifs PAP, PPS, PPRE, ULIS, aménagements pédagogiques et d'examen, AESH, équipes) : le socle 34 garde **PAI, inaptitudes, consignes de santé, avis médical** et fournit au 35 l'avis du médecin (§7).

### 2.4 Ce que fait l'application Next.js de production (sans lecture nouvelle)
D'après la matrice (`MATRICE_CHARLEMAGNE.md`, ligne 460 [F, lecture déjà faite par une autre session]) : `app/api/sante/`. **Aucune lecture nouvelle** n'a été faite. Le document `IA_COEUR.md` (§2, ligne 771 [F]) signale déjà que Next.js envoie à un fournisseur d'IA des données de santé mentale d'élèves mineurs : **risque de production à signaler à Florian**, hors du périmètre de ce document (Next.js intouchable).

**Ajout personas (lecture ciblée faite le 2026-11-10)** : NXS-01 à NXS-12 (`PERSONAS_34.md` §3.1). **Risque de production** : le module `sante` de Next.js est ouvert aux directions, administrateurs, secrétariat, CPE, surveillants, psychologue et infirmerie, avec textes de soins en clair, sans journal des accès, suppression en cascade avec l'élève (`app/lib/intranet-modules.ts:922-931`, `drizzle/0054`–`0056`) ; signalé à Florian (Q-S34).

---

## 3. Acteurs, besoins réels, irritants

Pratiques des logiciels du marché et du terrain **de mémoire** : **[H]** partout, à confirmer avec l'infirmier et le médecin référents du pilote (Q-S3) et à la passe personas.

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps aujourd'hui [H] | Ce qu'il redoute | Ce qui le ravirait | Fréquence |
|---|---|---|---|---|---|
| **Infirmier(e) scolaire** (public : EN ; privé : souvent salarié de l'OGEC) | Accueille 15 à 60 élèves par jour ; soigne, écoute, oriente ; appelle les familles ; renvoie au domicile ; tient le registre et le cahier ; rédige les PAI avec le médecin ; gère trousses et médicaments ; dépistages ; contraception d'urgence ; éducation à la santé ; rapport d'activité | Double saisie (LIEN + cahier papier + logiciel de vie scolaire) ; appeler trois numéros avant de joindre un parent ; chercher qui a le droit de venir chercher l'enfant ; relancer les PAI non renouvelés en septembre ; vérifier les péremptions des trousses à la main ; les statistiques de fin d'année au tableur | Remettre un enfant à la mauvaise personne ; qu'un dossier soit lu par la direction ou la vie scolaire ; qu'une mineure soit trahie par un SMS ; un PAI périmé le jour d'une crise ; être seule et remplacée sans transmission | **Un registre rapide** (trois touches sur la tablette : élève par badge, issue, heure), l'appel qui se met à jour seul, « appeler la famille » qui sonne les joignables dans l'ordre ; **la remise sécurisée** (photo, autorisés, interdictions en rouge) ; un **tableau de rentrée des PAI** (à renouveler, ordonnances reçues, trousses périmées) ; ses **statistiques prêtes** ; savoir **qui a lu** ses dossiers | plusieurs fois par heure |
| **Médecin scolaire** (EN, partagé entre de nombreux établissements ; ou médecin de PMI, médecin traitant pour un PAI) | Rédige les PAI, avise les PAP, examine les inaptitudes de plus de 3 mois, visites et bilans, cas complexes, signalements | Ne pas savoir quels PAI de quels établissements sont à revoir ; recevoir des pièces par courrier ; signer à la main quatre exemplaires | Que ce qu'il écrit soit lu par l'établissement ; signer une version qui change ensuite | **Une file multi-établissements** (PAI à rédiger, avis demandés) ; un PAI pré-rempli par l'infirmière ; **signature en ligne** d'une version figée ; aucune donnée qu'il n'a pas besoin de voir | hebdomadaire |
| **Psychologue de l'EN** | Entretiens, bilans, orientation, situations de détresse | Ne pas savoir qu'un élève est passé trois fois à l'infirmerie pour des maux de ventre avant chaque contrôle | Que son dossier se mélange à celui de l'infirmière | Une **coordination consentie** avec l'infirmière (« un échange ouvert pour Léa, avec son accord »), sans fusion des dossiers (socle 36) | hebdomadaire |
| **CPE / vie scolaire (AED)** | Envoie les élèves à l'infirmerie ; gère les absences ; reçoit les familles ; prévient l'infirmerie d'une situation | « Il est où ? » ; un élève parti de l'infirmerie sans repasser par la vie scolaire ; un renvoi au domicile découvert le lendemain | Connaître un diagnostic qu'il ne devrait pas connaître ; qu'un élève quitte l'établissement sans trace | **Savoir qui est à l'infirmerie maintenant** (liste sans motif) ; le renvoi au domicile **crée l'absence** et la sortie ; « Léa a une consigne : prévenir l'infirmerie en cas de malaise » | plusieurs fois par jour |
| **Professeur** (dont EPS, professeur principal) | Envoie un élève à l'infirmerie ; adapte son cours ; encadre des sorties | Fiches papier perdues ; dispense d'EPS transmise par l'élève lui-même ; ne pas savoir qu'un élève a un PAI le jour d'un malaise | Être responsable d'un accident faute d'information ; recevoir des confidences médicales qu'il ne sait pas garder | Dans **l'appel** : « à l'infirmerie depuis 10 h 12 », « inapte EPS partiel : pas de course, pas de saut » ; sur la fiche de ses élèves : **consignes fonctionnelles** et « conduite à tenir en cas d'urgence » si un PAI le prévoit ; **jamais de diagnostic** | quotidienne |
| **Direction** (chef d'établissement, adjoint) | Signe les PAI (organisation) ; responsable de la sécurité ; pilote les crises ; reçoit les signalements ; organise l'infirmerie (local, horaires, remplacements) | Ne pas savoir combien d'élèves ont un PAI avant un voyage ; une épidémie découverte par la presse | **Être accusée d'avoir lu un dossier médical** ; un accident sans conduite à tenir ; une crise mal communiquée | **Indicateurs agrégés** (passages par jour, PAI à jour, trousses valides) sans nom ; la **liste opérationnelle** d'une sortie (« 3 élèves avec trousse, 1 avec panier repas ») ; un **mode crise** avec modèles validés ; signer le PAI **sans voir le diagnostic** | hebdomadaire, crise |
| **Secrétariat** | Collecte la fiche d'urgence annuelle, les justificatifs de vaccination à l'admission, les certificats ; transmet à l'infirmerie | Enveloppes fermées à faire passer ; photocopies du carnet de santé | Ouvrir une enveloppe qui ne lui est pas destinée ; perdre un certificat | **« Dépôt sous pli »** : la famille dépose en ligne, le secrétariat voit « pièce de santé reçue », **seule l'infirmière l'ouvre** ; statut vaccinal d'admission « conforme / provisoire jusqu'au… » sans le détail | rentrée surtout |
| **Accueil** | Voit partir l'élève renvoyé au domicile ; reçoit le parent | « Qui vient le chercher ? » ; parent pressé sans pièce d'identité | Laisser partir un enfant avec une personne interdite | L'écran de **remise** du socle 4 avec la **sortie santé** déjà préparée par l'infirmière (qui, à quelle heure, photo) | quotidienne |
| **Restauration / internat** | Repas adaptés, paniers repas, PAI alimentaires ; surveillance de nuit | Listes papier non à jour ; un nouvel élève allergique inconnu du chef | **Un choc anaphylactique au self** | **Liste du jour des consignes alimentaires** (photo + consigne + lieu de la trousse), alerte au passage du badge, mise à jour à chaque PAI ; à l'internat : consignes de nuit et trousse | quotidienne |
| **Organisateur de sortie ou de voyage** (professeur) | Prépare la liste, la trousse, les autorisations ; encadre | Collecter les fiches sanitaires papier ; ne pas savoir si un PAI prévoit les nuitées | Partir sans la trousse ; ne pas pouvoir joindre une famille à l'étranger | **Fiche d'urgence hors ligne** (photo, contacts, consignes, conduite à tenir prévue par le PAI, trousse emportée cochée), **effacée au retour** ; récapitulatif « 2 trousses à emporter » (socle 42) | par sortie |
| **AESH** | Accompagne un élève (aide humaine notifiée) | Apprendre une consigne de santé par l'élève | Ne pas savoir réagir à une crise | Les **consignes** et la **conduite à tenir** de l'élève accompagné, et rien d'autre (§9) | quotidienne |
| **Enseignant référent (MDPH), référent handicap** | Suit les PPS, organise les équipes de suivi | Pièces médicales envoyées par erreur à l'établissement | Être destinataire de données médicales | Socle 35 ; le 34 ne lui transmet **que des avis** du médecin, sous pli | trimestrielle |
| **Assistante sociale** | Situations familiales, protection de l'enfance | — | — | Coordination avec l'infirmière pour une information préoccupante, chacun dans son dossier (socle 36) | ponctuelle |
| **Famille unie** | Remplit la fiche d'urgence, fournit ordonnances et trousse, signe le PAI, est appelée | Remplir la même fiche papier chaque année ; ne jamais savoir que l'enfant est passé à l'infirmerie | Que l'information médicale circule dans l'établissement ; ne pas être prévenue | **Fiche reconduite** (« rien n'a changé ? »), **PAI signé en ligne**, rappel « l'ordonnance expire le 30 septembre », **avis neutre** « l'infirmerie a vu Léa aujourd'hui, ouvrez l'ENT », possibilité de **joindre l'infirmerie** (boîte C3) | rentrée, ponctuelle |
| **Parent séparé** (titulaire de l'autorité) | Même chose, chacun de son côté | Apprendre le PAI par l'autre parent ; ne pas être appelé lors d'un malaise | Que l'autre lui cache un traitement ; que l'autre voie son adresse | **Ses propres avis**, **sa propre signature** du PAI, la même information que l'autre, **jamais l'adresse protégée** de l'autre (socle 4) | ponctuelle |
| **Parent sans autorité ou visé par une interdiction** | — | — | — | **Rien** de la santé ; jamais désigné pour une remise (SD-02) | — |
| **Élève mineur** | Va à l'infirmerie, parle, demande de l'aide ; peut demander le secret | Passer par la vie scolaire pour un sujet intime ; que la file d'attente voie pourquoi il vient | **Que ses parents ou ses camarades sachent** (contraception, santé sexuelle, détresse) | Écrire **discrètement** à l'infirmière (socle 16) ; **prendre rendez-vous** ; savoir **ce qui sera dit à ses parents** (« rien sur ta demande ; on peut les appeler si tu veux ») | hebdomadaire |
| **Élève majeur** | Décide seul de sa santé | Les parents continuent d'être appelés | Que ses parents lisent son dossier | **Ses parents n'ont plus accès** sauf s'il l'ouvre ; il signe son PAI ; il choisit qui prévenir | rare |
| **DPO** | Tient le registre, l'AIPD, les durées ; répond aux demandes d'accès | Découvrir un traitement de santé non déclaré | Un hébergement hors HDS ; une fuite | **Fiche de traitement générée** (finalités, données, destinataires, durées, hébergeur) ; **AIPD pré-remplie** ; journal des accès ; attestation d'activation par niveau | annuelle, incident |
| **Support ScolIA / exploitation** | Diagnostique, sauvegarde | — | Accéder à un dossier médical | **Aucun accès** au cabinet (clés hors de portée, base séparée) ; diagnostics sur métadonnées ; procédure de « bris de glace » technique tracée et soumise à l'hébergeur certifié | — |
| **Assistant IA** (au nom d'une personne) | Aide à rédiger une information générale, à préparer une campagne | — | — | **Aucun accès au cabinet** ; consignes fonctionnelles lues comme la personne les voit ; jamais de déduction de cause (§13) | — |

**Ce que montrent ces portraits [R].** (1) Le besoin le plus fréquent n'est pas médical, il est **logistique** : où est l'élève, qui le récupère, qui appeler, où est la trousse, est-ce que le PAI est à jour. C'est précisément ce que N-1 sert sans dossier de soins. (2) Le danger principal est **le partage** : un diagnostic qui fuit vers la direction, un SMS qui trahit une mineure, une remise au mauvais adulte. (3) L'infirmière veut **moins de saisie** : elle tient déjà LIEN dans le public ; ScolIA ne doit lui demander que ce qui sert l'établissement. (4) Les familles veulent **savoir sans qu'on dise tout** : un avis neutre, et la possibilité de joindre l'infirmière.

---

## 4. L'année vue de l'infirmerie [H, à confirmer avec l'infirmier référent]

| Moment | Ce qui se passe | Ce que fait ScolIA |
|---|---|---|
| **Juin–juillet** | Réinscriptions ; fiche d'urgence de l'année suivante ; PAI à renouveler ; sortants | Fiche d'urgence **reconduite** dans la campagne de réinscription (socle 3, section fournie par le 34 au socle 4) ; liste des PAI à renouveler envoyée aux familles (avis U0) ; dossiers des sortants : transmission ou destruction (S9) |
| **Fin août – septembre** | Rentrée : nouveaux élèves, PAI transmis par l'ancien établissement, ordonnances, trousses, inaptitudes EPS de l'année | **Tableau de rentrée** : PAI à rédiger / à signer / ordonnance attendue / trousse à déposer ; consignes publiées à la restauration et aux professeurs **dès la signature** ; inaptitudes EPS saisies et transmises sans motif ; statut vaccinal d'admission (provisoire 3 mois) |
| **Septembre–octobre** | Sorties d'intégration ; dépistages (bilan de la 12e année en 6e [V]) | Fiches d'urgence des sorties ; convocations aux dépistages (socle 19), comptes rendus au dossier N-2 ; avis aux familles |
| **Toute l'année** | Passages quotidiens, urgences, renvois au domicile, contraception d'urgence, écoute | Registre rapide, appel mis à jour, remise sécurisée, absences créées, avis neutres, cahier de l'infirmière |
| **Hiver** | Épidémies (grippe, gastro-entérites, gale, méningite) | Mode crise : décompte agrégé, liste de contacts depuis l'EDT (pour l'ARS), courriers types, évictions (arrêté de 1989 [Ve]) |
| **Janvier–juin** | Campagne de vaccination HPV et méningocoques en 5e–4e [Ve] | Autorisations parentales (socle 18), listes de convocation, aucun acte vaccinal enregistré par l'établissement (l'équipe vaccinale tient le sien) |
| **Printemps** | Voyages ; examens (aménagements, socle 38) ; PAP à revoir | Trousses emportées, fiches hors ligne ; avis du médecin pour les PAP (socle 35) |
| **Juin** | Rapport d'activité ; état chiffré de la contraception d'urgence (mineures / majeures) à l'autorité académique [V, D5134] | **Statistiques prêtes**, agrégées, petits effectifs masqués ; export du rapport |

---
## 5. Ce qui est réglementaire (vérifié le 2026-11-10)

Sources lues par un sous-agent ; **[V]** texte lu, **[Ve]** extrait officiel vu, **[NV]** non vérifié. Le privé sous contrat n'a **aucune source propre** sur l'organisation de sa santé scolaire (statut de l'infirmière salariée de l'OGEC, application de SAGESSE) [NV] : point à confirmer avec le DPO et un juriste.

### 5.1 Secret professionnel et partage [V]
- **CSP L1110-4** (version du 4/08/2021) — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000043895798 : le secret « s'impose à tous les professionnels intervenant dans le système de santé » ; (II) échange entre professionnels **identifiés** participant **tous** à la prise en charge, informations **strictement nécessaires** ; (III) dans une équipe de soins, informations réputées confiées à l'équipe ; **hors équipe : consentement préalable** de la personne ; (IV) information et **droit d'opposition** à tout moment ; (V) obtenir des informations en violation : 1 an, 15 000 €.
- **CSP L1110-12** (équipe de soins) — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000031919050 : même établissement de santé ou médico-social, équipe reconnue par le patient, ou organisation formalisée. *Analyse* : un collège ou un lycée n'est pas un établissement de santé ; l'infirmier et le médecin relèvent du II ou de la reconnaissance par la personne.
- **CSP R1110-1 à R1110-3, D1110-3-1 à D1110-3-3** — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006072665/LEGISCTA000006190174/ : R1110-2 liste les **non-soignants** autorisés à échanger (assistants de service social notamment) ; **les personnels de l'Éducation nationale — enseignants, CPE, direction — n'y figurent pas** ; R1110-3 : informer la personne de la nature des informations et du destinataire ; D1110-3-1 : consentement hors équipe **attesté par un support écrit remis à la personne**, valable jusqu'à retrait.
- **Code pénal 226-13** (1 an, 15 000 €) [Ve] ; **226-14** (version du 12/05/2024) [V] — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000049532171 : levée du secret pour signaler au procureur ou à la CRIP ; pour un mineur, son accord n'est pas nécessaire ; pas de responsabilité sauf mauvaise foi.

### 5.2 Hébergement de données de santé [V]
- **CSP L1111-8** (version du 1/07/2025) — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000049577902 : **certificat de conformité obligatoire** pour l'hébergement sur support numérique ; usage aux seules fins de la prestation ; restitution sans copie ; secret professionnel de l'hébergeur.
- **CSP R1111-8-8** — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000036656709 : hébergement = conservation « **pour le compte** » du responsable de traitement ; l'obligation vise ceux qui « **confient l'hébergement … à un tiers** ». *Analyse* : un établissement qui héberge lui-même n'est pas visé ; **un éditeur SaaS qui héberge pour des établissements est un tiers** : **ScolIA doit être certifié pour les activités qu'il exerce** (au minimum l'administration et l'exploitation du système, 5°), en plus du certificat de Scaleway pour l'infrastructure.
- **CSP R1111-9 à R1111-11**, modifiés par le **décret 2026-209 du 24/03/2026** — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006072665/LEGISCTA000006196138/ : six activités d'hébergement (sites physiques, infrastructure, plateforme, …, administration et exploitation, sauvegarde) ; **R1111-9-1 (depuis le 26/09/2026) : stockage exclusivement dans l'UE/EEE**, tout transfert (y compris un **accès à distance**) seulement sous décision d'adéquation ou garanties de l'art. 46 RGPD ; **R1111-11 : 15 clauses contractuelles obligatoires** et cartographie publique des transferts.
- **Référentiel HDS v2** : arrêté du 26/04/2024 (JO du 16/05/2024) — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000049537692 ; transition annoncée par l'ANS jusqu'au **16/05/2026** [Ve] ; un référentiel **v2.1** postérieur au décret 2026-209 est mentionné par l'ANS [Ve, date au JO non vérifiée].
- **Scaleway** : certification HDS annoncée **par l'éditeur** (organisme BSI ; périmètre Instances, stockage objet et bloc, Bare Metal, VPC ; contrat HDS et support Business ou Enterprise exigés) [H — source non officielle ; la liste de l'ANS https://esante.gouv.fr/produits-services/hds/liste-des-hebergeurs-certifies a refusé l'accès]. **À vérifier sur la liste de l'ANS avant tout contrat** (`MISE_EN_SERVICE` B1).

### 5.3 Données de santé et RGPD [V/Ve/H]
- **CJUE, grande chambre, C-184/20 du 1/08/2022** — https://curia.europa.eu/site/upload/docs/application/pdf/2022-08/cp220133fr.pdf : des données « susceptibles de dévoiler, **de manière indirecte** » une information sensible relèvent de l'art. 9 [V]. *Conséquence* : « repas sans arachide », « inapte EPS partiel », « trousse d'urgence » sont des **données de santé** au sens du RGPD. **Le niveau N-1 n'est pas « sans donnée de santé »** : il est « sans dossier de soins ».
- **Base légale** (*analyse* [H]) : art. 9.2.h (médecine préventive, soins) avec la garantie du secret (9.3) pour les soins, dépistages, visites ; art. 6.1.e / 6.1.c (mission d'intérêt public ; obligations légales : C. éduc. L541-1, arrêté du 3/11/2015) ; 9.2.g possible pour les mesures imposées par la loi (PAI, PPS). **Aucun texte ne tranche pour l'infirmerie scolaire** : à fixer par le DPO (Q-S2).
- **CNIL, formalités santé** — https://www.cnil.fr/fr/quelles-formalites-pour-les-traitements-de-donnees-de-sante [V] : 9.2.h mobilisée pour la prise en charge par des professionnels de santé ; pas de formalité préalable, **inscription au registre**.
- **Liste des traitements soumis à AIPD** (délibération 2018-327) — https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000037559521 [V] : vise l'accompagnement médico-social et la gestion des alertes et signalements sanitaires ou sociaux ; élèves cités comme personnes vulnérables. **AIPD obligatoire** pour ce socle [R : certitude forte].
- **Norme simplifiée CNIL NS-058** (affaires scolaires) [Ve] : fiche sanitaire non informatisée, allergies sur consentement exprès. Les normes simplifiées n'ont plus de valeur juridique depuis le RGPD [H] ; elles indiquent la **prudence historique** de la CNIL.

### 5.4 Infirmerie : textes d'organisation [V]
- **Arrêté du 4/05/2001 « SAGESSE »** (gestion des infirmeries d'EPLE, toujours en vigueur) — https://www.legifrance.gouv.fr/loda/id/LEGITEXT000030312547 : données (identité, antécédents, vaccinations, soins et traitements, responsable légal) ; **destinataires : chef d'établissement, CPE, professeurs reçoivent seulement le nom et le prénom** (mouvements à l'infirmerie, dispenses) ; **la responsabilité des données couvertes par le secret incombe exclusivement à l'infirmier** ; conservation **jusqu'à la fin de la scolarité dans l'établissement, puis destruction** ; droit d'accès auprès de l'infirmier, et pour le mineur, informations médicales par un médecin désigné. *Statut* : arrêté d'un traitement ministériel ancien ; vaut au moins **référence de minimisation** pour ScolIA, y compris dans le privé [H].
- **Arrêté du 10/09/2001 « MEDSI »** (médecins) : conservation pendant la scolarité [Ve].
- **Circulaire 2015-119 du 10/11/2015** (missions des infirmiers, BO n° 42) — https://www.education.gouv.fr/sites/default/files/document/BO_MEN_42_12-11-2015_496847.pdf-232608.pdf : l'infirmier « enregistre tous les actes infirmiers afin d'en garder une traçabilité **dans l'application numérique prévue à cet effet** » ; accès aux ordonnances et aux certificats de vaccination ; délivrance de la contraception d'urgence ; renouvellement de contraceptifs prescrits depuis moins d'un an ; **local garantissant la confidentialité** fourni par le chef d'établissement.
- **Logiciels nationaux** — rapport de l'Assemblée nationale n° 1228 (11/05/2023) https://www.assemblee-nationale.fr/dyn/opendata/RINFANR5L16B1228.html ; réponse ministérielle au Sénat (JO 23/08/2018) https://www.senat.fr/questions/base/2018/qSEQ180303634.html [V] : « les médecins de l'éducation nationale utilisent le logiciel **Esculape**, et les infirmiers scolaires le logiciel **Lien** », non interconnectés ; portail commun « Diapason ». Arrêté de création, hébergement, usage dans le privé : [NV].
- **Protocole national sur l'organisation des soins et des urgences**, BO hors-série n° 1 du 6/01/2000 — https://www.education.gouv.fr/bo/BoAnnexes/2000/hs1/hs1.pdf [V] : **registre spécifique** dans chaque établissement (nom, date et heure, soins et mesures d'urgence, orientation) ; **fiche d'urgence non confidentielle** remplie chaque année par les familles, informations confidentielles **« sous enveloppe fermée »** ; armoire à pharmacie fermée à clé ; aucun médicament sur prescription sans ordonnance ; trousse de premiers secours, comprenant les médicaments des élèves ayant un PAI.
- **Actes infirmiers** — décret 2025-1306 du 24/12/2025, en vigueur au 30/06/2026 — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006072665/LEGISCTA000006178628/ [V] : **R4311-14 abrogé** ; protocoles d'urgence écrits, datés, signés par un médecin, avec compte rendu annexé au dossier, désormais en **R4311-7** ; R4311-1 : traçabilité des soins dans le dossier. *Conséquence* : la circulaire 2015-119 et le protocole de 2000 citent des articles renumérotés ; ScolIA cite les textes **en vigueur** dans ses gabarits.

### 5.5 Visites, dépistages, vaccinations [V]
- **C. éduc. L541-1** (version du 29/07/2026) — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006151362/ : visite à 3–4 ans (PMI), visite pendant la 6e année ; avec l'accord du représentant légal **ou de l'élève majeur**, versement au DMP.
- **Arrêté du 3/11/2015** modifié le 20/08/2021 — https://www.legifrance.gouv.fr/loda/id/JORFTEXT000031425153 : **dépistage infirmier de la 12e année** (annexe III) ; inscription au carnet de santé, dans les « applications numériques professionnelles sécurisées » et, le cas échéant, au DMP ; CE n° 395858 du 24/11/2017 : annulation partielle des dispositions sur le partage avec les personnels.
- **CSP L3111-2** (version du 1/01/2025) — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000048701919 : 11 valences obligatoires ; preuve exigée pour l'admission ou le maintien « dans toute école » ; **R3111-8** : carnet ou attestation, **admission provisoire et vaccinations à faire dans les 3 mois** [V]. Application au second degré et règle « nés depuis le 1/01/2018 » : [NV].
- **Campagne HPV et méningocoques ACWY au collège** (5e–4e) : autorisation écrite des parents, séances de janvier à juin 2026 [Ve] — https://www.education.gouv.fr/vaccination-au-college-un-acces-simple-et-gratuit-la-vaccination-contre-les-hpv-et-les-meningocoques-467651.
- **Maladies contagieuses** : arrêté du 3/05/1989 (évictions) affiché en vigueur [Ve] — https://www.legifrance.gouv.fr/loda/id/JORFTEXT000000705286/ ; guide du HCSP du 28/09/2012 [Ve, non lu].

### 5.6 Contraception d'urgence et confidentialité du mineur [V]
- **CSP L5134-1** (version du 28/12/2023) — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000046812013 : consentement des titulaires de l'autorité parentale **non requis** ; actes **couverts par le secret** pour les mineures (le texte dit secret, pas anonymat) ; les infirmiers du second degré administrent la contraception d'urgence aux élèves mineures et majeures selon un protocole national.
- **CSP D5134-5 à D5134-10** — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006072665/LEGISCTA000006178659/ : **entretien préalable** ; proposition d'en parler aux parents, que la mineure **peut refuser** ; **compte rendu écrit, daté, signé dans le « cahier de l'infirmière »** ; **état chiffré annuel** à l'autorité académique, mineures et majeures distinguées ; local confidentiel ; suivi organisé.
- **CSP L1111-5** [Ve] et **L1111-5-1** [V] : le professionnel peut se passer du consentement parental si le mineur s'oppose à leur consultation (santé sexuelle et reproductive pour L1111-5-1), le mineur étant alors accompagné d'un majeur de son choix ; **R1111-6** [Ve] : la demande d'accès des parents ne peut être satisfaite tant que l'opposition du mineur est maintenue.
- **CSP L1111-7** (version du 1/11/2021) — https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000042685313 [V] : accès de la personne aux informations détenues par les professionnels de santé ; pour un mineur, par les titulaires de l'autorité parentale **sous réserve des oppositions** ; délais de 8 jours (2 mois au-delà de 5 ans) après 48 h de réflexion [Ve]. **Élève majeur** : il exerce seul ses droits (circulaire PAI « le jeune s'il est majeur », L541-1 « accord de l'élève majeur ») [V] ; les parents n'ont pas d'accès propre (*analyse*).

### 5.7 PAI, PAP, PPS, examens, EPS [V]
- **Circulaire PAI du 10/02/2021** (BO n° 9 du 4/03/2021, abroge la circulaire 2003-135) — https://www.education.gouv.fr/sites/default/files/imported_files/documents/BO_MENJS_9_1383666.pdf : rédigé et signé par le **médecin** (EN, PMI ou de la structure) en accord avec l'enfant, son représentant légal et le chef d'établissement ; l'organisateur du périscolaire peut signer ; la **partie spécifique** (« conduite à tenir en cas d'urgence ») **ne révèle pas le diagnostic** : seules les conséquences utiles sont connues des personnels ; **renouvellement à chaque rentrée** (ordonnance, fiche d'urgence, trousse vérifiée) ; le PAI reste valide en début d'année dans l'attente ; trousses **accessibles aux adultes, non aux élèves**, **emportées à chaque sortie** ; participation aux sorties avec ou sans nuitée précisée ; **registre des PAI** tenu par le secrétariat médico-scolaire ; sans ordonnance valide, le volet soins n'est pas appliqué et l'on appelle le 15 ; personnel non soignant : médicament seulement en exécution du PAI avec ordonnance valide. **Contradiction interne relevée** : ordonnance « valable un an au plus » d'un côté, « de moins de trois mois » fournie chaque année de l'autre (§28).
- **C. éduc. D351-9** (version du 15/08/2022) [V] : PAI avec le médecin de l'EN ou de la PMI. **D311-13** [Ve] : PAP après avis du médecin de l'EN, révisé chaque année. **D351-5 à D351-7** [V] : PPS, équipe pluridisciplinaire de la MDPH, CDAPH. **Examens** : D351-27, D351-28 (demande à un médecin désigné par la CDAPH), D351-28-1 (procédure simplifiée pour les titulaires d'un PPS, PAI ou PAP) [V].
- **EPS** : **R312-2, R312-3** — https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000006166809/ [V] : certificat médical obligatoire (public et privé sous contrat), inaptitude totale ou partielle, indications « dans le respect du secret médical », validité limitée à l'année, **médecin scolaire destinataire au-delà de 3 mois** ; **D312-4** : jamais rétroactif ; **arrêté du 13/09/1989** — https://www.legifrance.gouv.fr/loda/id/JORFTEXT000000840437 [V] : modèle de certificat en **incapacités fonctionnelles** (mouvements, efforts, situations), sans diagnostic.

### 5.8 Signalements, informations préoccupantes [V/Ve]
- **CASF L226-2-1** (transmission à la CRIP du département, parents informés sauf intérêt contraire de l'enfant) et **L226-2-2** (secret partagé, strict nécessaire) [Ve] ; **CPP art. 40** (en vigueur, abrogé au 1/01/2029 par l'ordonnance 2025-1091) [V] ; **C. éduc. L542-1, L542-3** [V]. Le circuit appartient au **socle 36** ; le 34 fournit la **saisie par un soignant** et le **pli** vers le 36 (§10.12).

### 5.9 Ce que la réglementation ne dit pas (à faire trancher)
1. L'infirmerie scolaire relève-t-elle du champ de l'HDS (« prévention, soins ») ? *Analyse* : oui pour les soins et le registre (N-2) ; **incertain pour N-1** (consignes organisationnelles saisies par l'établissement) → **avis écrit du DPO** avant toute activation de N-1 sans HDS (Q-S2), avec un juriste si besoin.
2. Durées de conservation hors SAGESSE et MEDSI ; statut de l'infirmerie du privé sous contrat ; usage de LIEN et Esculape dans le privé → DPO (Q-S6).
3. Accès des deux parents séparés au PAI : principe d'égalité des titulaires de l'autorité parentale (code civil 371-1, 373-2 [NV]) → appliqué par défaut (§11), avis juridique au premier cas conflictuel.

---

## 6. Trois niveaux d'activation et l'hébergement

### 6.1 Les trois niveaux [R]
| Niveau | Contient | Ne contient jamais | Hébergement exigé | Qui active |
|---|---|---|---|---|
| **N-0 — inactif** (défaut) | rien : les routes répondent « module désactivé » ; l'appel et la vie scolaire gardent « envoyé à l'infirmerie » comme simple **localisation** sans motif (socle 12) | toute donnée de santé | — | — |
| **N-1 — sécurité et organisation** | consignes fonctionnelles codées et datées (§10.5), fiche d'urgence (contacts, consignes, lieu de la trousse, numéro du 15 et du 112), protocoles **génériques** de l'établissement, inventaire et péremption des trousses, **mouvements** à l'infirmerie (arrivée, sortie, issue : retour en classe, repos, rentré au domicile, secours ; **sans motif ni soin**), renvoi au domicile et remise sécurisée, **registre des PAI sans contenu médical** (existence, dates, signatures, échéances, trousse), inaptitudes EPS **en incapacités fonctionnelles** (arrêté 1989), statut vaccinal d'**admission** (conforme / provisoire jusqu'au …), dépôt **sous pli** de pièces (stockées chiffrées, **non ouvrables** tant que N-2 n'est pas actif : la pièce attend dans le coffre ou n'est pas proposée, Q-S9) | soins, motifs, diagnostics, traitements, ordonnances lisibles, texte libre médical, contenu du PAI, comptes rendus | **aucune obligation HDS établie** [H] ; données de santé au sens du RGPD (art. 9, §5.3) : chiffrement, AIPD, journal, durées — **avis écrit du DPO obligatoire** avant activation | direction, avec attestation du DPO (★ N3) |
| **N-2 — cabinet infirmier** | N-1 + registre des soins complet (motif, soins, issue, orientation), cahier de l'infirmière, traitements et administrations, contraception d'urgence, contenu du PAI et ordonnances, vaccinations détaillées, dépistages et visites, comptes rendus, pièces de santé ouvrables, dossier infirmier, transmissions | données d'un autre cercle (psychologue, assistante sociale : socle 36) | **certificat HDS de toute la chaîne** : infrastructure (Scaleway HDS, contrat signé) **et** ScolIA pour l'administration et l'exploitation (ou infogérance certifiée) ; stockage UE/EEE ; aucun accès à distance hors UE | direction + infirmier référent + attestation HDS et DPO (★ N3) ; **refus serveur** si la plateforme n'est pas déclarée HDS (R-S2) |

**Public / privé (D-019) [R]** : dans un **EPLE**, N-2 est **désactivé par défaut et déconseillé** tant que l'infirmier trace ses actes dans LIEN (circulaire 2015-119 : « l'application numérique prévue à cet effet ») ; ScolIA sert N-1 et la logistique ; l'infirmier peut choisir de tenir dans ScolIA les seuls **mouvements** (N-1). Dans le **privé sous contrat** et le **hors contrat**, N-2 est l'offre complète, après HDS. Un réglage signalé (S-180) permet à un EPLE d'activer N-2 si son académie le demande.

**Ajout personas (§30.1, R-S35)** : le tableau ci-dessus se contredit pour N-0 (« ne contient jamais » de donnée de santé, mais l'appel garde « envoyé à l'infirmerie », donnée de santé indirecte). **N-0 devient « sans donnée de santé » au sens strict** : statut neutre « sortie autorisée », sans lieu ni motif ; c'est le défaut, sans HDS ni avis du DPO.

### 6.2 Architecture d'hébergement [R]
1. **Deux bases.** La base principale de l'ENT reçoit N-1 (tables `ent_san_*` de la brique `sante`, champs chiffrés). Le **cabinet** (N-2) vit dans **une base PostgreSQL séparée**, sur des instances du périmètre HDS, avec ses sauvegardes propres (activité 6° de R1111-9), sans réplique dans la base principale ; la base principale ne contient du cabinet que des **identifiants opaques** et des **états** (« passage clos », « PAI version 3 signée »).
2. **Clés.** Chiffrement par champ avec une **clé par établissement et par niveau** (enveloppe : clé de données chiffrée par une clé maîtresse dans le KMS du périmètre HDS) ; la clé N-2 n'est **jamais** chargée par le processus principal : un **service cabinet** distinct (même code Rust, binaire séparé ou crate `sante-cabinet` déployé à part) la détient. Destruction de la clé d'un établissement = effacement cryptographique (fin de contrat).
3. **Pièces.** Ordonnances et certificats N-2 : stockage objet du périmètre HDS, **chiffrement applicatif par objet** (en plus de celui du fournisseur), URL pré-signée de 60 s émise après contrôle de droit et trace.
4. **Exploitation.** Aucun accès humain aux bases du cabinet hors procédure de l'hébergeur certifié (bris de glace technique à deux personnes, tracé, notifié au DPO de l'établissement) ; journaux techniques **sans contenu** (identifiants opaques).
5. **Tant que N-2 n'existe pas**, le code du cabinet n'est pas déployé ; les tests tournent sur des données fictives seulement (R-S33).
6. **Coût** : contrat HDS Scaleway (support Business ou Enterprise [H]), **certification de ScolIA** (audit annuel par un organisme certificateur, système de management ISO 27001 sous-jacent [H]) ou **infogérance** par un prestataire certifié pour l'activité 5° : **décision et budget soumis à Florian** (Q-S1).

---

## 7. Frontières

### 7.1 Qui possède quoi
| Donnée | 34 Santé | 35 Inclusion | 36 Psy, social, protection | 12 Appel | 4 Dossier administratif | 7 Documents | Autres lecteurs |
|---|---|---|---|---|---|---|---|
| Mouvements et passages à l'infirmerie | **possède** | — | — | lit la **localisation** (« à l'infirmerie ») | — | — | 14 (sortie), 8 (avis) |
| Registre des soins, cahier, traitements (N-2) | **possède** | — | — | — | — | pièces au coffre C3 | — |
| PAI (contenu, versions, signatures, trousse) | **possède** | lit l'existence (pastille) | — | — | lit pour la remise ? non | dépôt, signature S2 | 40, 41, 42 lisent les **consignes** |
| Consignes fonctionnelles de santé | **possède** | — | — | — | — | — | 40, 41, 42, 10 (AESH), 21, 38, professeurs |
| Inaptitudes EPS (médicales) | **possède** | dispense d'enseignement (décision du recteur, PPS) | — | lit `dispense` **sans motif** | — | certificat C3 | 21 (statut « dispensé »), 10 (CL-33) |
| PAP, PPS, PPRE, ULIS, aménagements pédagogiques et d'examen, AESH, équipes de suivi | **fournit l'avis médical** (PAP : D311-13) | **possède** | — | — | — | — | 21, 38, 10 |
| Dossier du psychologue, service social, informations préoccupantes | rédaction par un soignant possible, **pli** vers le 36 | — | **possède** | — | — | — | — |
| Contacts d'urgence, personnes autorisées, interdictions | lit | — | — | — | **possède** | — | 14 |
| Fiche d'urgence annuelle (section de la fiche de rentrée) | **fournit la section** et ses règles | — | — | — | **possède** la campagne (fiche de rentrée) | — | 3 (réinscription) |
| Autorisations parentales (vaccination HPV, soins en voyage) | consomme | — | — | — | — | — | 18 **possède** |
| Sorties et voyages | fournit fiches et trousses | — | — | — | — | — | 42 **possède** |
| Absence créée par un renvoi au domicile | demande | — | — | **possède** | — | — | — |

### 7.2 Règles de frontière [R]
1. **Le 35 ne reçoit jamais un diagnostic** : il reçoit l'**avis** du médecin (« avis favorable au PAP ») et les consignes ; le PPS et la notification MDPH (pièces médicales) sont des documents **C3 du domaine `sante`** déposés au coffre, ouverts par le cercle de soins et, pour la notification MDPH sans diagnostic, par le référent désigné (Q-S12).
2. **Les tables 212 (dispositifs, aménagements, AESH, réunions) passent au socle 35** à sa conception ; d'ici là, le 34 les **garde en l'état** avec les correctifs S0 (SD-21) et ne les étend pas.
3. **Le dossier du psychologue reste au 36** ; une coordination infirmière–psychologue est un **échange consenti** (D1110-3-1), pas un accès croisé.
4. **L'appel ne connaît que la localisation** : `a_l_infirmerie`, `rentre_domicile` (avec heure) ; jamais le motif ; un passage confidentiel (contraception d'urgence) **ne laisse pas de trace de localisation visible de la famille** (§10.10).
5. **Aucun socle n'écrit dans les tables santé** ; ils appellent les ports du 34 (D-026).

### 7.3 Ports exposés par le socle 34 [R]
- `sante::consignes(eleve, audience, date) -> [Consigne]` (code, libellé fonctionnel, lieu de trousse ; **jamais la source**).
- `sante::consignes_du_jour(etab, audience, date)` (restauration, internat, encadrants) — liste opérationnelle.
- `sante::fiche_urgence(eleve, niveau, contexte)` (minimale : encadrant ; complète : cercle de soins).
- `sante::localisation(eleve, instant) -> {a_l_infirmerie depuis | rentre_domicile a | aucune}` (12, 14, 29).
- `sante::inaptitude(eleve, matiere, date) -> {totale | partielle(incapacites) | aucune}` (12, 21, 38) — **sans motif**.
- `sante::pai_existe(eleve) -> {actif, a_renouveler, aucune}` (pastille, 2 ; 42 pour la préparation).
- `sante::trousses(sortie | eleves) -> [Trousse]` (42).
- `sante::avis_medical(eleve, dispositif)` (35) — avis sans motif.
- `sante::statistiques(etab, periode)` (47) — agrégats, petits effectifs masqués.

### 7.4 Ports consommés [R]
`eleves` (2 : identité réduite, photo, classe, majorité, décès, fusion), `responsables`, `joignables`, `remise` et `interdictions` (4), `cours_du_moment` et `seances` (10 : « de quel cours vient-il »), `appel.localiser` et `absence.creer` (12), `sortie.preparer` (14), `documents.deposer|signer|coffre` (7), `notifications.avis` (8), `messagerie.boite` (16), `rendez_vous` (19), `autorisations` (18), `sorties` (42), `agents` et `fonctions` (6 : infirmier, médecin, remplaçant).

---

## 8. Concepts et vocabulaire

### 8.1 Vocabulaire fixé [R]
| Terme | Sens | Remplace |
|---|---|---|
| **Cercle de soins** | ensemble des personnes autorisées à lire le cabinet d'un établissement : infirmier(s) titulaire(s), remplaçant désigné **pour une période**, médecin scolaire rattaché, médecin auteur d'un PAI pour ce PAI ; défini par fonction du socle 6, jamais par le rôle « direction » | rôle `infirmerie` + `can_saisir_infirmerie` |
| **Mouvement** | arrivée et sortie d'un élève à l'infirmerie, avec issue ; **sans motif** (N-1) | partie de `ent_infirmerie_passage` |
| **Passage** | mouvement + motif, soins, orientation, compte rendu (N-2) | `ent_infirmerie_passage` |
| **Consigne** | conséquence fonctionnelle d'un état de santé, code d'un catalogue fermé, audience, dates, source cachée | « contrainte publiée » (`ent_contrainte_publiee`) |
| **Conduite à tenir** | partie du PAI qui dit quoi faire en cas d'urgence, **sans diagnostic** (circulaire 2021) ; lisible des adultes désignés de l'élève en N-2, sur papier dans la trousse en N-1 | `protocole_urgence` du PAI |
| **PAI** | projet d'accueil individualisé : **versions immuables** (contenu chiffré N-2, empreinte), signatures par partie **et par titulaire**, échéance, trousse(s), consignes dérivées | `ent_pai` (ligne unique) |
| **Trousse** | contenant physique identifié (« sac rouge Léa »), lieu, contenu non médical (« trousse PAI »), péremption, emportée ou non | colonnes `trousse_*` |
| **Inaptitude** | incapacité fonctionnelle à l'EPS (totale, partielle : mouvements, efforts, situations), datée, d'après certificat, **sans diagnostic** | dispense `medical` |
| **Acte confidentiel** | acte du cabinet couvert par le secret vis-à-vis des titulaires (contraception d'urgence, opposition du mineur) | — (absent) |
| **Pli** | pièce déposée par une famille ou un tiers, chiffrée, ouvrable du seul cercle de soins (l'« enveloppe fermée » du protocole de 2000) | pièce `documents.lire_sante` |
| **Niveau** | N-0, N-1, N-2 (§6) | booléen `ent_module_etablissement.actif` |

### 8.2 Quatre distinctions qui manquent aujourd'hui [R]
1. **Où est l'élève** (localisation, partagée) ≠ **pourquoi il y est** (motif, secret).
2. **Ce qu'il faut faire** (consigne, conduite à tenir) ≠ **ce qu'il a** (diagnostic).
3. **Ce que l'établissement organise** (N-1) ≠ **ce que le soignant soigne** (N-2).
4. **Ce que la famille sait** ≠ **ce que le mineur a confié** (acte confidentiel).

---

## 9. Secret et partage strictement minimal

### 9.1 Matrice de partage [R]
| Information | Cercle de soins | Direction | CPE / VS | Professeur de l'élève | Professeur EPS | Restauration / internat | Encadrant de sortie | AESH de l'élève | Secrétariat | Accueil | Titulaires | Élève |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Soins, motifs, diagnostic, traitements, comptes rendus | ✔ | — | — | — | — | — | — | — | — | — | sur demande d'accès (L1111-7), sauf actes confidentiels | ✔ (le sien, §11) |
| Contenu du PAI (hors conduite à tenir) | ✔ (+ médecin auteur) | — | — | — | — | — | — | — | — | — | ✔ (signataires) | ✔ majeur ; mineur : en accord avec lui |
| Conduite à tenir du PAI | ✔ | lecture pour signer | si désigné | ✔ si le PAI le prévoit | ✔ | ✔ (alimentaire) | ✔ (sortie) | ✔ | — | — | ✔ | ✔ |
| Existence d'un PAI (pastille) | ✔ | ✔ | ✔ | ✔ | ✔ | si alimentaire | ✔ | ✔ | — | — | ✔ | ✔ |
| Consignes fonctionnelles | ✔ | agrégées | selon audience | audience « professeurs » | ✔ | audience « restauration » | audience « sorties » | ✔ | — | — | ✔ | ✔ |
| Mouvement (à l'infirmerie, rentré) | ✔ | ✔ (agrégé + élève) | ✔ | ✔ (son cours) | ✔ | — | — | ✔ | — | sortie seulement | issue utile (rentré, secours), **jamais** pour un acte confidentiel | ✔ |
| Inaptitude EPS (incapacités) | ✔ | — | — | « dispensé » sans motif | ✔ incapacités | — | si sortie sportive | ✔ | — | — | ✔ | ✔ |
| Statut vaccinal d'admission | ✔ | ✔ | — | — | — | — | — | — | ✔ (conforme / provisoire) | — | ✔ | ✔ |
| Pièce sous pli | ✔ | — | — | — | — | — | — | — | « reçue » | — | ✔ (déposant) | majeur |

### 9.2 Règles du partage [R]
1. **Besoin d'en connaître, par audience, par période.** Une consigne a une **audience** (restauration, professeurs de l'élève, encadrants d'une sortie donnée, AESH, internat de nuit) et des **dates** ; elle n'est jamais listée « pour tout l'établissement » hors du cercle de soins (corrige SD-17).
2. **Consentement attesté** pour tout partage d'une information de soins hors cercle (D1110-3-1) : document remis à la famille ou à l'élève majeur, journalisé, révocable ; ScolIA propose le **formulaire** et le garde au coffre C3.
3. **La direction signe sans lire le diagnostic** : l'écran de signature du PAI montre la conduite à tenir, les mesures d'organisation, les trousses, les signataires ; le diagnostic et les traitements restent dans la partie réservée au cercle.
4. **Pas de rôle « supérieur ».** Ni `securite.politique`, ni l'administrateur, ni la direction n'ouvrent le cabinet ; le **bris de glace** (R-E11 du socle 2) **n'ouvre jamais** N-2 ; seul un membre du cercle peut ouvrir un accès d'urgence (remplaçant non désigné, SAMU au téléphone), motivé, 2 h, signalé au titulaire (SAN-231).
5. **Agrégats sans réidentification** : tout tableau hors cercle masque les effectifs inférieurs à 5 (S-187) et ne croise jamais classe × motif.

### 9.3 Formulation des consignes [R]
Le catalogue fermé existant est gardé ; ses libellés deviennent des **instructions** à l'adulte (« Repas : servir sans arachide ni fruits à coque ; trousse : bureau du chef de cuisine ») plutôt que des **états** de l'enfant (« allergie arachide »). La donnée reste de santé (C-184/20), mais l'adulte reçoit ce qu'il doit **faire**, l'élève n'est pas étiqueté, et l'écran ne pousse pas à demander « pourquoi ». Ajouts au catalogue (S2) : `prevenir_infirmerie_malaise`, `pas_de_jeune_force` (examens, sorties — à valider), `sortie_avec_trousse`, `nuitee_avec_trousse`, `ne_pas_laisser_seul_en_crise`, `appeler_15_d_abord`, `place_proche_sortie`, `exposition_soleil_limitee`, `pas_de_piscine`, `accompagnement_escaliers`. Toute addition passe par le cercle de soins référent du produit (Q-S3) ; **aucun code ne nomme une maladie** (règle de lint du catalogue, R-S4).

---

## 10. Les domaines un par un

### 10.1 Mouvements et registre des passages [R]
- **Arrivée** : par badge, photo, recherche en trois lettres ; « envoyé par » (professeur de la séance en cours, lu dans le socle 10) ou « venu seul » ; heure automatique ; l'appel de la séance passe l'élève en **« à l'infirmerie »** (présent, localisé) — le professeur le voit, la famille n'est pas prévenue d'une absence (corrige SD-20).
- **Issue** : retour en classe (avec billet de retour numérique visible du professeur), repos (durée), famille appelée, **rentré au domicile** (remise, §10.3), secours appelés (15, 112 : heure d'appel, heure d'arrivée, hôpital d'orientation en N-2), transport par la famille vers un médecin.
- **N-2** : motif (catalogue élargi : douleur, malaise, traumatisme, écoute, suivi de traitement, PAI, santé sexuelle, autre), soins, compte rendu ; **ajout seulement** (on complète, on ne réécrit pas : chaque correction est une nouvelle entrée liée, R-S9) ; signature horodatée de l'infirmière (le « cahier »).
- **Registre de l'établissement** (protocole 2000) : en N-1, le registre des **mouvements et urgences** (nom, date, heure, mesure d'urgence en code, orientation) ; en N-2, le registre complet. Export papier signé pour l'inspection.
- **Absence de l'infirmière** : mode « infirmerie fermée » — la vie scolaire ne peut pas envoyer d'élève ; consigne affichée (appeler le 15 pour une urgence, CPE pour un malaise léger) ; un AED peut saisir un **mouvement de premiers secours** (sans soin) visible ensuite de l'infirmière.

### 10.2 Dossier infirmier (N-2) [R]
Antécédents utiles, allergies, traitements au long cours, vaccinations détaillées, dépistages, PAI, passages, comptes rendus, pièces ; **propositions** de la famille (fiche d'urgence, allergies déclarées) **validées** par l'infirmière, jamais écrasantes (corrige SD-15) ; **transmissions** entre infirmières (remplacement, changement d'établissement, avec accord) ; **lecture seule du médecin** sauf ses propres écrits ; **dossier de liaison** à la sortie de l'établissement (§10.14).

### 10.3 Renvoi au domicile et remise sécurisée [R]
1. L'infirmière choisit « faire venir la famille » : ScolIA appelle (ou affiche) les **joignables** du socle 4 dans l'ordre, journal de chaque appel ; avis U3 `sante.rappeler_infirmerie` (« merci de rappeler l'infirmerie de [établissement] »).
2. À l'arrivée : écran de **remise** du socle 4 — photo de l'élève, personnes **autorisées aujourd'hui**, **interdictions en rouge** (corrige SD-02), pièce d'identité contrôlée (case), signature sur tablette ou « remis à » ; la personne visée par une interdiction **n'est pas proposée** et sa venue déclenche la consigne (« appeler la direction »).
3. La sortie crée **l'absence** des séances restantes (socle 12, motif « santé — rentré au domicile », motif **visible de la vie scolaire comme « santé »** sans détail ; R-S16) et la **sortie** (socle 14).
4. Élève majeur : peut partir seul si son état le permet (décision de l'infirmière, consignée) ; les parents ne sont pas prévenus sans son accord (R-S13).
5. Interne : remise à l'internat ou à la famille selon le règlement ; consigne de nuit.

### 10.4 Traitements et médicaments (N-2) [R]
- **Traitement ponctuel** apporté par la famille avec ordonnance : enregistré (nom commercial, posologie, période), rangé dans l'armoire fermée (protocole 2000), **administrations tracées** (heure, dose, par qui ; refus de l'élève) ; alerte de fin de traitement.
- **Traitement au long cours / PAI** : prises à l'infirmerie planifiées, rappel à l'infirmière, absence de l'élève signalée.
- **Médicaments de l'infirmerie** (protocole national, R4311-7 en vigueur) : inventaire, lots, péremptions, sorties par administration ; commande ; **pas de gestion de stock pharmaceutique** au-delà (réglage S-183).
- **Personnel non soignant** : n'administre qu'en exécution d'un PAI avec ordonnance valide ; ScolIA affiche « ordonnance valide jusqu'au … » à côté de la conduite à tenir, et **« ordonnance expirée : appeler le 15 »** quand elle ne l'est plus (circulaire 2021 [V]).

### 10.5 Consignes fonctionnelles, fiche d'urgence, protocoles, trousses (N-1) [R]
- **Consignes** : publiées par un membre du cercle, depuis un PAI (dérivées, proposées à la signature et publiées **à la signature** de la version), un certificat (inaptitude) ou à la main (source `manuelle`, motif obligatoire, durée maximale 3 mois sans PAI, S-184) ; audiences fermées ; **retrait** daté ; historique.
- **Fiche d'urgence** : minimale (encadrant, AESH, internat) — photo, classe, date de naissance, responsables **joignables et autorisés** (sans interdits, sans adresse protégée), consignes de l'audience, trousse et lieu, conduite à tenir (N-2) ou mention « conduite à tenir : dans la trousse » (N-1), numéros d'urgence ; complète — cercle de soins.
- **Protocoles génériques** de l'établissement (malaise, plaie, crise d'asthme type, anaphylaxie type) : écrits par l'infirmière, **validés par le médecin** (R4311-7 [V]), versionnés, signés, affichables ; **aucun nom d'élève**.
- **Trousses** : chaque trousse a un identifiant, un propriétaire (élève, ou « trousse collective de premiers secours »), un lieu, un contenu **non médical** (« trousse PAI — Léa M. »), une **date de péremption** (alerte J-30 à l'infirmière et à la famille, U0), un état (présente, sortie avec [sortie], manquante) ; trousse collective : inventaire de premiers secours (protocole 2000).

### 10.6 PAI [R]
- **Cycle** : demande (famille ou direction, D351-9 [V]) → préparation par l'infirmière (modèle national de la circulaire 2021, sections fixes) → **rédaction et signature du médecin** → signatures : **chaque titulaire** de l'autorité parentale (un suffit pour la validité, l'autre est informé et peut signer, Q-S10), élève (selon l'âge, majeur obligatoire), chef d'établissement, organisateur du périscolaire ou collectivité si concernée → **version figée** (empreinte, signature S2 du socle 7) → consignes publiées, trousses attendues, conduite à tenir diffusée aux adultes désignés.
- **Versions** : toute modification crée une **nouvelle version** ; les anciennes restent lisibles du cercle et des signataires (preuve) ; l'ancienne reste en vigueur jusqu'à la signature de la nouvelle (sauf retrait motivé par le médecin) ; une coquille corrigée par le médecin ne demande pas une nouvelle signature des familles **si et seulement si** le médecin la qualifie « sans effet sur la conduite à tenir » (S-185, désactivé par défaut).
- **Renouvellement** : à chaque rentrée [V] ; le PAI reste valide en début d'année dans l'attente des pièces [V] ; tableau « à renouveler », avis aux familles en juin et en septembre, **ordonnance attendue** avec échéance ; sans ordonnance valide, le volet soins est suspendu et la fiche dit « appeler le 15 » [V].
- **Registre des PAI** (circulaire 2021 [V]) : existence, dates, versions, signatures, échéances — N-1 ; contenu — N-2.
- **Sorties** : le PAI dit « sorties sans nuitée / avec nuitée / avec conditions » ; le socle 42 le lit au moment de la préparation (§10.7).
- **Changement d'établissement** : PAI de l'ancien établissement **déposé sous pli** par la famille ou transmis par l'ancien cercle de soins avec accord ; reconduit provisoirement, revu par le médecin.
- **Pièces** : ordonnances, certificats, compte rendu d'allergologue → **pli C3** (socle 7), plafond 20 Mo, conversion (D-005), N-2 seulement pour l'ouverture (corrige SD-13).

### 10.7 Sorties et voyages (avec le socle 42) [R]
- À la préparation : liste des participants → **récapitulatif santé de la sortie** pour l'organisateur : trousses à emporter (« 2 trousses PAI : Léa M., Tom R. »), consignes de l'audience « sorties », PAI qui **interdisent** ou conditionnent la sortie (« nuitée : accompagnement requis ») → alerte à l'infirmière et à l'organisateur ; **jamais** le diagnostic.
- Au départ : trousses **cochées « emportée »** par l'encadrant (scan du code de la trousse, socle 11) ; fiche d'urgence **hors ligne** dans l'appli de l'encadrant (chiffrée sur l'appareil, clé liée à la session, **effacement automatique** au retour + 24 h ou à la clôture de la sortie, SEJ-08) ; numéros d'urgence du pays.
- Soins pendant la sortie : l'encadrant saisit un **mouvement de premiers secours** (heure, mesure, famille prévenue) transmis à l'infirmière au retour ; aucun texte libre médical.
- Retour : trousses **rendues** (état), effacement des fiches, bilan.

### 10.8 Inaptitudes à l'EPS [R]
- Saisie par l'infirmière ou le secrétariat **à partir du certificat** (modèle de 1989 : incapacités fonctionnelles cochées, dates) [V] ; le certificat va **sous pli** ; validité ≤ fin d'année scolaire ; **jamais rétroactive** (D312-4 [V]) ; au-delà de **3 mois**, avis au médecin scolaire (R312-3 [V]).
- Le professeur d'EPS voit **totale / partielle + incapacités** (« pas de course, pas de saut, natation interdite ») et les dates ; les autres professeurs ne voient rien ; l'appel affiche « dispensé » (socle 12) ; le 21 pré-remplit « dispensé » **sans motif** ; le DNB et le LSU lisent le statut (21, 22).
- Distinction : **inaptitude médicale** (34) ≠ **dispense d'enseignement** (décision, 35) ≠ **dispense ponctuelle** de séance demandée par la famille (12, sans certificat, le professeur décide).

### 10.9 Vaccinations [R]
- **N-1** : statut d'admission (conforme / provisoire jusqu'au [date + 3 mois] / non conforme) saisi par le secrétariat à l'inscription d'après le carnet (R3111-8 [V]) ; le détail des valences n'est **pas** saisi en N-1 ; alerte à la direction à l'échéance provisoire.
- **N-2** : valences et dates (lecture du carnet par l'infirmière), rappels conseillés, mise à jour au dépistage.
- **Campagnes** (HPV, méningocoques) : ScolIA gère l'**information**, les **autorisations** (socle 18), les **listes de convocation** et les créneaux (socle 19) ; l'acte vaccinal est tracé par l'équipe vaccinale (centre, ARS), **pas** par l'établissement ; l'établissement ne connaît que « vacciné en séance : oui / non » si la famille l'autorise (Q-S16).
- **Éviction** (maladie contagieuse) : §10.13.

### 10.10 Contraception d'urgence et confidentialité du mineur (N-2) [R]
- **Acte confidentiel** par construction : marqué à la création (case « secret vis-à-vis des titulaires », pré-cochée pour la contraception d'urgence et la santé sexuelle, R-S12).
- **Effets** : aucun avis à la famille ; **aucune localisation** transmise à l'appel (l'élève est marqué « présent » si elle vient d'un cours, ou rien ; s'il faut justifier une absence, le motif est « rendez-vous » neutre choisi par l'infirmière, S-186) ; aucune ligne dans la chronologie de l'élève hors cercle ; exclu des **demandes d'accès** des titulaires tant que l'opposition est maintenue (R1111-6 [Ve]) ; exclu de tout export, de tout résumé, de l'IA ; dans les statistiques, seulement l'**état chiffré** (mineures / majeures) ; journal des accès consultable par l'infirmière, jamais par la direction.
- **Contenu** : entretien préalable, proposition d'en parler aux parents et réponse, délivrance (heure, spécialité), orientation (centre de santé sexuelle, médecin), suivi programmé (D5134 [V]) ; compte rendu signé dans le cahier.
- **Accompagnement** par un majeur de son choix (L1111-5-1) : nom consigné dans le dossier, sans compte.
- **État chiffré annuel** à l'autorité académique : généré, **aucune donnée nominative**.

### 10.11 Dépistages, visites et bilans (N-2) [R]
Bilan infirmier de la 12e année (annexe III de l'arrêté de 2015 [V]) : liste des élèves concernés (6e), convocations (socle 19), information des familles, compte rendu, **inscription au carnet** remise à la famille, orientation ; visites médicales demandées (examen à la demande de la famille, de l'équipe, de l'élève) ; versement au DMP **seulement avec accord** du représentant légal ou de l'élève majeur (L541-1 [V]) — **pas d'interface DMP** au premier temps (S13).

### 10.12 Signalements et informations préoccupantes [R]
Un soignant qui constate un danger rédige l'**information préoccupante** ou le **signalement** dans le **socle 36** (circuit, CRIP, procureur, accès très restreint) ; le 34 fournit le bouton « rédiger une information préoccupante » depuis un passage, qui **crée un pli** vers le 36 sans copier le dossier, et l'**exception au secret** (226-14 [V]) est rappelée à l'écran ; aucune information à la famille par ScolIA (L226-2-1 : « sauf intérêt contraire de l'enfant » — décision humaine) ; aucun avis automatique (R-N58 du socle 8).

### 10.13 Crises sanitaires et maladies contagieuses [R]
- **Mode crise** (direction + infirmière) : déclaration (épidémie, cas grave, canicule, intoxication alimentaire), **décompte agrégé** (cas par jour, par niveau, sans nom hors cercle), **liste des contacts** d'un cas (élèves des mêmes cours et groupes, transport, internat ; socle 10, 12, 41) **produite pour l'ARS** à la demande de celle-ci, exportée chiffrée, journalisée ; **le cas index n'est jamais nommé** dans les courriers.
- **Évictions** (arrêté de 1989 [Ve]) : l'infirmière enregistre une éviction (durée, retour conditionné à un certificat ou non) ; l'appel reçoit « absent — motif santé » sans maladie ; la famille reçoit la notification ; retour avec certificat sous pli.
- **Courriers types** (méningite, gale, coqueluche, pédiculose, varicelle) **validés par le médecin** et l'ARS, diffusion urgence du socle 8 (N3, aperçu, coût).
- **Canicule, pollution** : consignes générales (eau, activités) diffusées ; élèves à consigne `eviter_chaleur` listés pour les professeurs concernés.
- **PPMS** (risques majeurs, confinement) : socle 14 ; le 34 fournit la **trousse PPMS** et la liste des élèves à trousse / consigne pour la zone de confinement.

### 10.14 Conservation, accès, transferts (N-2) [R]
- **Demande d'accès** de l'élève majeur, des titulaires pour un mineur (sous réserve des oppositions), ou du mineur par un médecin désigné (SAGESSE, L1111-7 [V]) : enregistrée, délai affiché (8 jours, 2 mois au-delà de 5 ans), 48 h de réflexion, dossier exporté **par l'infirmière** (jamais automatique), actes confidentiels exclus tant que l'opposition tient.
- **Sortie de l'établissement** : proposition de **dossier de liaison** (PAI, vaccinations, inaptitude en cours) remis à la famille ou transmis à l'infirmière du nouvel établissement **avec accord** ; le reste est **détruit** à l'échéance (§18).

### 10.15 Messagerie de l'infirmerie (avec le socle 16 ; point ouvert Q-M21) [R]
- **Boîte « infirmerie »** C3 (socle 16, R-M11) : membres = cercle de soins ; invisible de la vie scolaire, de la direction et des parents (pour les fils de l'élève) ; aucune supervision ordinaire, aucune recherche transversale.
- **En N-0 et N-1** : la boîte existe pour la **logistique** (rendez-vous, retour à la maison, PAI à renouveler, « je voudrais te parler ») avec un **avertissement permanent** « pas de détail médical ici » et un **lint doux** (mots de santé repérés → rappel) ; ses messages sont stockés dans la base principale (non HDS) [R, appliqué par défaut, **soumis au DPO** comme le recommande Q-M21].
- **En N-2** : les fils de la boîte infirmerie sont **stockés dans le cabinet** (périmètre HDS) ; les pièces jointes deviennent des plis.
- **Élève mineur** : écrit discrètement à l'infirmière (MSG-128) ; ses parents ne voient pas le fil ; il voit à l'ouverture ce que l'infirmière peut dire à ses parents et ce qu'elle doit signaler (danger).

### 10.16 Statistiques et rapport d'activité [R]
Passages par jour, motif agrégé (N-2), issue, heure, niveau ; renvois au domicile ; appels au 15 ; PAI actifs, à renouveler ; trousses valides ; contraception d'urgence (état chiffré) ; dépistages réalisés ; **petits effectifs masqués** hors cercle ; export du **rapport annuel** de l'infirmière (modèle académique [NV]) ; indicateurs pour la direction et le pilotage (socle 47) **sans aucun nom**.

---

## 11. Familles, mineur, majeur

1. **Titulaires de l'autorité parentale** (socle 4, `lien` actif avec autorité) : chacun reçoit les avis de santé, lit le PAI, la fiche d'urgence, les consignes, l'issue des mouvements ; chacun peut signer le PAI ; **chacun pour soi** (aucune adresse, aucun numéro de l'autre ; R-S17).
2. **Parent sans autorité** (retrait, délégation) ou visé par une **interdiction** (judiciaire ou de communication) : **rien** de la santé ; jamais proposé à la remise (R-S18).
3. **Personne de confiance** désignée par un titulaire (socle 8 R-N60) : peut venir chercher l'élève si elle est **autorisée à la remise** ; ne reçoit **aucune** information de santé.
4. **Désaccord entre titulaires** (l'un refuse le PAI) : le PAI est un acte usuel ? [NV] — **recommandation** : un titulaire suffit à la mise en place de mesures de sécurité ; l'opposition écrite de l'autre est enregistrée et transmise au médecin et à la direction, qui décident ; ScolIA **ne tranche pas** (Q-S11).
5. **Élève mineur** : voit sa fiche d'urgence, ses consignes, son PAI (en accord avec lui, selon son âge), ses rendez-vous ; ne voit pas les notes de l'infirmière ; peut demander le secret (L1111-5 [Ve]) : l'infirmière enregistre l'**opposition** (date, portée), qui rend l'acte confidentiel.
6. **Élève majeur** (événement `eleve.majeur` du socle 2) : à minuit du jour anniversaire, les titulaires **perdent** l'accès au dossier de santé, aux avis de santé, aux signatures ; l'élève reçoit un message explicatif ; il peut **rouvrir** un accès à un parent (portée : avis, PAI, tout), révocable, journalisé ; il signe seul son PAI (une nouvelle version est proposée à ses 18 ans) ; ses contacts d'urgence restent ceux qu'il choisit (R-S13).
7. **Décès** d'un élève : arrêt immédiat des avis (socle 8), gel du dossier, conservation selon §18, aucune purge précipitée (contentieux).

---

## 12. Cas limites (catalogue)

### 12.1 Personnes et familles
| # | Cas | Réponse [R] |
|---|---|---|
| CS-01 | Le père visé par une **interdiction de remise** se présente pour récupérer son fils malade | non proposé à la remise ; consigne en rouge « appeler la direction » ; l'infirmière garde l'enfant ; appel à la mère (SD-02) |
| CS-02 | Parents séparés, la mère seule a déposé le PAI ; le père découvre la trousse | le père, titulaire, reçoit l'avis « PAI de Léa signé » et peut le lire et le signer ; il ne voit ni l'adresse ni le téléphone de la mère |
| CS-03 | Les deux titulaires sont en désaccord sur le PAI | mesures de sécurité mises en place sur la signature d'un titulaire ; opposition de l'autre enregistrée, transmise ; décision humaine (Q-S11) |
| CS-04 | Le beau-père (sans autorité) vient chercher l'enfant | seulement s'il est **autorisé à la remise** par un titulaire ; aucune information de santé |
| CS-05 | Élève devenu majeur en cours d'année avec un PAI | nouvelle version proposée à sa signature ; l'ancienne reste en vigueur ; parents informés **une fois** de la perte d'accès, sans détail |
| CS-06 | Élève majeur qui veut que sa mère continue d'être appelée | il ouvre l'accès « avis de santé » à sa mère ; révocable |
| CS-07 | Mineure qui vient pour une contraception d'urgence pendant le cours de maths | acte confidentiel ; l'élève reste « présente » pour le professeur (aucune localisation publiée) ou l'infirmière choisit un billet neutre ; aucun avis ; aucune trace hors cercle |
| CS-08 | Les parents de cette mineure demandent l'accès à son dossier infirmier | l'acte confidentiel est exclu tant que l'opposition est maintenue ; l'infirmière traite la demande ; aucun signe de l'exclusion dans l'export |
| CS-09 | Mineur qui demande « ne dites rien à mes parents » sans motif légal de secret | l'infirmière évalue ; ScolIA propose « opposition du mineur » (L1111-5) seulement pour les actes de soins concernés ; le danger lève le secret (§10.12) |
| CS-10 | Élève placé à l'ASE : le service gardien et les parents ont des droits différents | les droits viennent du socle 4 (lien, autorité, délégation) ; l'ASE peut être signataire du PAI si elle détient l'autorité déléguée [NV] |
| CS-11 | Famille allophone | PAI et fiche d'urgence traduits par le modèle national si disponible [NV] ; traduction automatique **interdite** pour la conduite à tenir (risque vital) ; interprète proposé |
| CS-12 | Adresse protégée d'un parent | jamais dans la fiche d'urgence, jamais dans le dossier de liaison |
| CS-13 | Décès d'un élève en cours d'année | arrêt des avis, gel du dossier, trousses récupérées, consignes retirées de toutes les listes le jour même |

### 12.2 Soins, urgences, organisation
| # | Cas | Réponse [R] |
|---|---|---|
| CS-14 | Infirmerie fermée (infirmière absente, poste vacant) | mode « fermée » : envoi bloqué ; consigne d'urgence ; mouvements de premiers secours par la vie scolaire, transmis à la remplaçante |
| CS-15 | Remplaçante non désignée qui doit ouvrir un dossier en urgence | accès d'urgence du cercle (SAN-231), motif, 2 h, signalé au titulaire ; impossible pour un non-soignant |
| CS-16 | Appel au 15 : le régulateur demande le traitement de l'élève | la remplaçante ou l'infirmière lit le dossier N-2 ; en N-1, la conduite à tenir est **dans la trousse** ; aucun accès de la vie scolaire au dossier |
| CS-17 | Élève emmené à l'hôpital par les pompiers sans parent | mouvement « secours », heure, hôpital ; avis U3 aux joignables ; un adulte accompagne si possible (décision de la direction) ; absence créée |
| CS-18 | Élève qui part de l'infirmerie sans autorisation | mouvement « parti sans autorisation » ; alerte immédiate à la vie scolaire (U3 interne) ; procédure du socle 14 |
| CS-19 | Élève qui vient tous les jours à la même heure avant un cours | signal **déterministe** pour l'infirmière seule (« 6 passages avant l'EPS en 3 semaines ») ; jamais visible du professeur ; jamais de score |
| CS-20 | Interne malade la nuit | surveillant de nuit : mouvement de premiers secours, appel au 15 ou à l'infirmière d'astreinte [H] ; consignes de nuit de l'élève visibles |
| CS-21 | Trousse PAI périmée le jour d'une sortie | alerte à l'organisateur et à l'infirmière ; décision humaine (la sortie n'est pas bloquée par ScolIA, la circulaire impose la trousse) |
| CS-22 | Ordonnance du PAI expirée | volet soins suspendu ; la fiche d'urgence affiche « appeler le 15 » ; famille relancée (U0 puis U1) |
| CS-23 | PAI d'un élève arrivé en cours d'année depuis un autre établissement | pli déposé ; reconduction provisoire par l'infirmière ; revue par le médecin dans le mois (S-188) |
| CS-24 | Médecin qui modifie le PAI après la signature des familles | nouvelle version ; l'ancienne reste en vigueur jusqu'aux signatures ; familles averties |
| CS-25 | Élève avec deux PAI (allergie et épilepsie, deux médecins) | **un PAI par élève** avec sections ; plusieurs médecins signataires possibles ; deux trousses possibles |
| CS-26 | PAI sans médecin disponible (pénurie) | la circulaire permet le médecin traitant ou de PMI [V] ; signature consignée par l'infirmière **d'après le document signé**, pièce au pli |
| CS-27 | Traitement apporté sans ordonnance | refusé (protocole 2000) ; famille informée |
| CS-28 | Élève qui refuse de prendre son traitement | administration « refusée » tracée ; famille informée selon le PAI ; aucune contrainte |
| CS-29 | Accident en EPS | mouvement + déclaration d'accident scolaire (formulaire de l'établissement, **sans diagnostic**, socle 7) ; le compte rendu de soins reste au cabinet |
| CS-30 | Inaptitude EPS d'un an fournie en mars, datée de septembre | refusée pour la période passée (non rétroactive, D312-4) ; enregistrée à partir de sa date de remise |
| CS-31 | Inaptitude de 4 mois | avis automatique au médecin scolaire (R312-3) |
| CS-32 | Dispense ponctuelle demandée par un mot des parents | socle 12, sans certificat : le professeur décide ; ne devient pas une inaptitude |
| CS-33 | Examen blanc : élève avec consigne `eviter_chaleur` et aménagement « pauses » | le 38 lit la consigne (audience « examens », S-189) et l'aménagement (35) |
| CS-34 | Restauration : nouvel élève allergique inscrit le matin même | consigne publiée par l'infirmière **avant** la signature du PAI (source manuelle, 3 mois au plus) ; alerte au badge de cantine |
| CS-35 | Panier repas de l'internat | consigne `repas_panier` ; stockage du panier : affaire de la restauration (socle 40) |
| CS-36 | Professeur qui demande à l'infirmière « qu'a-t-il ? » par la messagerie | la boîte C3 répond sans détail ; ScolIA rappelle à la saisie ce qui peut être partagé (§9) |
| CS-37 | AESH mutualisée sur trois élèves | consignes et conduites à tenir des **trois seuls** élèves accompagnés |
| CS-38 | Sortie d'un jour à l'étranger | fiche hors ligne + numéros du pays ; PAI « sans nuitée » respecté |
| CS-39 | Téléphone de l'encadrant perdu pendant le voyage | fiches chiffrées sur l'appareil ; révocation à distance de la session (socle 11) ; effacement à la prochaine connexion ; incident signalé au DPO |
| CS-40 | Épidémie de gastro-entérite : la direction veut la liste des malades | décompte agrégé ; liste nominative au cercle seulement ; l'ARS peut demander les contacts (export tracé) |
| CS-41 | Cas de méningite : l'ARS demande les contacts sous 24 h | liste des contacts (cours, groupes, transport, internat) produite par l'infirmière ou la direction sur demande de l'ARS, export chiffré, journalisé |
| CS-42 | Éviction pour gale : retour avec certificat | absence « santé » ; retour sur dépôt du certificat sous pli ; consigne levée |
| CS-43 | Campagne HPV : un parent autorise, l'autre refuse | autorisation du socle 18 (règle de la décision partagée [NV]) ; ScolIA n'inscrit pas l'élève tant que le conflit n'est pas tranché par l'équipe vaccinale |

### 12.3 Système, données, sécurité
| # | Cas | Réponse [R] |
|---|---|---|
| CS-44 | Le chef d'établissement veut lire un dossier « pour se protéger » après un accident | refus ; il peut demander au cercle une **attestation** de conduite tenue ; journal des accès disponible au DPO |
| CS-45 | L'administrateur technique ouvre le module « pour voir » | N-1 et N-2 exigent l'attestation du DPO (et HDS pour N-2) ; refus serveur (SD-23) |
| CS-46 | Désactivation de N-2 (fin du contrat HDS) | lecture seule pendant 3 mois pour transmission et export par l'infirmière, puis effacement cryptographique ; N-1 conservé |
| CS-47 | Passage de N-1 à N-2 en cours d'année | les mouvements N-1 restent ; les plis deviennent ouvrables ; aucune migration de texte libre (il n'y en a pas) |
| CS-48 | Fusion de deux fiches élèves qui ont chacune un PAI | fusion **bloquée** pour la partie santé ; tâche au cercle de soins qui choisit (corrige SD-16) |
| CS-49 | Élève qui change d'établissement du même organisme | aucun partage automatique ; dossier de liaison avec accord (§10.14) |
| CS-50 | Infirmière partagée entre deux établissements | un cercle par établissement ; un compte, deux casquettes (socle 6) ; aucune vue croisée |
| CS-51 | Médecin scolaire de secteur (30 établissements) | accès par établissement rattaché, limité à ses PAI, avis et inaptitudes > 3 mois ; file unique de travail |
| CS-52 | Trace des accès impossible à écrire (base saturée) | **refus** de la lecture (fail-closed, R-S8) |
| CS-53 | Demande d'effacement RGPD d'une famille | refus motivé pendant la scolarité (obligation légale, mission d'intérêt public) ; effacement à l'échéance (§18) [H, DPO] |
| CS-54 | Démonstration commerciale | données fictives sans pathologie nommée (SD-22) ; N-2 jamais activé en démonstration |
| CS-55 | Recherche « élèves avec PAI » par un professeur | ses élèves seulement, pastille seule ; recherche transversale réservée au cercle |
| CS-56 | Export de la liste des consignes alimentaires pour le prestataire de restauration | export de l'audience « restauration » (nom, classe, consigne, photo), **sans source**, tracé, durée de validité imprimée |
| CS-57 | Élève dont la consigne a été retirée hier | disparaît des listes du jour ; reste dans l'historique du cercle |
| CS-58 | Soupçon de maltraitance révélé à l'infirmerie | bouton « information préoccupante » → pli vers le 36 ; aucune information de la famille par ScolIA |

---

## 13. Intelligence artificielle (contrat IA du socle, `IA_COEUR.md` §5, §7.2, Q-IA5)

**Ajout personas (§30.4, R-S48) : remplace le §13.1 (deuxième point) et le §13.3** : aucune capacité IA sur une donnée du 34, N-1 compris ; seule reste la rédaction d'informations collectives (C0).

### 13.1 Ce que l'IA fait [R]
- **Hors santé personnelle** : aider à rédiger un **protocole générique** ou une **information collective** (canicule, pédiculose) à partir des modèles validés, en proposition (N3 pour toute diffusion) ; reformuler une consigne en langage simple ; traduire une **information générale** (jamais une conduite à tenir).
- **Pour la personne, dans sa portée N-1** : répondre « Léa a-t-elle une consigne pour la sortie de jeudi ? » comme l'écran le ferait (consigne + lieu de trousse) ; préparer le récapitulatif santé d'une sortie à partir du port `sante::trousses`.
- **Signaux** : **déterministes**, calculés par le socle (PAI à renouveler, trousse périmée, inaptitude > 3 mois), pas par un modèle.

### 13.2 Ce que l'IA ne fait pas [R]
- **Aucun accès au cabinet (N-2)**, même pour l'infirmière, tant qu'un fournisseur de classe « sensible » n'est pas contractualisé **en HDS**, qu'une AIPD ne l'a pas validé et que `ia.donnees_sensibles` n'est pas accordé (Q-IA5 : exclu jusqu'à décision du DPO).
- Ne **déduit jamais une cause** d'une consigne (« sans arachide » → « allergie ») ; consigne de refus explicite (« je n'ai pas accès à la santé de l'élève ») et non « aucune allergie » (T11 d'`IA_COEUR.md`).
- Ne lit **jamais** un acte confidentiel, un pli, la boîte infirmerie.
- N'envoie rien, ne publie rien, ne signe rien.

### 13.3 Capacités déclarées (registre `ent-ia`) [R]
| Capacité | Classe | Données | Confirmation |
|---|---|---|---|
| `sante.rediger_information_collective` | C0 | modèles, consignes générales | N3 pour diffusion |
| `sante.recapituler_sortie` | C3 dérivé (consignes, trousses) | port `sante::trousses` dans la portée | lecture |
| `sante.expliquer_consigne` | C3 dérivé | une consigne visible de la personne | lecture |
| `sante.statistiques_rapport` | C0 (agrégats masqués) | `sante::statistiques` | lecture |
| (aucune capacité sur le cabinet) | — | — | — |

---
## 14. Actions métier (SAN-100 et suivantes)

Abréviations : INF (infirmier·e du cercle de soins), INR (infirmier·e remplaçant·e désigné·e), MED (médecin scolaire ou médecin auteur du PAI), PSY, CPE, VS (AED, surveillant, dont de nuit), PRF (professeur), PEPS (professeur d'EPS), PP, DIR (direction), ADF (secrétariat), ACC (accueil), RES (restauration, internat), ORG (organisateur et encadrant de sortie), AESH, PAR (titulaire de l'autorité parentale), ELV (élève mineur), ELM (élève majeur), DPO, SYS (système), IA (assistant, au nom d'une personne). ★ = confirmation renforcée N3 (passkey récente, aperçu complet). **Niv.** = niveau minimal d'activation (N-1, N-2). Droits : §16.

### 14.1 Activation, paramétrage, cercles
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SAN-100 | Voir le niveau d'activation et ses conditions | DIR, INF, DPO · `sante.module.lire` | — | affiche attestations, hébergeur, AIPD | — |
| SAN-101 | Demander l'activation de N-1 ★ | DIR · `sante.module.gerer` | — | attestation du DPO (référence, date, document au coffre) obligatoire ; refus sinon | `sante.niveau_demande` |
| SAN-102 | Attester l'avis du DPO pour un niveau | DPO · `sante.module.attester` | — | compte DPO (socle 6) ; avis daté, motivé, document joint | `sante.niveau_atteste` |
| SAN-103 | Activer N-2 ★ | DIR + INF · `sante.module.gerer` + `sante.cabinet.gerer` | — | plateforme déclarée HDS (paramètre d'exploitation signé, R-S2), attestation DPO, AIPD jointe ; double geste | `sante.niveau_active` |
| SAN-104 | Désactiver un niveau ★ | DIR · `sante.module.gerer` | — | N-2 : lecture seule 3 mois puis effacement cryptographique (CS-46) | `sante.niveau_desactive` |
| SAN-105 | Déclarer l'infirmerie (lieux, horaires, téléphone, astreinte) | INF, DIR · `sante.parametrer` | N-1 | lieux du socle 5 | — |
| SAN-106 | Ouvrir ou fermer l'infirmerie pour une période (« infirmerie fermée ») | INF, DIR · `sante.parametrer` | N-1 | consigne de repli affichée à la vie scolaire | `sante.infirmerie_fermee` |
| SAN-107 | Composer le cercle de soins (titulaires, médecin rattaché) | DIR · `sante.cercle.gerer` | N-1 | depuis les fonctions du socle 6 ; un titulaire au moins ; jamais la direction elle-même | `sante.cercle_modifie` |
| SAN-108 | Désigner une remplaçante pour une période | INF, DIR · `sante.cercle.gerer` | N-1 | dates de début et de fin, fin automatique | idem |
| SAN-109 | Rattacher un médecin de secteur à plusieurs établissements | DIR de chaque établissement · `sante.cercle.gerer` | N-1 | une casquette par établissement (socle 6) | idem |
| SAN-110 | Paramétrer les règles d'information de la famille par issue | INF · `sante.parametrer` | N-1 | défauts : rentré, secours, famille appelée → avis ; retour en classe → rien (S-181) | — |
| SAN-111 | Paramétrer les audiences des consignes (restauration, internat, sorties, examens) | INF · `sante.parametrer` | N-1 | liste fermée | — |
| SAN-112 | Générer la fiche de traitement RGPD et l'AIPD pré-remplie | DPO, DIR · `sante.module.lire` | — | finalités, données par niveau, destinataires, durées, hébergeur | — |
| SAN-113 | Voir le journal de configuration du module | DPO, DIR, INF · `sante.module.lire` | — | ajout seul | — |

### 14.2 Mouvements, registre, passages
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SAN-120 | Envoyer un élève à l'infirmerie depuis l'appel ou la vie scolaire | PRF, VS, CPE · `sante.envoyer` | N-1 | refus si infirmerie fermée (consigne) ; billet numérique | `sante.eleve_envoye` |
| SAN-121 | Enregistrer l'arrivée (badge, photo, recherche) | INF, INR · `sante.registre.saisir` | N-1 | heure auto ; séance en cours lue au 10 ; localisation publiée à l'appel | `sante.arrivee` |
| SAN-122 | Enregistrer l'issue d'un mouvement | INF, INR · `sante.registre.saisir` | N-1 | issues fermées ; heure ; billet de retour | `sante.issue` |
| SAN-123 | Émettre un billet de retour en classe | INF · `sante.registre.saisir` | N-1 | visible du professeur de la séance, sans motif | idem |
| SAN-124 | Mettre un élève au repos (durée) | INF · `sante.registre.saisir` | N-1 | rappel à l'échéance | — |
| SAN-125 | Saisir un mouvement de premiers secours (infirmerie fermée, sortie, nuit) | VS, ORG, surveillant de nuit · `sante.premiers_secours` | N-1 | aucun texte libre ; codes de mesures ; transmis à l'infirmière | `sante.premiers_secours` |
| SAN-126 | Valider un mouvement de premiers secours | INF · `sante.registre.saisir` | N-1 | — | — |
| SAN-127 | Signaler un départ sans autorisation | INF · `sante.registre.saisir` | N-1 | alerte U3 interne vie scolaire (CS-18) | `sante.depart_non_autorise` |
| SAN-128 | Voir « qui est à l'infirmerie maintenant » | CPE, VS, DIR · `sante.mouvements.lire` | N-1 | sans motif ; élèves de la portée | — |
| SAN-129 | Voir la localisation d'un élève de son cours | PRF · (port `sante::localisation`) | N-1 | séance en cours seulement | — |
| SAN-130 | Saisir motif, soins et compte rendu d'un passage | INF, INR · `sante.cabinet.saisir` | N-2 | chiffré ; ajout seul | `sante.passage_complete` |
| SAN-131 | Compléter ou corriger un passage (nouvelle entrée liée) | INF · `sante.cabinet.saisir` | N-2 | jamais de réécriture (R-S9) | idem |
| SAN-132 | Clore un passage | INF · `sante.cabinet.saisir` | N-1 | une seule clôture ; réouverture motivée (SD-14) | `sante.passage_clos` |
| SAN-133 | Signer le cahier de la journée | INF · `sante.cabinet.saisir` | N-2 | horodatage, empreinte du jour | `sante.cahier_signe` |
| SAN-134 | Consulter le registre (jour, période, élève) | INF, INR, MED · `sante.registre.lire` (N-1) / `sante.cabinet.lire` (N-2) | N-1 | trace de chaque lecture | — |
| SAN-135 | Imprimer le registre réglementaire (protocole 2000) | INF · `sante.registre.lire` | N-1 | export signé, tracé | `sante.registre_exporte` |
| SAN-136 | Enregistrer un appel au 15 / 112 (heures, orientation) | INF, VS · `sante.registre.saisir` | N-1 | hôpital en N-2 | `sante.secours_appeles` |
| SAN-137 | Déclarer un accident scolaire (formulaire, sans diagnostic) | INF, DIR · `sante.accident.declarer` | N-1 | document du socle 7 | `sante.accident_declare` |
| SAN-138 | Voir les signaux de passages répétés | INF · `sante.cabinet.lire` | N-2 | déterministes, jamais hors cercle (CS-19) | — |

### 14.3 Famille appelée, renvoi au domicile, remise
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SAN-140 | Appeler la famille (joignables dans l'ordre, journal d'appel) | INF, CPE · `sante.registre.saisir` | N-1 | port `joignables` (socle 4) ; titulaires autorisés seulement | `sante.famille_appelee` |
| SAN-141 | « Merci de rappeler l'infirmerie » aux joignables | INF · `sante.registre.saisir` | N-1 | avis U3 enveloppe, accusé | `sante.rappel_demande` |
| SAN-142 | Préparer le renvoi au domicile | INF · `sante.registre.saisir` | N-1 | crée une sortie en attente (socle 14) | `sante.renvoi_prepare` |
| SAN-143 | Remettre l'élève (personne autorisée, pièce contrôlée, signature) | INF, ACC · `remise.lire` + `sante.registre.saisir` | N-1 | **interdictions lues** ; personne interdite non proposée (R-S18) | `sante.eleve_remis` |
| SAN-144 | Laisser partir seul un élève majeur | INF · `sante.registre.saisir` | N-1 | décision consignée ; parents non prévenus sans accord | idem |
| SAN-145 | Créer l'absence des séances restantes | SYS (sur SAN-143) | N-1 | motif « santé » sans détail (socle 12) | `appel.absence_creee` (12) |
| SAN-146 | Remettre un interne à l'internat ou à la famille | INF, VS · `sante.registre.saisir` | N-1 | règlement de l'internat (41) | idem |
| SAN-147 | Informer la famille d'un mouvement (avis neutre) | INF · `sante.registre.saisir` | N-1 | refus si acte confidentiel (R-S12) | `sante.famille_informee` |
| SAN-148 | Voir les avis de santé de son enfant | PAR · lien avec autorité | N-1 | issue utile, jamais motif ni soin | — |

### 14.4 Dossier infirmier (N-2)
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SAN-150 | Ouvrir le dossier infirmier d'un élève | INF, INR, MED · `sante.cabinet.lire` | N-2 | trace transactionnelle | — |
| SAN-151 | Saisir antécédents utiles, allergies, traitements au long cours | INF, MED · `sante.cabinet.saisir` | N-2 | versions, chiffré | `sante.dossier_modifie` |
| SAN-152 | Proposer une mise à jour (fiche d'urgence, allergie déclarée) | PAR, ELM · lien | N-1 | proposition, jamais écrasante (SD-15) | `sante.proposition_recue` |
| SAN-153 | Valider ou refuser une proposition | INF · `sante.cabinet.saisir` | N-2 | motif de refus | `sante.proposition_decidee` |
| SAN-154 | Écrire une transmission (remplacement) | INF · `sante.cabinet.saisir` | N-2 | lisible du cercle | — |
| SAN-155 | Joindre une pièce au dossier (pli) | INF · `sante.cabinet.saisir` | N-2 | socle 7, C3, domaine `sante` | `documents.deposee` (7) |
| SAN-156 | Ouvrir un pli déposé par une famille | INF, INR · `sante.plis.ouvrir` | N-2 | trace ; l'ouverture est visible du déposant (« lu par l'infirmerie ») | `sante.pli_ouvert` |
| SAN-157 | Voir « pièce de santé reçue » sans l'ouvrir | ADF · `sante.plis.suivre` | N-1 | aucun titre médical | — |
| SAN-158 | Rechercher dans les dossiers du cercle | INF · `sante.cabinet.lire` | N-2 | dans l'établissement seulement | — |
| SAN-159 | Recevoir le dossier de liaison d'un autre établissement | INF · `sante.cabinet.saisir` | N-2 | pli déposé avec l'accord de la famille | `sante.liaison_recue` |

### 14.5 Traitements et médicaments (N-2)
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SAN-160 | Enregistrer un traitement apporté avec ordonnance | INF · `sante.traitements.gerer` | N-2 | refus sans ordonnance (CS-27) | `sante.traitement_enregistre` |
| SAN-161 | Planifier les prises | INF · `sante.traitements.gerer` | N-2 | rappels | — |
| SAN-162 | Tracer une administration (heure, dose) ou un refus | INF · `sante.traitements.gerer` | N-2 | ajout seul | `sante.administration` |
| SAN-163 | Être alerté d'une prise manquée (élève absent) | INF · SYS | N-2 | lit l'appel | — |
| SAN-164 | Clore un traitement, restituer le médicament à la famille | INF · `sante.traitements.gerer` | N-2 | remise tracée | — |
| SAN-165 | Tenir l'inventaire des médicaments de l'infirmerie (lots, péremptions) | INF · `sante.pharmacie.gerer` | N-2 | alerte J-30 (S-183) | — |
| SAN-166 | Tracer une délivrance selon protocole infirmier | INF · `sante.traitements.gerer` | N-2 | protocole validé par le médecin (R4311-7) | idem |

### 14.6 PAI
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SAN-170 | Demander un PAI (famille) | PAR, ELM · lien | N-1 | formulaire, pli pour les pièces | `sante.pai_demande` |
| SAN-171 | Proposer un PAI (direction, équipe) | DIR, INF · `sante.pai.proposer` | N-1 | la famille est informée et associée (D351-9) | idem |
| SAN-172 | Préparer un projet de PAI (modèle national) | INF · `sante.pai.rediger` | N-2 | sections fixes ; en N-1 : métadonnées seulement | `sante.pai_projet` |
| SAN-173 | Rédiger et signer en tant que médecin | MED · `sante.pai.rediger` + `sante.pai.signer_medecin` | N-2 | signature S2 de la version (empreinte) | `sante.pai_signe` |
| SAN-174 | Consigner la signature d'un médecin externe (document signé) | INF · `sante.pai.rediger` | N-1 | pièce au pli ; « signé hors ScolIA » (CS-26) | idem |
| SAN-175 | Signer en tant que titulaire | PAR · lien avec autorité | N-1 | chaque titulaire, version lue = version signée | idem |
| SAN-176 | Signer en tant qu'élève (selon l'âge ; majeur obligatoire) | ELV, ELM · soi | N-1 | — | idem |
| SAN-177 | Signer en tant que chef d'établissement (sans voir le diagnostic) | DIR · `sante.pai.signer_chef` | N-1 | vue d'organisation (§9.2.3) | idem |
| SAN-178 | Signer pour le périscolaire ou la collectivité | signataire invité (lien nominatif, socle 7) | N-1 | partie « collectivité » | idem |
| SAN-179 | Enregistrer l'opposition écrite d'un titulaire | INF · `sante.pai.rediger` | N-1 | transmise au médecin et à la direction (CS-03) | `sante.pai_opposition` |
| SAN-180 | Mettre le PAI en vigueur | SYS (signatures minimales réunies) | N-1 | médecin + un titulaire (ou ELM) + chef ; consignes publiées | `sante.pai_en_vigueur` |
| SAN-181 | Créer une nouvelle version | MED, INF · `sante.pai.rediger` | N-2 | ancienne en vigueur jusqu'aux signatures | `sante.pai_version_creee` |
| SAN-182 | Voir les versions et leurs signatures | INF, MED, signataires | N-1 | empreintes vérifiables | — |
| SAN-183 | Retirer un PAI (fin de besoin) ★ | MED, INF · `sante.pai.rediger` | N-1 | motif ; consignes retirées | `sante.pai_retire` |
| SAN-184 | Reconduire provisoirement en début d'année | INF · `sante.pai.rediger` | N-1 | jusqu'à date (S-188) | `sante.pai_reconduit` |
| SAN-185 | Demander le renouvellement aux familles | INF · SYS | N-1 | juin et septembre, U0 | `sante.pai_a_renouveler` |
| SAN-186 | Déposer l'ordonnance du PAI | PAR, ELM · lien | N-1 | pli C3 ; échéance saisie par l'infirmière | `sante.ordonnance_deposee` |
| SAN-187 | Suspendre le volet soins (ordonnance expirée) | SYS | N-1 | fiche « appeler le 15 » (CS-22) | `sante.pai_soins_suspendus` |
| SAN-188 | Lire la conduite à tenir | adultes désignés (PRF, PEPS, AESH, RES, ORG, VS) · `sante.conduite.lire` | N-2 | audience et période ; trace | — |
| SAN-189 | Imprimer la conduite à tenir pour la trousse | INF · `sante.pai.rediger` | N-1 | document C3 numéroté, version imprimée tracée | — |
| SAN-190 | Tenir le registre des PAI | INF · `sante.pai.rediger` | N-1 | existence, dates, versions | — |
| SAN-191 | Tableau de rentrée des PAI | INF · `sante.pai.rediger` | N-1 | à rédiger, à signer, ordonnance attendue, trousse | — |
| SAN-192 | Voir le PAI de son enfant | PAR · lien avec autorité | N-1 | contenu en N-2 ; métadonnées en N-1 | — |
| SAN-193 | Reprendre un PAI de l'existant (ligne unique → version 1 figée) | SYS (reprise S3) | N-1 | rapport de reprise | `sante.pai_repris` |

### 14.7 Consignes, fiche d'urgence, protocoles, trousses
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SAN-200 | Publier une consigne (source PAI, certificat ou manuelle) | INF, MED · `sante.consignes.publier` | N-1 | catalogue fermé, audiences, dates ; manuelle ≤ 3 mois (S-184) | `sante.consigne_publiee` |
| SAN-201 | Retirer une consigne | INF, MED · `sante.consignes.publier` | N-1 | retrait daté | `sante.consigne_retiree` |
| SAN-202 | Voir les consignes de ses élèves | PRF (audience « professeurs »), PEPS, AESH · `sante.consignes.lire@mes_classes` | N-1 | sans source | — |
| SAN-203 | Liste du jour de la restauration / de l'internat | RES · `sante.consignes.lire@audience` | N-1 | photo, classe, consigne, lieu de trousse | — |
| SAN-204 | Alerte au passage du badge de cantine | SYS (socle 40) | N-1 | « consigne alimentaire : voir le chef » | — |
| SAN-205 | Exporter la liste de l'audience pour un prestataire | RES, DIR · `sante.consignes.exporter` ★ | N-1 | tracé, validité imprimée (CS-56) | `sante.consignes_exportees` |
| SAN-206 | Proposer un code au catalogue | INF · `sante.parametrer` | N-1 | validé par le référent produit ; lint « aucune maladie » (R-S4) | — |
| SAN-207 | Voir la fiche d'urgence minimale | VS, ORG, AESH, internat · `sante.urgence.lire` | N-1 | contexte (sortie, nuit) et portée | — |
| SAN-208 | Voir la fiche d'urgence complète | INF, INR, MED · `sante.cabinet.lire` | N-2 | — | — |
| SAN-209 | Fournir la section santé de la fiche de rentrée (socle 4) | SYS | N-1 | reconduction « rien n'a changé ? » | — |
| SAN-210 | Remplir ou reconduire la fiche d'urgence annuelle | PAR, ELM · lien | N-1 | informations confidentielles **sous pli** | `sante.fiche_urgence_maj` |
| SAN-211 | Écrire un protocole générique | INF · `sante.protocoles.gerer` | N-1 | versionné, aucun nom d'élève | `sante.protocole_modifie` |
| SAN-212 | Valider et signer un protocole | MED · `sante.protocoles.valider` | N-1 | R4311-7 | `sante.protocole_valide` |
| SAN-213 | Afficher les protocoles (salle des professeurs, internat) | tout personnel · `sante.protocoles.lire` | N-1 | version en vigueur | — |
| SAN-214 | Enregistrer une trousse (propriétaire, lieu, péremption) | INF · `sante.trousses.gerer` | N-1 | contenu non médical | `sante.trousse_enregistree` |
| SAN-215 | Être alerté d'une péremption | INF, PAR · SYS | N-1 | J-30, U0 | `sante.trousse_a_renouveler` |
| SAN-216 | Inventorier la trousse collective de premiers secours | INF · `sante.trousses.gerer` | N-1 | protocole 2000 | — |
| SAN-217 | Voir où est la trousse d'un élève | adultes de l'audience · `sante.consignes.lire` | N-1 | lieu, état | — |

### 14.8 Sorties et voyages
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SAN-220 | Récapitulatif santé d'une sortie | ORG · `sante.consignes.lire@sortie` | N-1 | trousses, consignes « sorties », conditions du PAI | — |
| SAN-221 | Être alerté d'un PAI qui conditionne la sortie | ORG, INF · SYS | N-1 | nuitée, accompagnement | `sante.sortie_conditionnee` |
| SAN-222 | Cocher « trousse emportée » (scan) | ORG · `sante.trousses.emporter` | N-1 | — | `sante.trousse_sortie` |
| SAN-223 | Télécharger les fiches d'urgence hors ligne | ORG · `sante.urgence.lire@sortie` | N-1 | chiffrées, liées à la session, effacement auto (SEJ-08) | `sante.fiches_hors_ligne` |
| SAN-224 | Saisir un premier secours pendant la sortie | ORG · `sante.premiers_secours` | N-1 | hors ligne possible, synchronisé | `sante.premiers_secours` |
| SAN-225 | Rendre les trousses et clore le volet santé de la sortie | ORG, INF · `sante.trousses.gerer` | N-1 | effacement des fiches | `sante.sortie_close` |
| SAN-226 | Révoquer à distance les fiches d'un appareil perdu | ORG, DIR · `sante.urgence.revoquer` | N-1 | CS-39 | `sante.fiches_revoquees` |

### 14.9 Accès d'urgence, coordination, signalement
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SAN-230 | Ouvrir un accès d'urgence au dossier (soignant hors cercle) | infirmier ou médecin d'un autre établissement du même organisme · `sante.acces_urgence` | N-2 | motif, 2 h, signalé au titulaire ; jamais un non-soignant | `sante.acces_urgence` |
| SAN-231 | Ouvrir un accès d'urgence (remplaçant non désigné) | INF d'astreinte · `sante.acces_urgence` | N-2 | idem | idem |
| SAN-232 | Proposer un échange consenti avec le psychologue ou l'AS | INF · `sante.partage.proposer` | N-2 | consentement attesté (D1110-3-1) | `sante.partage_propose` |
| SAN-233 | Recueillir et attester le consentement | INF · `sante.partage.proposer` ; PAR, ELM signent | N-2 | support écrit remis (coffre) | `sante.consentement_recueilli` |
| SAN-234 | Retirer un consentement | PAR, ELM, ELV (pour ses actes) · lien / soi | N-2 | effet immédiat | `sante.consentement_retire` |
| SAN-235 | Rédiger une information préoccupante depuis un passage | INF, MED · `sante.cabinet.saisir` | N-2 | pli vers le socle 36 ; 226-14 rappelé | `sante.ip_transmise` |
| SAN-236 | Transmettre un avis médical au socle 35 (PAP, aménagement) | MED · `sante.avis.emettre` | N-2 | avis sans motif | `sante.avis_medical` |
| SAN-237 | Demander l'avis du médecin (inaptitude > 3 mois, PAP) | INF, DIR · `sante.avis.demander` | N-1 | file du médecin | `sante.avis_demande` |

### 14.10 Inaptitudes à l'EPS
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SAN-240 | Enregistrer une inaptitude d'après certificat | INF, ADF · `sante.inaptitudes.saisir` | N-1 | incapacités du modèle 1989 ; non rétroactive ; ≤ fin d'année | `sante.inaptitude_enregistree` |
| SAN-241 | Déposer le certificat d'inaptitude | PAR, ELM · lien | N-1 | pli C3 | `sante.certificat_depose` |
| SAN-242 | Voir l'inaptitude de ses élèves (EPS) | PEPS · `sante.inaptitudes.lire@mes_classes` | N-1 | incapacités, dates | — |
| SAN-243 | Voir « dispensé » dans l'appel et la grille de notes | PRF · (ports 12, 21) | N-1 | sans motif | — |
| SAN-244 | Aviser le médecin au-delà de 3 mois | SYS | N-1 | R312-3 | `sante.avis_demande` |
| SAN-245 | Lever ou modifier une inaptitude | INF · `sante.inaptitudes.saisir` | N-1 | nouveau certificat | `sante.inaptitude_modifiee` |
| SAN-246 | Liste des inaptitudes de l'année (EPS, examens) | PEPS coordonnateur, DIR · `sante.inaptitudes.lire` | N-1 | pour le contrôle en cours de formation (38) | — |

### 14.11 Vaccinations, dépistages, visites
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SAN-250 | Enregistrer le statut vaccinal d'admission | ADF, INF · `sante.vaccination.admission` | N-1 | conforme / provisoire (date + 3 mois) / non conforme | `sante.vaccination_admission` |
| SAN-251 | Être alerté à l'échéance d'une admission provisoire | DIR, ADF · SYS | N-1 | R3111-8 | — |
| SAN-252 | Saisir les valences et dates | INF · `sante.cabinet.saisir` | N-2 | lecture du carnet | `sante.vaccinations_maj` |
| SAN-253 | Organiser une campagne (HPV, méningocoques) | INF, DIR · `sante.campagnes.gerer` | N-1 | autorisations (18), créneaux (19), listes | `sante.campagne_ouverte` |
| SAN-254 | Autoriser la vaccination en séance | PAR · lien (socle 18) | N-1 | décision de chaque titulaire selon la règle du 18 | `autorisations.*` (18) |
| SAN-255 | Préparer la liste de convocation de l'équipe vaccinale | INF · `sante.campagnes.gerer` | N-1 | export tracé | — |
| SAN-256 | Planifier un dépistage (bilan de la 12e année) | INF · `sante.depistages.gerer` | N-2 | élèves concernés, convocations | `sante.depistage_planifie` |
| SAN-257 | Saisir le compte rendu de dépistage | INF · `sante.cabinet.saisir` | N-2 | — | `sante.depistage_realise` |
| SAN-258 | Remettre la mention pour le carnet de santé | INF · `sante.cabinet.saisir` | N-2 | document à la famille | — |
| SAN-259 | Demander une visite médicale | PAR, ELM, INF, CPE · `sante.avis.demander` | N-1 | file du médecin | `sante.visite_demandee` |
| SAN-260 | Enregistrer l'accord de versement au DMP | PAR, ELM · lien | N-2 | L541-1 ; pas d'interface au départ | `sante.dmp_accord` |

### 14.12 Actes confidentiels, mineur, majeur
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SAN-265 | Enregistrer un entretien de contraception d'urgence | INF · `sante.cabinet.saisir` | N-2 | acte confidentiel pré-coché ; D5134 | (aucun événement nominatif) |
| SAN-266 | Enregistrer la délivrance et le suivi | INF · `sante.cabinet.saisir` | N-2 | compte rendu signé au cahier | — |
| SAN-267 | Enregistrer l'opposition du mineur à l'information des parents | INF · `sante.cabinet.saisir` | N-2 | L1111-5, L1111-5-1 ; portée | — |
| SAN-268 | Noter l'accompagnement par un majeur choisi | INF · `sante.cabinet.saisir` | N-2 | nom, sans compte | — |
| SAN-269 | Choisir un billet neutre pour la séance manquée | INF · `sante.cabinet.saisir` | N-2 | « rendez-vous » (S-186) | — |
| SAN-270 | Produire l'état chiffré annuel de la contraception d'urgence | INF · `sante.statistiques.produire` | N-2 | mineures / majeures, aucun nom | `sante.etat_chiffre_produit` |
| SAN-271 | Écrire discrètement à l'infirmière | ELV, ELM · soi (socle 16) | N-0 | boîte C3 ; aucune trace famille | `messagerie.fil_ouvert` (16) |
| SAN-272 | Prendre rendez-vous à l'infirmerie | ELV, ELM · soi (socle 19) | N-1 | créneau sans motif visible | — |
| SAN-273 | Rouvrir un accès à un parent après la majorité | ELM · soi | N-1 | portée choisie, révocable | `sante.acces_parent_ouvert` |
| SAN-274 | Révoquer cet accès | ELM · soi | N-1 | — | `sante.acces_parent_ferme` |
| SAN-275 | Voir sa fiche, ses consignes, son PAI, ses rendez-vous | ELV, ELM · soi | N-1 | mineur : PAI en accord avec lui ; jamais les notes de l'infirmière | — |

### 14.13 Crises sanitaires
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SAN-280 | Déclarer une situation sanitaire (épidémie, cas grave, canicule) ★ | DIR, INF · `sante.crise.piloter` | N-1 | type, dates | `sante.crise_ouverte` |
| SAN-281 | Suivre le décompte agrégé | DIR, INF · `sante.crise.piloter` | N-1 | sans nom hors cercle | — |
| SAN-282 | Produire la liste des contacts d'un cas pour l'ARS ★ | INF, DIR · `sante.crise.contacts` | N-1 | sur demande de l'ARS (référence) ; export chiffré tracé | `sante.contacts_exportes` |
| SAN-283 | Enregistrer une éviction (durée, condition de retour) | INF · `sante.evictions.gerer` | N-1 | arrêté de 1989 ; motif « santé » à l'appel | `sante.eviction` |
| SAN-284 | Diffuser un courrier type validé ★ | DIR · `notifications.diffuser` (socle 8) | N-1 | modèles validés par le médecin ; cas index jamais nommé | `notifications.diffusion` (8) |
| SAN-285 | Lister les élèves à consigne pour un risque (chaleur, confinement) | INF, DIR · `sante.crise.piloter` | N-1 | audience des professeurs concernés | — |
| SAN-286 | Clore la situation | DIR, INF · `sante.crise.piloter` | N-1 | bilan agrégé | `sante.crise_close` |

### 14.14 Accès aux données, conservation, transferts, pilotage
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SAN-288 | Recevoir une demande d'accès au dossier (L1111-7) | INF · `sante.acces_droits.traiter` | N-2 | délais affichés ; 48 h de réflexion | `sante.demande_acces` |
| SAN-289 | Produire l'export pour la personne (actes confidentiels exclus si opposition) | INF · `sante.acces_droits.traiter` | N-2 | jamais automatique | `sante.acces_servi` |
| SAN-290 | Préparer le dossier de liaison à la sortie (avec accord) | INF · `sante.cabinet.saisir` | N-2 | PAI, vaccinations, inaptitude en cours | `sante.liaison_preparee` |
| SAN-291 | Lancer la destruction à l'échéance (revue avant destruction) | INF · `sante.conservation.gerer` ; SYS | N-1 | liste revue, effacement cryptographique, procès-verbal sans contenu | `sante.donnees_detruites` |
| SAN-292 | Voir le journal des accès à ses dossiers | INF titulaire, DPO · `sante.journal.lire` | N-1 | qui, quand, quoi (catégorie), pourquoi | — |
| SAN-293 | Produire le rapport annuel d'activité | INF · `sante.statistiques.produire` | N-1 | agrégats, petits effectifs masqués | `sante.rapport_produit` |
| SAN-294 | Voir les indicateurs agrégés | DIR · `sante.statistiques.lire` | N-1 | aucun nom | — |
| SAN-295 | Contrôle nocturne d'intégrité (versions, empreintes, plis orphelins, consignes expirées) | SYS | N-1 | anomalie au cercle et au DPO | `sante.integrite_anomalie` |

**Compte** : 14 (14.1) + 19 (14.2) + 9 (14.3) + 10 (14.4) + 7 (14.5) + 24 (14.6) + 18 (14.7) + 7 (14.8) + 8 (14.9) + 7 (14.10) + 11 (14.11) + 11 (14.12) + 7 (14.13) + 8 (14.14) = **160 actions** (SAN-100 à SAN-295, numérotation à trous pour les ajouts des personas).

---

## 15. Règles transverses du socle 34 (R-S1 et suivantes) [R]

| # | Règle | Lot |
|---|---|---|
| R-S1 | **Aucun rôle par défaut n'ouvre la santé** hors du cercle de soins ; ni la direction, ni l'administrateur, ni `securite.politique`, ni le bris de glace du socle 2 | S0 |
| R-S2 | **Pas de donnée de soins hors HDS** : le service cabinet refuse de démarrer sans déclaration d'hébergement HDS signée ; N-2 refusé sinon | S1, S5 |
| R-S3 | **Partage par consignes codées** : aucune table hors cabinet n'accepte de texte libre médical ; contrôle en base (codes) | S0, S2 |
| R-S4 | **Catalogue sans maladie** : un test échoue si un code ou un libellé du catalogue contient un nom de maladie, de médicament ou d'organe (liste noire) | S2 |
| R-S5 | **Garde du module dans le service** : chaque commande et lecture santé vérifie le niveau ; la couche HTTP reste en défense en profondeur ; marquage explicite des routes (attribut), plus de liste de segments | S0 |
| R-S6 | **Fiche d'urgence sans interdits** : les personnes visées par une interdiction active ne figurent jamais comme « peut récupérer » ; l'adresse protégée n'apparaît jamais | S0 |
| R-S7 | **Chiffrement de tout champ de santé**, pièces comprises (enveloppe par objet), avec AAD par ligne | S0, S5 |
| R-S8 | **Journal des accès transactionnel et fail-closed** : pas de trace, pas de donnée ; sans contenu médical ; lisible du titulaire et du DPO, jamais de `audit.lire` générique | S0, S1 |
| R-S9 | **Ajout seul** pour le registre, le cahier, les administrations, les versions de PAI : corriger = ajouter une entrée liée ; déclencheurs en base | S0, S5 |
| R-S10 | **Version signée = version figée** (empreinte SHA-256 du contenu canonique, signature S2) ; aucune réécriture | S0, S3 |
| R-S11 | **Une localisation n'est pas un motif** : l'appel reçoit `a_l_infirmerie` / `rentre_domicile` ; jamais le motif | S2 |
| R-S12 | **Acte confidentiel** : exclu de tout avis, localisation publiée, chronologie hors cercle, export, demande d'accès sous opposition, IA ; garde en base sur les vues familiales | S5 |
| R-S13 | **Majorité** : à 18 ans, accès des titulaires fermé automatiquement ; réouverture par l'élève, révocable | S1 |
| R-S14 | **Autorité parentale** : seuls les titulaires avec autorité lisent et signent ; signature par titulaire | S0, S3 |
| R-S15 | **Aucune information de santé dans un événement, un SMS, un push, un objet d'e-mail** ; enveloppe neutre « Message de l'établissement concernant {prénom} » ; lint du socle 8 | S0 |
| R-S16 | **Absence créée par un renvoi** : motif « santé » sans détail, justifiée d'office | S2 |
| R-S17 | **Chaque titulaire pour soi** : aucune donnée de l'autre (adresse, téléphone, signature) | S0 |
| R-S18 | **Remise** : la personne interdite n'est pas proposée ; sa venue déclenche la consigne | S0 |
| R-S19 | **Consigne datée et audiencée** ; manuelle sans PAI ≤ 3 mois ; retrait daté ; jamais « tout l'établissement » hors cercle | S0, S2 |
| R-S20 | **Conduite à tenir jamais traduite automatiquement** | S3 |
| R-S21 | **Inaptitude non rétroactive**, ≤ fin d'année, médecin avisé au-delà de 3 mois | S4 |
| R-S22 | **Ordonnance expirée = volet soins suspendu** et « appeler le 15 » affiché | S3 |
| R-S23 | **Fiches hors ligne effacées** au retour + 24 h ou à la clôture | S7 |
| R-S24 | **Cas index jamais nommé** dans une diffusion ; contacts exportés seulement sur demande de l'ARS | S8 |
| R-S25 | **Conservation bornée** (§18) ; destruction après revue ; procès-verbal sans contenu | S9 |
| R-S26 | **Agrégats** : petits effectifs (< 5) masqués hors cercle ; jamais classe × motif | S1 |
| R-S27 | **Fusion d'élèves** : la partie santé n'est jamais fusionnée automatiquement | S0 |
| R-S28 | **Aucune donnée de santé dans le seed ou une démonstration** au-delà de consignes génériques | S0 |
| R-S29 | **Activation par niveau** : N3, attestation du DPO (et HDS pour N-2), journal transactionnel | S0, S1 |
| R-S30 | **IA** : aucun accès au cabinet ; aucune déduction de cause ; refus explicite | S14 |
| R-S31 | **Public** : N-2 désactivé par défaut dans un EPLE (LIEN, Esculape) | S1 |
| R-S32 | **Un seul service santé** : toute route (historique, Leptos, mobile, IA, labo) passe par les mêmes commandes ; garde de code qui échoue si un module écrit dans les tables santé hors du service | S1 |
| R-S33 | **Tests sur données fictives seulement** ; aucune base réelle | toujours |
| R-S34 | **Boîte infirmerie** : C3, membres = cercle ; avertissement « pas de détail médical » tant que N-2 n'est pas actif | S10 |

---

## 16. Droits et minimisation

### 16.1 Catalogue d'actions proposé (D-017) [R]
L'existant a `sante.lire`, `sante.saisir`, `contraintes.lire`, `sante.urgence_lire`, `amenagements.lire`, `accompagnement.gerer`, `documents.lire_sante` [F] — trop grossiers, et **donnés à la direction** (SD-01). Proposé :

| Droit | Contenu | Portées | Modèles par défaut |
|---|---|---|---|
| `sante.module.lire` | voir niveau, attestations, journal de configuration | établissement | Direction, Infirmerie, DPO |
| `sante.module.gerer` ★ | demander, activer, désactiver un niveau | établissement | Direction |
| `sante.module.attester` | attester l'avis du DPO | établissement | DPO |
| `sante.cercle.gerer` | composer le cercle, désigner une remplaçante | établissement | Direction (ne s'y inscrit pas elle-même) |
| `sante.parametrer` | infirmerie, horaires, règles d'information, audiences, propositions au catalogue | établissement | Infirmerie |
| `sante.envoyer` | envoyer un élève à l'infirmerie | mes classes, établissement | Professeur (mes classes), CPE, VS |
| `sante.mouvements.lire` | qui est à l'infirmerie, issues | établissement, mes classes | CPE, VS, Direction, Accueil (sorties) |
| `sante.registre.saisir` / `.lire` | mouvements, issues, remise, appels | établissement | Infirmerie |
| `sante.premiers_secours` | mouvement de premiers secours | établissement, sortie | VS, surveillant de nuit, encadrants de sortie |
| `sante.cabinet.lire` / `.saisir` | N-2 : dossier, passages complets, cahier | établissement | **cercle de soins seulement** |
| `sante.plis.ouvrir` / `.suivre` | ouvrir un pli / voir qu'il est reçu | établissement | cercle / Secrétariat |
| `sante.traitements.gerer`, `sante.pharmacie.gerer` | traitements, administrations, inventaire | établissement | cercle |
| `sante.pai.proposer` | proposer un PAI | établissement | Direction, Infirmerie |
| `sante.pai.rediger` | préparer, versions, registre, retrait | établissement | Infirmerie, Médecin |
| `sante.pai.signer_medecin` | signature du médecin | établissement | Médecin |
| `sante.pai.signer_chef` | signature du chef (vue d'organisation) | établissement | Direction |
| `sante.conduite.lire` | conduite à tenir | désignés, mes classes, sortie | Professeurs de l'élève, AESH, Restauration, encadrants, VS désignés |
| `sante.consignes.publier` | publier / retirer une consigne | établissement | cercle |
| `sante.consignes.lire` | lire les consignes de son audience | mes classes, audience, sortie | Professeur (mes classes), Restauration, Internat, AESH (désignés), encadrants (sortie), CPE (mes classes) |
| `sante.consignes.exporter` ★ | exporter une audience | audience | Restauration, Direction |
| `sante.urgence.lire` | fiche d'urgence minimale | sortie, nuit, désignés | encadrants, internat, AESH |
| `sante.urgence.revoquer` | révoquer des fiches hors ligne | sortie | encadrant, Direction |
| `sante.protocoles.gerer` / `.valider` / `.lire` | protocoles génériques | établissement | Infirmerie / Médecin / tout personnel |
| `sante.trousses.gerer` / `.emporter` | trousses | établissement / sortie | Infirmerie / encadrants |
| `sante.inaptitudes.saisir` / `.lire` | inaptitudes EPS | établissement / mes classes | Infirmerie, Secrétariat / Professeurs d'EPS |
| `sante.vaccination.admission` | statut d'admission | établissement | Secrétariat, Infirmerie |
| `sante.campagnes.gerer`, `sante.depistages.gerer` | campagnes, dépistages | établissement | Infirmerie |
| `sante.avis.demander` / `.emettre` | demander / émettre un avis médical | établissement | Infirmerie, Direction, CPE / Médecin |
| `sante.partage.proposer` | échange consenti | établissement | cercle |
| `sante.acces_urgence` | accès d'urgence d'un soignant | élève | soignants du même organisme |
| `sante.accident.declarer` | déclaration d'accident scolaire | établissement | Infirmerie, Direction |
| `sante.crise.piloter`, `sante.crise.contacts` ★, `sante.evictions.gerer` | crises | établissement | Direction, Infirmerie |
| `sante.acces_droits.traiter`, `sante.conservation.gerer` | droits des personnes, destruction | établissement | Infirmerie (titulaire) |
| `sante.journal.lire` | journal des accès santé | établissement | Infirmerie titulaire, DPO |
| `sante.statistiques.lire` / `.produire` | agrégats / rapport et état chiffré | établissement | Direction / Infirmerie |

Les droits des **familles et élèves** ne sont pas des rôles : ils découlent du lien avec autorité (socle 4), de la majorité (socle 2) et des accès ouverts par l'élève majeur. Les droits historiques sont **conservés comme alias** pendant la transition (S1) : `sante.lire` → `sante.cabinet.lire` (cercle seulement), `contraintes.lire` → `sante.consignes.lire` (portées réduites), `sante.urgence_lire` → `sante.urgence.lire`.

### 16.2 Pièges de minimisation à tester [R]
1. Direction, administrateur, secrétariat, CPE : **aucune** route du cabinet ne leur répond (matrice de séparation).
2. Professeur : consignes et inaptitudes de **ses** élèves seulement, sans source ; aucune liste de l'établissement.
3. `/eleves/{id}/dossier` : aucune section santé ou dispositif sans droit ; module inactif = rien.
4. Liste des dispenses du jour : ni motif, ni commentaire hors cercle.
5. Avis, SMS, push, objet d'e-mail : enveloppe neutre ; lint.
6. Événements outbox : aucun motif, aucun code de consigne, aucun nom.
7. Acte confidentiel : absent de toute vue familiale, de la chronologie, des notifications, des exports, de l'IA, de l'appel.
8. Parent sans autorité, parent interdit, personne de confiance : aucune donnée de santé ; jamais proposés à la remise s'ils sont interdits.
9. Parents d'un élève majeur : plus rien, sauf accès ouvert.
10. Journal d'audit générique : aucune entrée qui révèle l'existence d'un PAI hors cercle.
11. Support ScolIA : aucun accès au cabinet ; clés hors de portée.
12. IA : aucune donnée N-2 dans un contexte ; « je n'ai pas accès à la santé ».

---

## 17. Chiffrement et journal des accès [R]

1. **Champs** : en N-1, tous les champs qui décrivent une personne et sa santé sont chiffrés (consignes : le **code** reste en clair pour les contrôles du catalogue mais la table est sous RLS stricte et lue par ports) ; en N-2, tout le contenu du cabinet ; clé par établissement et par niveau (§6.2).
2. **Pièces** : chiffrement applicatif par objet (enveloppe), y compris l'existant (SD-09) ; purge du `BYTEA` hérité de 053 après migration vers le magasin chiffré.
3. **Journal des accès santé** (`ent_san_acces` en N-1, `cab_acces` en N-2) : qui (personne, casquette), quand, quoi (**catégorie** : registre, PAI, consignes, dossier, pli ; identifiant opaque), pourquoi (contexte : écran, port, accès d'urgence et motif), d'où (appareil) ; **dans la même transaction** que la lecture ; ajout seul ; **aucun contenu**.
4. **Lecture du journal** : l'infirmière titulaire (ses dossiers), le DPO (tout), la personne concernée **sur demande** (droit d'accès RGPD) ; jamais la direction en tant que telle.
5. **Alertes** : accès d'urgence, lecture massive (plus de N dossiers en une heure par une même personne, S-190), lecture hors horaires d'un remplaçant expiré → alerte au titulaire et au DPO.

---

## 18. Conservation [R, durées soumises au DPO]

| Donnée | Durée proposée | Source / raison |
|---|---|---|
| Dossier infirmier, passages, cahier (N-2) | **fin de la scolarité dans l'établissement + 1 an** (délai de transmission et de réclamation), puis destruction | SAGESSE : fin de scolarité puis destruction [V] ; +1 an [R] |
| Actes confidentiels | comme le dossier ; jamais transmis dans le dossier de liaison sans accord de l'élève | L5134-1 [V] |
| Mouvements (N-1) | **année scolaire + 1 an** | registre du protocole 2000 [V] ; durée [R] |
| Registre des urgences et accidents (secours appelés, accidents scolaires) | **10 ans** après les faits [R] | prudence contentieuse ; **à confirmer par le DPO** |
| PAI (versions, signatures) | durée du PAI + **1 an** ; la dernière version suit le dossier de liaison si accord | circulaire 2021 [V] ; durée [R] |
| Ordonnances et certificats | fin de validité + 1 an | instruction de tri : certificats médicaux 1 an (`VERIF_REGLEMENTAIRE.md` R41) [V via R41] |
| Consignes | durée de validité + 1 an (historique du cercle) | [R] |
| Inaptitudes | année scolaire + 1 an | R312-2 [V] ; durée [R] |
| Statut vaccinal d'admission | scolarité dans l'établissement | R3111-8 [V] |
| Fiches hors ligne | retour + 24 h | R-S23 |
| Journal des accès santé | **3 ans** [R] | preuve d'accès ; à confirmer par le DPO |
| Contacts exportés pour l'ARS | 1 mois après la fin de la situation | [R] |
| État chiffré et rapport annuel (agrégats) | sans limite (anonymes) | D5134 [V] |
| Fils de la boîte infirmerie | année + 1 an | socle 16 §15 [F] |

---

## 19. Événements (outbox, dans la transaction du fait) [R]

**Émis par le 34** (identifiants opaques et états ; **aucun motif, aucun code de consigne, aucun nom**) : `sante.niveau_demande|atteste|active|desactive`, `sante.cercle_modifie`, `sante.infirmerie_fermee`, `sante.eleve_envoye`, `sante.arrivee`, `sante.issue` (issue fermée), `sante.premiers_secours`, `sante.depart_non_autorise`, `sante.passage_clos`, `sante.secours_appeles`, `sante.accident_declare`, `sante.famille_appelee`, `sante.rappel_demande`, `sante.renvoi_prepare`, `sante.eleve_remis`, `sante.famille_informee`, `sante.pai_demande|projet|signe|en_vigueur|version_creee|retire|reconduit|a_renouveler|soins_suspendus|opposition|repris`, `sante.ordonnance_deposee`, `sante.consigne_publiee|retiree` (avec **audiences**, sans code), `sante.trousse_enregistree|a_renouveler|sortie`, `sante.sortie_conditionnee|close`, `sante.fiches_hors_ligne|revoquees`, `sante.inaptitude_enregistree|modifiee` (totale / partielle, dates), `sante.vaccination_admission`, `sante.campagne_ouverte`, `sante.avis_demande|avis_medical`, `sante.visite_demandee`, `sante.crise_ouverte|close`, `sante.eviction`, `sante.contacts_exportes`, `sante.donnees_detruites`, `sante.integrite_anomalie`. **Les événements du cabinet (N-2)** restent dans le cabinet (outbox propre) ; seuls des états opaques passent à la base principale. **Aucun événement** pour un acte confidentiel.

Les événements historiques `infirmerie.*` [F] sont **renommés** avec un alias de lecture pendant la transition (S1).

**Consommateurs déclarés** : 12 (`arrivee`, `issue`, `eleve_remis` → localisation, absence) ; 14 (`renvoi_prepare`, `eleve_remis` → sortie) ; 8 (avis) ; 2 (pastille PAI, chronologie **côté cercle seulement**, et « à l'infirmerie à 10 h 12 » pour la vie scolaire) ; 40, 41 (`consigne_*` → listes du jour) ; 42 (`trousse_*`, `sortie_conditionnee`) ; 21 et 38 (`inaptitude_*` → « dispensé ») ; 35 (`avis_medical`) ; 36 (`ip_transmise` par pli, pas par événement) ; 47 (agrégats) ; 30 (IA, lecture N-1 bornée).

**Consommés par le 34** : `eleve.cree|majeur|decede|fusionne|sortie` (2) ; `lien.*`, `interdiction.*`, `personne.adresse_protegee` (4) ; `inscription.validee` (3 : fiche d'urgence, statut vaccinal) ; `seance.*` (10) ; `appel.ouvert` (12) ; `sortie.creee|participants|depart|retour` (42) ; `autorisation.decidee` (18) ; `agent.fonction_*` (6 : cercle).

---

## 20. Notifications (natures déclarées au socle 8) [R]

| Nature | Urgence · sensibilité | Destinataires | Canaux | Contenu externe |
|---|---|---|---|---|
| `sante.passage_infirmerie` (issue utile : rentré, secours, famille appelée) | U1 · C3 | titulaires avec autorité (indépendamment de `voir_vie_scolaire`, corrige SD-06) ; élève majeur **à la place** | in-app, push, e-mail enveloppe | « Message de l'établissement concernant {prénom} — ouvrez l'ENT » (corrige SD-07) |
| `sante.rappeler_infirmerie` | U3 · C3 | joignables autorisés | tous, accusé | « Merci de rappeler l'établissement au {numéro de l'infirmerie} » |
| `sante.pai_a_renouveler`, `sante.ordonnance_attendue`, `sante.trousse_a_renouveler` | U0 · C3 | titulaires signataires, ELM | in-app, e-mail enveloppe, résumé | « Un document de santé est attendu » |
| `sante.pai_a_signer` | U1 · C3 | chaque partie, pour soi | in-app, e-mail enveloppe, lien nominatif | « Un document est à signer » |
| `sante.inaptitude_transmise` | U0 · C2 | professeur d'EPS | in-app | — |
| `sante.consigne_nouvelle` | U1 · C3 | audience (restauration, professeurs de l'élève, AESH) | in-app | — |
| `sante.sortie_conditionnee` | U1 · C3 | organisateur, cercle | in-app | — |
| `sante.depart_non_autorise` | U3 · C2 | vie scolaire du jour | in-app, push | — |
| `sante.avis_demande` | U0 · C3 | médecin | in-app, résumé | — |
| `sante.crise_information` | selon le modèle | diffusion validée (N3) | selon le socle 8 | texte validé, cas index jamais nommé |
| `sante.acces_urgence_signale` | U1 · C2 | titulaire du cercle, DPO | in-app, e-mail | « Un accès d'urgence a été ouvert » |
| `sante.majorite` | U0 · C1 | élève majeur ; titulaires (une fois, sans détail) | in-app | — |

**Jamais d'avis** pour un acte confidentiel, une consultation d'écoute, une demande de rendez-vous de l'élève.

---

## 21. Écrans clés et mobile [R]

| Écran | Où | Ce qu'il montre |
|---|---|---|
| **Infirmerie — aujourd'hui** | `/web/sante` (web, tablette) | élèves présents (photo, depuis quand, envoyé par / cours en cours), boutons « issue », « appeler la famille », « préparer le renvoi » ; alertes (trousses, PAI, ordonnances) ; file des propositions et des plis |
| **Fiche élève — onglet Santé** | `/web/eleves/{id}` (socle 2) | cercle : dossier N-2 ; autres : **pastille** et consignes de leur audience seulement ; jamais l'onglet pour la direction (sauf pastille) |
| **PAI** | `/web/sante/pai/{id}` | versions, sections, signatures par partie et par titulaire, trousses, consignes dérivées, échéances ; vue d'organisation pour le chef |
| **Tableau de rentrée** | `/web/sante/rentree` | PAI à rédiger / signer / ordonnance attendue / trousse ; inaptitudes ; statuts vaccinaux provisoires |
| **Remise** | composant du socle 4 | photo, autorisés aujourd'hui, interdictions en rouge, pièce contrôlée, signature |
| **Consignes du jour** | `/web/restauration/consignes`, `/web/internat/consignes` | photo, classe, consigne, lieu de trousse |
| **Sortie — santé** | `/web/sorties/{id}/sante` (socle 42) | récapitulatif, trousses à emporter, alertes, fiches hors ligne |
| **Activation** | `/web/sante/activation` | niveaux, conditions, attestations, hébergeur, AIPD, journal |
| **Journal des accès** | `/web/sante/journal` | qui a lu quoi, accès d'urgence, alertes |
| **Crise** | `/web/sante/crise` | décompte, contacts (ARS), évictions, courriers |
| **Famille — santé** | espace famille | fiche d'urgence (reconduire), PAI (lire, signer), dépôt sous pli, avis, écrire à l'infirmerie |
| **Élève — santé** | espace élève | ses consignes, sa fiche, son PAI, prendre rendez-vous, écrire à l'infirmière ; majeur : accès des parents |

**Mobile (D-003, socle 11)** : **infirmière** — registre rapide (badge), issue en deux gestes, appeler la famille, remise sécurisée ; **professeur** — « à l'infirmerie » dans l'appel, consignes de ses élèves ; **encadrant** — fiches hors ligne, trousses (scan), premiers secours hors ligne ; **famille** — avis, signature du PAI, dépôt d'ordonnance (photo convertie), écrire à l'infirmerie ; **élève** — rendez-vous, écrire à l'infirmière. **Aucune copie locale** d'un document du cabinet (corrige SD-24) ; fiches hors ligne seulement dans le cadre d'une sortie.

---

## 22. Modèle de données cible (brique `sante`) [R]

Principe : **faire évoluer** les tables existantes pour N-1 (renommage progressif, alias), **créer** le cabinet dans une base séparée pour N-2 ; toutes les tables portent `etablissement_id`, clés étrangères **composites** `(etablissement_id, id)`, RLS ENABLE + FORCE, aucun `ON DELETE CASCADE` sur une donnée de santé, ajout seul là où R-S9 s'applique (déclencheurs).

```
-- Base principale (N-1)
ent_san_niveau                      -- remplace ent_module_etablissement (gardée en vue de compatibilité)
  etablissement_id, niveau ('n0','n1','n2'), depuis, demande_par, active_par[]
  attestation_dpo_document_id, attestation_dpo_le, hebergement_hds_ref NULL, aipd_document_id
ent_san_niveau_journal              -- ajout seul : chaque changement, motif, acteurs
ent_san_cercle                      -- membres du cercle : agent_id, fonction ('infirmier','medecin','remplacant'), du, au
ent_san_infirmerie                  -- lieux, horaires, téléphone, état (ouverte/fermée, période)
ent_san_mouvement                   -- (évolution de ent_infirmerie_passage, partie N-1)
  id, etablissement_id, eleve_id, arrivee_at, envoye_par_agent_id NULL, seance_id NULL
  issue ('en_cours','retour_classe','repos','famille_appelee','rentre_domicile','secours','parti_sans_autorisation')
  sortie_at NULL, remis_a_personne_id NULL, sortie_vs_id NULL, absence_id NULL
  premier_secours BOOL, saisi_par, confidentiel BOOL   -- confidentiel = aucune publication
  cabinet_ref UUID NULL             -- opaque, vers cab_passage (N-2)
ent_san_mouvement_correction        -- ajout seul
ent_san_consigne                    -- (évolution de ent_contrainte_publiee)
  id, etablissement_id, eleve_id, code (catalogue), audiences TEXT[], du, au NULL
  source ('pai','certificat','manuelle'), source_ref UUID NULL (opaque), publie_par, retire_le, retire_par
ent_san_trousse                     -- id, etablissement_id, eleve_id NULL (collective), libelle_neutre, lieu, peremption, etat
ent_san_trousse_mouvement           -- sortie avec (sortie_id), retour, état
ent_san_pai                         -- registre (métadonnées) : id, etablissement_id, eleve_id, etat, version_courante, du, au, echeance, sorties ('sans_nuitee','avec_nuitee','conditions')
ent_san_pai_version                 -- ajout seul : pai_id, version, empreinte, cree_par, cree_le, cabinet_ref (contenu en N-2), en_vigueur_du
ent_san_pai_signature               -- (évolution de ent_pai_signature) + personne_id (titulaire), empreinte signée, preuve S2 (socle 7)
ent_san_pai_opposition              -- titulaire, date, document au coffre
ent_san_inaptitude                  -- eleve_id, matiere ('eps'), nature ('totale','partielle'), incapacites TEXT[] (codes 1989), du, au, certificat_doc_id, avis_medecin_demande_le
ent_san_vaccination_admission       -- eleve_id, statut ('conforme','provisoire','non_conforme'), provisoire_jusquau
ent_san_campagne, ent_san_campagne_eleve
ent_san_protocole                   -- (évolution de ent_sante_protocole) + valide_par_medecin, valide_le
ent_san_crise, ent_san_crise_compte, ent_san_eviction, ent_san_contacts_export
ent_san_acces_majeur                -- élève majeur → personne, portée, du, revoque_le
ent_san_acces                       -- journal des accès N-1 (ajout seul)
ent_san_catalogue                   -- codes, libellés, audiences permises, état (géré par le produit)

-- Cabinet (base séparée, périmètre HDS, N-2) — préfixe cab_
cab_dossier (eleve_ref opaque, établissement), cab_dossier_version
cab_passage (motif, soins, orientation, compte_rendu, confidentiel, signe_le) + cab_passage_ajout
cab_cahier_jour (empreinte quotidienne)
cab_traitement, cab_administration, cab_pharmacie_lot
cab_pai_contenu (version, sections chiffrées, conduite_a_tenir chiffrée)
cab_pli (pièce chiffrée par objet), cab_proposition
cab_vaccination, cab_depistage, cab_visite
cab_acte_confidentiel (type, opposition, accompagnant), cab_etat_chiffre
cab_consentement, cab_partage
cab_demande_acces, cab_liaison
cab_acces                           -- journal des accès du cabinet (ajout seul)
cab_outbox                          -- événements internes du cabinet
```

---

## 23. Plan de migration depuis l'existant (sans casser l'UI historique, le mobile ni le labo)

| Étape | Quoi | Garantie |
|---|---|---|
| MIG-1 (S0) | Droits : `sante.*` retirés d'Admin et Direction ; `contraintes.lire` / `sante.urgence_lire` réduits à « mes classes » / audiences ; `can_read_sante_docs` sans Administratif ; test `actions.rs:1240` réécrit | aucune donnée modifiée |
| MIG-2 (S0) | Garde de service `exiger_niveau(etab, niveau)` dans toutes les fonctions santé et accompagnement ; `/eleves/{id}/dossier` sans section `dispositifs` hors droit ; dispenses masquées | test de chaque route non couverte (SD-03) |
| MIG-3 (S0) | Interdictions de remise lues dans `personne_habilitee_a_recuperer` et la fiche d'urgence | test CS-01 |
| MIG-4 (S0) | Migration **580** : RLS sur `ent_dispense_vs`, `ent_eleve_dispositif` ; `ent_pai_version_instantane` (instantané chiffré à chaque écriture et à chaque signature, rétroactif sur la version courante) ; déclencheur « clôture unique » et table de corrections des passages ; `ent_fiche_sante_version` ; colonnes chiffrées déclarées (`commentaire` de dispense, `libelle` de dispositif, `motif` de passage) avec reprise au premier démarrage | aucune donnée perdue ; reprise idempotente |
| MIG-5 (S0) | Pièces du PAI : chiffrement par objet des pièces existantes (réécriture dans le magasin), purge du `BYTEA` hérité après vérification d'empreinte | rapport de reprise |
| MIG-6 (S1) | `ent_san_niveau` créée depuis `ent_module_etablissement` (actif → `n1` **en attente d'attestation**, jamais `n2`) | aucun établissement ne passe en N-2 par migration |
| MIG-7 (S2–S3) | `ent_infirmerie_passage` → `ent_san_mouvement` (partie N-1) + **contenu N-2 en attente** (chiffré, illisible tant que le cabinet n'existe pas, transféré au cabinet en S5) ; `ent_contrainte_publiee` → `ent_san_consigne` ; `ent_pai` → `ent_san_pai` + version 1 figée | vues de compatibilité pour `ui.rs` et le labo |
| MIG-8 (S5) | Transfert du contenu chiffré vers le cabinet HDS, **puis effacement** dans la base principale | aucune donnée de soins hors HDS après S5 |
| MIG-9 (S12) | Tables 212 transférées au socle 35 | selon la conception du 35 |
| MIG-10 (retrait) | Retrait des anciennes tables et routes avec `ui.rs` (F-001) après la durée d'archive | migrations 598–599 (ou 6498–6499) |

---

## 24. Lots de code (équipe Sonnet)

### 24.1 Tableau des lots
**Ajout personas** : S0 complété, S7-min nouveau (avant le 16 avril 2027), compléments de S1, S2, S3, S9, S11, S14 : **§30.10**.

| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| **S0** correctifs urgents | **SD-01** droits (MIG-1) ; **SD-02** interdictions (MIG-3) ; **SD-03** garde de service + `/dossier` + dispenses + journal + documents (MIG-2) ; **SD-04** masquage de `/vs/dispenses` ; **SD-05** secrétariat hors documents santé ; **SD-06** `droit_requis = lien` pour les avis de santé (avec N0b du socle 8) ; **SD-07** enveloppe neutre ; **SD-08** fermeture : lecture et signature réservées aux titulaires avec autorité, élève majeur exclu de l'accès parental, `visible_famille` forcé à faux tant que S5 n'existe pas ; **SD-09** chiffrement des pièces (MIG-5), commentaire, libellé, motif ; **SD-10** RLS ; **SD-11** trace fail-closed, PDF et listes tracés, `audit.lire` sans entrées santé ; **SD-12** instantané des versions du PAI ; **SD-14** clôture unique et corrections ; **SD-15** historique de la fiche ; **SD-16** fusion bloquée pour la santé ; **SD-17** portées réduites ; **SD-21** libellé de dispositif masqué ; **SD-22** seed sans pathologie ; **SD-23** activation N3 + attestation obligatoire + journal transactionnel ; **SD-24** PDF de fiche santé retiré du mobile | — | un test par défaut corrigé ; matrice de séparation des routes santé (direction, admin, secrétariat, CPE, professeur, parent sans autorité, parent interdit, parents d'un majeur, autre établissement) ; aucune régression des tests S2-11 hors ceux qui figeaient SD-01 ; `MIGRATION_TAG` à jour |
| **S1** droits, cercles, niveaux, journal | catalogue §16.1 avec alias ; rôles et fonctions infirmier, médecin, remplaçant (socle 6) ; `ent_san_niveau` et activation N-1 ; journal des accès complet ; garde de code R-S32 ; majorité (R-S13) ; public : N-2 désactivé par défaut (R-S31) ; fiche de traitement et AIPD générées | S0 ; socle 6 P-lots | aucun accès sans trace ; changement de niveau sans attestation refusé |
| **S2** N-1 : mouvements, remise, consignes | mouvements et issues, localisation à l'appel (12), renvoi au domicile avec absence et sortie (12, 14), appeler la famille (4), premiers secours, infirmerie fermée ; consignes v2 (formulation, audiences, dates, lint R-S4), listes du jour (40, 41), fiche d'urgence v2, protocoles validés, trousses | S1 ; socles 4, 10, 12, 14 (ports) | CS-01, CS-14, CS-17, CS-18, CS-34, CS-37, CS-56 ; aucune localisation pour un mouvement confidentiel |
| **S3** PAI v2 | versions figées, signatures S2 par partie et par titulaire, opposition, mise en vigueur, renouvellement, ordonnances au pli, registre, tableau de rentrée, reprise de l'existant ; vue d'organisation du chef ; conduite à tenir (contenu en N-2, impression pour la trousse en N-1) | S2 ; socle 7 (signature S2, pli) | CS-02, CS-03, CS-05, CS-22 à CS-26 ; aucune version modifiable |
| **S4** inaptitudes EPS | modèle 1989, non-rétroactivité, avis médecin > 3 mois, ports vers 12, 21, 38 ; reprise des dispenses médicales existantes | S2 | CS-30 à CS-32 |
| **S5** N-2 : cabinet | base séparée HDS, service cabinet, clés, transfert MIG-8 ; dossier infirmier, passages complets, cahier, propositions, plis ouvrables, traitements et administrations, pharmacie, actes confidentiels (contraception d'urgence, opposition), état chiffré | S1–S3 ; **hébergement HDS contractualisé (Q-S1)** | CS-07 à CS-09, CS-15, CS-16 ; R-S12 testée sur toutes les vues |
| **S6** vaccinations, dépistages, visites | statut d'admission (N-1), valences (N-2), campagnes (18, 19), bilan de la 12e année, visites, accord DMP | S2 (N-1), S5 (N-2) | CS-43 |
| **S7** sorties et voyages | récapitulatif, alertes, trousses emportées, fiches hors ligne chiffrées et effacées, premiers secours hors ligne, révocation | S2, S3 ; socle 42 ; socle 11 | CS-21, CS-38, CS-39 |
| **S8** crises sanitaires | mode crise, décompte, contacts pour l'ARS, évictions, courriers validés (8), PPMS (14) | S2 | CS-40 à CS-42 |
| **S9** conservation et droits | durées §18, revue et destruction, demandes d'accès (L1111-7), dossier de liaison, rapport annuel, statistiques | S5 ; DPO (durées) | destruction testée (effacement cryptographique) ; aucun acte confidentiel dans un export sous opposition |
| **S10** messagerie de l'infirmerie | boîte C3 (16), avertissement N-0/N-1, stockage au cabinet en N-2, rendez-vous (19) | socle 16 M3 | CM-31 du socle 16 ; aucun fil visible hors cercle |
| **S11** mobile | registre rapide, remise, appel, fiches hors ligne, signature du PAI, dépôt d'ordonnance, rendez-vous élève | S2–S7 ; socle 11 | aucune copie locale d'un document du cabinet |
| **S12** accompagnement (avec le 35) | avis médical vers le 35 ; transfert des tables 212 ; aménagements d'examen (38) | socle 35 | selon la conception du 35 |
| **S13** articulation LIEN / Esculape, DMP | étude des interfaces possibles (aucune API publique connue [NV]) ; export pour l'infirmière du public ; pas d'interface DMP avant décision | S5 ; socle 9 | note d'étude ; aucune double saisie imposée |
| **S14** IA | capacités §13.3 ; refus explicites ; aucune donnée N-2 | S2 ; `ent-ia` | évaluations T11 ; contexte sans donnée de soins |
| **S15** RLS stricte et retraits | RLS stricte sur toutes les tables santé ; retrait des tables et routes historiques avec `ui.rs` | tous | matrice d'isolation complète |

**Parcours minimal pour un pilote réel** : **S0, S1, S2, S3, S4 en N-1** — après l'**avis écrit du DPO** (Q-S2). N-2 (S5 et suivants) attend l'**hébergement HDS** (Q-S1). **S0 est un préalable à toute démonstration** qui activerait le module.

### 24.2 Migrations (registre `MIGRATIONS.md`)
| Plage si Q-B2 | Repli | Lot | Contenu |
|---|---|---|---|
| 6400 | **580** | S0 | `san_s0_correctifs` : RLS de `ent_dispense_vs` et `ent_eleve_dispositif` ; instantanés des versions de PAI ; clôture unique et corrections des passages ; historique de la fiche santé ; colonnes chiffrées déclarées |
| 6401 | 581 | S1 | `ent_san_niveau` (+ journal), cercle, journal des accès, accès parentaux après majorité |
| 6402 | 582 | S2 | mouvements, corrections, consignes v2, trousses, infirmerie, protocoles validés, catalogue |
| 6403 | 583 | S3 | registre des PAI, versions, signatures par titulaire, oppositions |
| 6404 | 584 | S4 | inaptitudes |
| 6405 | 585 | S5 | (base principale) références opaques vers le cabinet ; **le cabinet a son propre répertoire de migrations** (`migrations-cabinet/`, numérotation `C001`…) |
| 6406 | 586 | S6 | statut vaccinal d'admission, campagnes |
| 6407 | 587 | S7 | trousses en sortie, fiches hors ligne (révocations) |
| 6408 | 588 | S8 | crises, évictions, exports de contacts |
| 6409 | 589 | S9 | échéances de conservation, procès-verbaux de destruction |
| 6410–6489 | 590–597 | réserve | S10 à S14 et ajouts des personas |
| 6490–6499 | 598–599 | retrait | RLS stricte et retrait des tables historiques |

---

## 25. Tests et recette

- **Unitaires** : catalogue (lint R-S4), règles d'inaptitude (non-rétroactivité, 3 mois), renouvellement du PAI, calcul des échéances de conservation, masquage des petits effectifs.
- **Postgres** : scénarios S2-11 [F] gardés comme non-régression (sauf ceux qui figent SD-01) ; nouveaux : remise avec interdiction, renvoi → absence et sortie, PAI multi-titulaires et versions, opposition, majorité à minuit, fusion bloquée, trace fail-closed (écriture de trace qui échoue → lecture refusée), déclencheurs d'ajout seul (tentative d'`UPDATE` et de `DELETE` refusée).
- **Séparation** (matrice déclarative de `separation.rs`) : toutes les routes santé × (direction, admin, secrétariat, CPE, AED, professeur d'un autre groupe, professeur d'EPS, restauration, parent sans autorité, parent interdit, personne de confiance, parents d'un majeur, élève mineur sur le dossier d'un autre, autre établissement).
- **Niveau** : chaque route × (N-0, N-1, N-2) ; un appelant non HTTP (service appelé directement) refusé en N-0.
- **Acte confidentiel** : un test qui parcourt **toutes** les vues familiales, l'appel, la chronologie, les avis, les exports, l'IA, et échoue si l'acte y apparaît.
- **Cabinet** : démarrage refusé sans déclaration HDS ; aucune clé N-2 dans le processus principal (test d'architecture).
- **Recette par personas** (D-021 étape 4) : infirmière (journée de 40 passages, renvoi d'un enfant au parent autorisé alors que l'autre est interdit), médecin de secteur, chef d'établissement qui **tente** de lire un dossier, professeur d'EPS, chef de cuisine, encadrant de voyage qui perd son téléphone, parent séparé, mineure (contraception d'urgence), élève qui devient majeur, DPO (journal, AIPD).
- **Revue de sécurité** (`security-review`) obligatoire sur S0, S1, S5.

---

## 26. Réglages signalés « usine à gaz » (D-023) — signalés, non supprimés

| # | Réglage | Défaut | Pourquoi signalé |
|---|---|---|---|
| S-180 | N-2 activable dans un EPLE | non | double saisie avec LIEN |
| S-181 | Règles d'information de la famille par issue | rentré, secours, famille appelée | trop d'avis inquiètent, trop peu fâchent |
| S-182 | Signature de l'élève mineur sur le PAI à partir de quel âge | collège (11 ans) [H] | à valider avec le médecin référent |
| S-183 | Gestion de l'inventaire pharmaceutique de l'infirmerie | simple (lots, péremptions) | dérive vers un logiciel de pharmacie |
| S-184 | Durée maximale d'une consigne manuelle sans PAI | 3 mois | — |
| S-185 | Correction mineure du PAI sans nouvelle signature des familles | désactivé | risque de contestation |
| S-186 | Billet neutre pour une séance manquée par un acte confidentiel | « rendez-vous » | — |
| S-187 | Seuil de masquage des petits effectifs | 5 | — |
| S-188 | Délai de reconduction provisoire d'un PAI en début d'année | 30 septembre | circulaire : « en attendant » |
| S-189 | Audience « examens » pour les consignes | ouverte au 38 | — |
| S-190 | Seuil d'alerte de lecture massive | 30 dossiers / heure | — |
| S-191 | Remise sur tablette avec signature de la personne | oui | matériel de l'infirmerie |
| S-192 | Billet de retour en classe obligatoire | oui | habitudes locales |
| S-193 | Fiche d'urgence imprimable pour l'affichage (internat) | non | risque d'exposition |

---

## 27. Questions à Florian (avec recommandations, appliquées par défaut — D-031)

**Ajout personas** : Q-S29 à Q-S40 et précisions de Q-S1, Q-S2, Q-S3, Q-S6 : **§30.11** et `PERSONAS_34.md` §8.

| # | Question | Recommandation |
|---|---|---|
| Q-S1 | **Hébergement HDS** : contrat Scaleway HDS et **certification de ScolIA** comme hébergeur (activité d'administration et d'exploitation) ou **infogérance** par un prestataire certifié ? Budget ? | **Soumis à Florian** (contrat, coût). Recommandation : ne pas engager avant un pilote N-1 ; vérifier Scaleway sur la liste de l'ANS (v2) ; demander deux devis (certification propre / infogérance) ; N-2 reporté d'ici là. |
| Q-S2 | **N-1 sans HDS** : activable après **avis écrit du DPO** (et d'un juriste si besoin) ? | **Soumis au DPO** (engagement extérieur). Recommandation : oui, avec AIPD, chiffrement, journal ; **refus serveur** sans attestation. |
| Q-S3 | **Infirmier et médecin référents** du pilote (relecture des consignes, protocoles, PAI, recette) ? | **Soumis à Florian** (personnes réelles). Recommandation : un infirmier du privé et, si possible, un du public ; un médecin scolaire. |
| Q-S4 | Lancer **S0** tout de suite, avant toute activation et toute démonstration du module ? | **Oui** (appliqué) : SD-01 et SD-02 sont critiques. |
| Q-S5 | **Trois niveaux** au lieu d'un booléen ? | **Oui** (appliqué). |
| Q-S6 | **Durées de conservation** (§18) ? | **Soumis au DPO** ; défauts du §18 appliqués en attendant (alignés sur SAGESSE). |
| Q-S7 | **Public** : N-2 désactivé par défaut dans un EPLE (LIEN, Esculape) ? | **Oui** (appliqué) ; S-180 pour une académie qui le demande. |
| Q-S8 | **Direction** : aucun accès au cabinet, vue d'organisation pour signer le PAI, indicateurs agrégés ? | **Oui** (appliqué). |
| Q-S9 | Plis déposés en N-1 : acceptés (non ouvrables jusqu'à N-2) ou refusés ? | **Refusés en N-1** (appliqué) : la famille remet l'ordonnance à l'infirmière sous enveloppe ; évite de stocker des ordonnances hors HDS. |
| Q-S10 | PAI : **une signature de titulaire suffit**, l'autre informé et libre de signer ? | **Oui** (appliqué). |
| Q-S11 | Désaccord entre titulaires : ScolIA enregistre l'opposition et ne tranche pas ? | **Oui** (appliqué) ; avis juridique au premier cas. |
| Q-S12 | Notification MDPH et PPS : documents C3 du domaine santé, lus du cercle et du référent désigné ? | **Oui** (appliqué) ; détail au socle 35. |
| Q-S13 | **Contraception d'urgence et actes confidentiels** : modèle §10.10 (aucune trace hors cercle, billet neutre) ? | **Oui** (appliqué). |
| Q-S14 | **Élève majeur** : accès parental fermé à 18 ans, réouverture par l'élève ? | **Oui** (appliqué). |
| Q-S15 | **Localisation à l'appel** (« à l'infirmerie ») visible du professeur de la séance ? | **Oui** (appliqué, SAGESSE [V]). |
| Q-S16 | Campagnes de vaccination : l'établissement ne trace que les autorisations et la convocation ? | **Oui** (appliqué). |
| Q-S17 | **Catalogue de consignes** formulé en instructions, sans maladie, extensible par le seul référent produit ? | **Oui** (appliqué). |
| Q-S18 | `contraintes.lire` réduit aux audiences et à « mes classes » (plus d'accès établissement pour secrétariat et CPE) ? | **Oui** (appliqué) ; le CPE garde « mes classes » et la vie scolaire du jour. |
| Q-S19 | Restauration : export de l'audience pour un prestataire externe autorisé (N3, tracé) ? | **Oui** (appliqué). |
| Q-S20 | Fiches d'urgence **hors ligne** pour les encadrants, effacées au retour ? | **Oui** (appliqué). |
| Q-S21 | **Boîte infirmerie** en N-0/N-1 limitée à la logistique, avec avertissement (Q-M21 du socle 16) ? | **Oui** (appliqué) ; **soumis au DPO** comme le prévoyait Q-M21. |
| Q-S22 | Tables 212 (dispositifs, aménagements, AESH, réunions) transférées au socle 35, gelées d'ici là ? | **Oui** (appliqué). |
| Q-S23 | IA : aucune capacité sur le cabinet ; capacités N-1 bornées (§13.3) ? | **Oui** (appliqué ; conforme à Q-IA5). |
| Q-S24 | Registre des urgences et accidents conservé **10 ans** ? | **Oui par défaut** (appliqué) ; **à confirmer par le DPO** (Q-S6). |
| Q-S25 | Plages 6400–6499 (Q-B2) ou repli 580–599 ? | **Réservées** (appliqué) ; S0 part en **580**. |
| Q-S26 | Données de démonstration sans pathologie nommée ? | **Oui** (appliqué). |
| Q-S27 | Cabinet dans **une base séparée** avec un service et des clés propres ? | **Oui** (appliqué) ; coût d'exploitation à inclure dans Q-S1. |
| Q-S28 | Signaler à Florian le risque Next.js (données de santé mentale de mineurs envoyées à un fournisseur d'IA, `IA_COEUR.md`) ? | **Oui** (appliqué : signalé ici, aucune action sur Next.js). |

---

## 28. Faits, hypothèses, contradictions

**Faits [F]** : §2 (code et migrations lus sur `origin/ENT` `ee7706d3`, défauts relus ligne à ligne pour SD-01, SD-02, SD-04, SD-06, SD-23) ; plages de migrations vérifiées sur toutes les branches distantes (plus haute migration 480, plus haute réservation 560–579).
**Vérifié [V]** : §5 (sources citées) ; certitude « partielle » signalée quand seuls des extraits ont été vus ([Ve]).
**Hypothèses [H]** : pratiques du terrain (§3, §4) ; certificat HDS de Scaleway (source éditeur) ; organisation de la santé dans le privé sous contrat ; usage de LIEN et Esculape hors du public ; HDS non exigé pour N-1 ; âge de signature du mineur ; astreinte de nuit.
**Contradictions relevées** :
1. **Deux numérotations SAN** : `SOCLE2_DOMAINES.md` (SAN-01 à SAN-08 : fiche, passage, famille, renvoi, PAI, contrainte, psy, registre) et `MATRICE_CHARLEMAGNE.md` §3.12 (SAN-05 à SAN-12 avec d'autres sens : fiche chiffrée, passages, PAI, médicaments, extraits pour les sorties, contraintes). Ce document numérote **SAN-100 et suivants** et ne renumérote rien (signalé, non modifié).
2. **Propriété du PAI** : `SOCLES.md` place le PAI au socle 34 ; `21_EVALUATIONS.md` §6 (ligne 144) écrit « 35 possède PAP, PPS, PPRE, PAI ». Ce document **garde le PAI au 34** (circulaire 2021 : document médical rédigé par le médecin) et laisse PAP, PPS, PPRE au 35 ; `21_EVALUATIONS.md` est à corriger (signalé).
3. **`SOCLE2_DOMAINES.md` SAN-07** range le dossier du psychologue dans la santé ; la carte le place au **socle 36** : ce document suit la carte.
4. **Circulaire PAI 2021** : ordonnance « valable un an au plus » et « de moins de trois mois » fournie chaque année (contradiction interne du texte) ; ScolIA saisit la **date de fin de validité** portée par l'ordonnance, sans règle propre.
5. **Circulaire 2015-119 et protocole 2000** citent des articles du CSP renumérotés (R4311-14 abrogé au 30/06/2026, décret 2025-1306) : les gabarits de ScolIA citent R4311-7.
6. **Socle 2** (`02_ELEVE.md` §R-E11) donne le bris de glace à l'infirmerie, à la direction et au CPE pour la scolarité et l'état civil : compatible, **à condition** qu'il n'ouvre jamais le cabinet (§9.2.4) — précision à reporter au socle 2.
7. **Socle 4** (`04_ADMINISTRATIF.md` §22.1, question b) se demandait si l'infirmerie doit lire F0/F1/R des responsables : **oui** (joindre et remettre l'enfant), confirmé ici.
8. **Socle 8** prévoit `sante.passage_infirmerie` aux titulaires « indépendamment de `voir_vie_scolaire` » : ce document le confirme, **sauf actes confidentiels** et **sauf élève majeur**, deux exceptions à ajouter au socle 8.
9. **Socle 16** Q-M21 parlait d'une boîte infirmerie « avant l'HDS » ; ce document précise : logistique en N-0/N-1, stockage au cabinet en N-2.
10. `docs/socles/MIGRATIONS.md` (copie d'`ENT`) ne contient pas les réservations des socles 10, 16, 21, 24, 25, 26 (portées par leurs branches) : la ligne du socle 34 est ajoutée ici en tenant compte de ces branches.
11. **(personas)** §6.1 : N-0 « ne contient jamais » de donnée de santé, mais garde « envoyé à l'infirmerie » : tranché par R-S35 (§30.1).
12. **(personas)** §13 : l'IA lit les consignes N-1, alors que le §5.3 en fait des données de santé et que Florian exclut l'IA de toute donnée de santé : tranché par R-S48 (§30.4).
13. **(personas)** §10.10 : l'acte confidentiel n'existe qu'en N-2, alors que le pilote vivra en N-1 : tranché par R-S50 (mouvement confidentiel dès N-1).
14. **(personas)** §9.1 : la pastille PAI est visible du CPE pour tous les élèves : réduite par R-S38.
15. **(personas)** `PORTABILITE_EXPORTS.md` §4 fait approuver l'export santé par la direction : compatible avec R-S1 seulement sur une vue sans contenu (R-S45) ; §4 à préciser.
16. **(personas)** D-040 (deuxième destination de sauvegarde) : pour le cabinet, chaque destination doit être certifiée HDS (activité 6°) ; §18 : journal des accès 3 ans seulement si l'AIPD le justifie (délibération CNIL 2021-122), sinon 1 an.

---

## 29. Les cinq décisions à prendre en priorité

1. **Lancer S0 maintenant** (Q-S4) : la direction lit les dossiers médicaux, un enfant malade peut être remis à un parent interdit, la garde « module désactivé » laisse passer des motifs médicaux, et les avis de santé n'arrivent pas aux parents séparés — **sur `ENT` aujourd'hui**. **Recommandation : oui, appliquée par défaut.**
2. **Le secret avant tout** (Q-S8, Q-S13, Q-S14) : aucun rôle hors du cercle de soins, actes confidentiels du mineur invisibles par construction, majorité respectée. **Recommandation : oui, appliquée.**
3. **Trois niveaux, et N-1 d'abord** (Q-S5, Q-S2) : un pilote réel en N-1 (mouvements, remise, consignes, PAI sans contenu médical, inaptitudes) **après l'avis écrit du DPO**. **Soumis au DPO.**
4. **L'hébergement HDS** (Q-S1, Q-S27) : certification de ScolIA ou infogérance certifiée, contrat Scaleway HDS, base séparée pour le cabinet. **Soumis à Florian** (contrat, coût) ; N-2 attend.
5. **Les référents métier** (Q-S3) : un infirmier et un médecin scolaires pour relire consignes, protocoles et PAI et conduire la recette. **Soumis à Florian.**

---

## 30. Ajout personas (2026-11-10, `PERSONAS_34.md`)

**Statut :** proposition, non décidée ; recommandations appliquées par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (hébergement HDS et coût, DPO, référents médicaux : `PERSONAS_34.md` §8.2). **Ce paragraphe l'emporte** sur le texte d'origine quand ils se contredisent (contradictions écrites au §28, points 11 à 16). Sources réglementaires nouvelles : `PERSONAS_34.md` §5 (R1 à R19).

### 30.1 N-0 « sans donnée de santé » (R-S35, corrige SD-28)
1. **N-0 est le défaut**, activable sans HDS ni avis du DPO, parce qu'il ne contient **aucune donnée de santé**, même indirecte (C-184/20) : aucune table `ent_san_*` ne reçoit de ligne, aucune route santé ne répond, la boîte infirmerie n'existe pas.
2. L'appel et la vie scolaire disposent d'un **statut neutre « sortie autorisée »** (heure de sortie, heure de retour, autorisé par), **sans lieu ni motif** ; il remplace « envoyé à l'infirmerie » en N-0. En N-1, le lieu « infirmerie » s'affiche, sauf mouvement confidentiel (R-S50).
3. Restent disponibles en N-0, car ce ne sont pas des données de santé : contacts d'urgence, personnes autorisées, interdictions de remise (socle 4), remise sécurisée, numéros d'urgence, protocoles **génériques** sans nom d'élève, compteur anonyme de l'état chiffré (SAN-319).
4. **Registre des données de santé indirectes des autres socles** (tenu au registre de classement E12, colonne « santé indirecte ») : motif d'absence « maladie » et certificats d'absence (12), dispositifs, aménagements et AESH (35), statut « dispensé » (21), régimes de la cantine (40). Ils relèvent de l'AIPD générale de l'ENT, pas du mode du 34 ; le **dossier d'avis du DPO** (SAN-324) les liste.

### 30.2 Mode « sans infirmière » (R-S43, S-463)
1. Proposé quand le cercle de soins est **vide** (pas d'infirmière, départ, poste vacant : SAN-305) ; **N-1 seulement** ; **soumis au DPO** avec l'avis N-1 (Q-S33).
2. Le chef désigne un **référent PAI** non soignant (fonction du socle 6, SAN-322) : il tient le **registre des PAI** (métadonnées), consigne les signatures d'après le document signé (SAN-174), publie et retire les **consignes** à partir d'un PAI ou d'un certificat **signé par un médecin** (pièce vue, **non stockée**), enregistre les trousses et l'armoire à pharmacie collective (SAN-214, SAN-216).
3. Il n'existe **ni mouvements d'infirmerie, ni plis, ni soins** ; les **premiers secours** sont saisis par les adultes formés (SAN-125), et la fiche d'urgence montre les **adultes formés présents aujourd'hui** (SAN-323). Fondement : protocole de 2000, soins et urgences assurés par des personnels formés PSC1 ou SST en l'absence d'infirmier [Ve, `PERSONAS_34.md` §5 R7].
4. À l'arrivée d'une infirmière, le référent garde la lecture du registre pendant un mois pour la transmission, puis perd ses droits.

### 30.3 Sauvegardes, restauration, export, clés (R-S44 à R-S47 ; D-037 à D-046)
1. **Destructions rejouées** (R-S44) : toute destruction (§18, SAN-291) est inscrite dans un **journal des destructions** `ent_san_destruction` (ajout seul, identifiants opaques, date, procès-verbal sans contenu), **conservé hors de la rétention des données détruites** ; après toute restauration (globale ou d'un établissement, `PRA_SAUVEGARDES.md` §10–§11), le journal est **rejoué avant réouverture** (SAN-327) ; une restauration n'annule jamais une destruction. La notice aux familles dit que la donnée détruite peut subsister jusqu'à 12 mois dans les sauvegardes chiffrées (D-043), jamais restaurée.
2. **Effets de bord** : après restauration, aucun avis santé n'est renvoyé (E9) ; le journal des accès est restauré avec les données, jamais tronqué.
3. **Cabinet (N-2)** : sauvegardé dans sa propre chaîne ; **chaque destination certifiée HDS** (activité 6°, sites dans l'EEE ; OVHcloud Object Storage ou Cold Storage, Scaleway Object Storage HDS Paris sans règles de cycle de vie, donc purge par notre code [`PERSONAS_34.md` §5 R11–R13]) ; jamais lu par `ent-extraction` de la base principale ; restauré à part, dans le périmètre HDS. **Si** le DPO juge N-1 dans le champ HDS, la même règle vaut pour les sauvegardes de N-1 et « N-1 sans HDS » tombe (question posée explicitement, Q-S2).
4. **Registre de classement (E12)** : toutes les tables `ent_san_*` en catégorie **« particulière »** ; `ent_san_acces` et `ent_san_destruction` aussi ; le cabinet tient son **registre propre**. Un test échoue si un **export complet** (D-041) contient une ligne du 34 (CS-69).
5. **Export santé** (R-S45, SAN-328 ; précise `PORTABILITE_EXPORTS.md` §4) : initié par l'infirmière titulaire (ou le DPO) ; approuvé par le DPO **et** par la direction sur une **vue sans contenu** (volumes, catégories, destinataire) ; archive chiffrée **pour un destinataire nommé** (soignant successeur, nouvel hébergeur HDS), illisible des approbateurs comme de ScolIA ; actes confidentiels exclus sauf accord de l'élève ; journal. Aucun autre export de consignes que SAN-205 (N3) ; toute impression santé est **filigranée** (lecteur, date, « document de santé — ne pas diffuser »).
6. **Clés** (R-S47, SAN-329, corrige SD-27) : clé de données santé **par établissement et par niveau**, enveloppée par une clé du KMS, avec **identifiant** (`enc:v2:<id_cle>:`, E11), séquestre **séparé** de celui d'`ENT_SECRETS_KEY` (D-042) ; sa destruction en fin de contrat rend illisibles **aussi les sauvegardes** (effacement cryptographique) ; le support n'y a jamais accès ; aucun compte partagé sur les postes de l'infirmerie ou de la cuisine, verrouillage automatique (S-462).
7. **Sécurité d'écran et journaux techniques** (R-S46) : vues santé en écran sécurisé sur Android (capture bloquée), masquées dans le sélecteur d'applications et capture détectée et tracée sur iOS, filigrane nominatif ; aucun corps de requête santé dans les journaux applicatifs et les rapports d'erreur (E14 étendue).

### 30.4 IA (R-S48, remplace le §13.1 et le §13.3)
**Aucune capacité IA sur une donnée du socle 34, quel que soit le niveau** (N-1 compris : les consignes sont des données de santé, §5.3). Seule reste `sante.rediger_information_collective` (C0 : modèles validés, aucune donnée d'élève, N3 pour diffusion). `sante.recapituler_sortie`, `sante.expliquer_consigne` et `sante.statistiques_rapport` sont **retirées**. À une question sur la santé d'un élève, l'IA répond « je n'ai pas accès à la santé des élèves » et donne le lien vers l'écran qui répond. Les contextes IA de **tous** les socles filtrent les ports du 34 ; un test échoue sinon.

### 30.5 Next.js et jalon de la bascule des voyages
1. **Faits** : `PERSONAS_34.md` §3.1 (NXS-01 à NXS-12) ; **risque de production** NXS-10 et NXS-12 signalé à Florian (Q-S34) ; aucune action du pilote sur Next.js.
2. **Reprise** : aucune donnée de soins reprise sans HDS (inchangé, T0 R-08) ; les extraits sont **recodés** à la main par l'infirmière en consignes du catalogue (SAN-300) ; à l'arrêt du module Next.js, export par l'infirmière pour elle-même, puis destruction décidée par Florian.
3. **Jalon N-1 pour la bascule du 42 (17 avril 2027)** (Q-S30) : avis écrit du DPO **avant fin janvier 2027** ; S0, S1, S2, S3 et **S7-min** prêts **avant le 16 avril 2027** ; repli : fiche sanitaire papier remise par l'infirmière à l'encadrant pour les séjours repris et mention « santé : voir l'infirmière » à l'écran (changement d'usage signalé).

### 30.6 Actions ajoutées (SAN-300 à SAN-332)
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SAN-300 | Recoder un extrait santé de Next.js en consigne du catalogue | INF, référent PAI · `sante.consignes.publier` | N-1 | à partir du PAI ; aucun texte libre repris | `sante.consigne_publiee` |
| SAN-301 | Saisir un mouvement a posteriori (heure déclarée) ; contexte « nuit d'internat » | INF, INR, surveillant de nuit · `sante.registre.saisir` | N-1 | heure déclarée et heure de saisie tracées | `sante.arrivee` |
| SAN-302 | Gérer la file d'attente (arrivées scannées, prioritaire signalé par l'envoyeur, sans motif) | INF · `sante.registre.saisir` ; PRF, VS (case « urgent ») | N-1 | — | — |
| SAN-303 | Rappeler un joignable (numéros du socle 4), jamais confirmer à un appel entrant inconnu | INF, VS · `sante.registre.saisir` | N-0 | R-S36 | `sante.famille_appelee` |
| SAN-304 | Afficher le registre du jour pour le recopier dans LIEN (aucun fichier) | INF · `sante.registre.lire` | N-1 | public ; tracé | — |
| SAN-305 | Fermer l'accès à la fin de fonction ; alerter « cercle vide » | SYS (socle 6) ; DIR | N-1 | proposition du mode sans infirmière | `sante.cercle_modifie` |
| SAN-306 | Préparer la vacation du médecin (file, créneaux) | INF · `sante.avis.demander` | N-1 | socle 19 | — |
| SAN-307 | Ouvrir la fiche minimale d'un élève en urgence (vie scolaire) | CPE, VS · `sante.urgence.lire@urgence` | N-1 | motif en un geste ; signalé à l'infirmière ; seuil S-461 | `sante.acces_urgence` |
| SAN-308 | Déclencher « urgence en classe » depuis l'appel | PRF, VS · `sante.envoyer` | N-0 | U3 infirmerie et vie scolaire ; numéros d'urgence | `sante.urgence_classe` |
| SAN-309 | Saisir les incapacités d'un certificat reçu et le suivre jusqu'à l'infirmerie | PEPS · `sante.inaptitudes.saisir@mes_classes` | N-1 | aucun scan en N-1 | `sante.inaptitude_enregistree` |
| SAN-310 | Joindre une traduction de la conduite à tenir validée par le médecin | MED · `sante.pai.rediger` | N-2 | version du PAI ; jamais automatique (R-S20) | `sante.pai_version_creee` |
| SAN-311 | Tenir la main courante d'une crise | DIR, INF · `sante.crise.piloter` | N-1 | ajout seul ; aucun nom hors cercle | — |
| SAN-312 | Produire la fiche PPMS par zone (élèves à consigne présents, trousses) | DIR, INF · `sante.crise.piloter` ; responsable de zone en lecture | N-1 | d'après l'EDT ; zone seulement | — |
| SAN-313 | Produire le relevé de conformité d'un PAI (sans contenu) | DIR, INF · `sante.pai.signer_chef` | N-1 | signatures, consignes, lectures, trousses | `sante.releve_produit` |
| SAN-314 | Signer en série les PAI de rentrée | DIR · `sante.pai.signer_chef` | N-1 | confirmation par PAI | `sante.pai_signe` |
| SAN-315 | Suivre une enveloppe papier (reçue, remise à l'infirmerie) | ADF, INF · `sante.plis.suivre` | N-0 | aucun titre médical | — |
| SAN-316 | Informer la famille à chaque passage pour un élève et une période | INF · `sante.parametrer` | N-1 | S-460 ; jamais pour l'écoute ni un acte confidentiel | — |
| SAN-317 | Voir « ce que l'établissement sait de la santé de mon enfant » | PAR, ELM · lien | N-0 | niveau, consignes, registre, mouvements utiles, droit d'accès | — |
| SAN-318 | Enregistrer un mouvement confidentiel | INF · `sante.registre.saisir` | N-1 | R-S50 | (aucun événement nominatif) |
| SAN-319 | Compter un acte anonyme pour l'état chiffré (mineure / majeure, mois) | INF · `sante.statistiques.produire` | N-0 | aucune donnée personnelle | — |
| SAN-320 | Chronométrer une crise et l'enregistrer comme premier secours | adultes de l'audience · `sante.premiers_secours` | N-1 | sans texte libre | `sante.premiers_secours` |
| SAN-321 | Désigner les adultes formés à un geste du PAI | INF, référent PAI · `sante.pai.rediger` | N-1 | nom, formation, date ; lu par la fiche d'urgence | — |
| SAN-322 | Désigner le référent PAI (mode sans infirmière) | DIR · `sante.cercle.gerer` | N-1 | R-S43 | `sante.cercle_modifie` |
| SAN-323 | Voir les adultes formés présents aujourd'hui | tout personnel · `sante.protocoles.lire` | N-0 | socle 6 (habilitations), EDT | — |
| SAN-324 | Produire le dossier d'avis du DPO | DIR, DPO · `sante.module.lire` | N-0 | niveaux, matrice, tests, hébergement, sauvegardes, durées, données indirectes | — |
| SAN-325 | Suivre la procédure de contrôle de la CNIL | DPO · `sante.journal.lire` | N-0 | données médicales individuelles en présence d'un médecin (78-17, 19 III) | — |
| SAN-326 | Calculer le périmètre d'une fuite (export, fichier, appareil) | DPO · `sante.journal.lire` | N-1 | liste des élèves concernés, sans contenu | `sante.fuite_evaluee` |
| SAN-327 | Rejouer le journal des destructions après restauration | SYS (exploitation) | N-1 | avant réouverture ; bloque la réouverture en cas d'échec | `sante.destructions_rejouees` |
| SAN-328 | Exporter la santé (procédure particulière) ★ | INF titulaire ou DPO · `donnees.export.sante` ; DPO et DIR approuvent | N-1 | R-S45 | `sante.export_sante` |
| SAN-329 | Créer, faire tourner, détruire la clé santé d'un établissement ★ | SYS (exploitation, deux personnes) | N-1 | E11, D-042 | `sante.cle_*` |
| SAN-330 | Générer la notice aux familles par niveau | DIR, DPO · `sante.module.lire` | N-0 | sauvegardes comprises | — |
| SAN-331 | Afficher l'écran d'accueil du module par rôle (« ce que vous voyez, et pourquoi pas plus ») | tous | N-0 | — | — |
| SAN-332 | Tenir le protocole « santé mentale » de l'établissement et recueillir l'accord parental d'orientation | INF, MED, PSY · `sante.protocoles.gerer` ; PAR · lien | N-1 | circulaire 2026/58 ; aucune liste d'élèves | `sante.protocole_modifie` |

**Précisions sur des actions existantes** : SAN-137 (déclaration d'accident) ouverte au **témoin** (PRF, PEPS) ; SAN-138 (passages répétés) **dès N-1** sur les mouvements ; SAN-174 aussi par le référent PAI ; SAN-179 : chaque titulaire est informé séparément ; SAN-214 : trousse « portée par l'élève » et « conservation au froid » ; SAN-216 aussi par le référent PAI ; SAN-232 possible **en N-1** pour les seuls mouvements ; SAN-292 : réponse du DPO aux familles par catégories, dates et finalités. **Compte** : 160 + 33 = **193 actions**.

### 30.7 Règles ajoutées (R-S35 à R-S52)
| # | Règle | Lot |
|---|---|---|
| R-S35 | **N-0 sans donnée de santé** : aucune ligne santé ; statut neutre « sortie autorisée » sans lieu ni motif | S0 |
| R-S36 | **Jamais de confirmation à un appel entrant non identifié** ; rappel aux seuls joignables | S2 |
| R-S37 | **Accès d'urgence de la vie scolaire** : fiche minimale seulement, motif, trace, signalement, seuil bas, revue hebdomadaire par l'infirmière | S1 |
| R-S38 | **Pastille PAI du CPE** limitée aux élèves dont il a la charge (corrige la matrice §9.1) | S0 |
| R-S39 | **Audience calculée sur qui a l'élève maintenant** (séance, remplacement, permanence, intervenant, sortie, affectation AESH du jour) | S2 |
| R-S40 | **Bénévole de sortie** : fiche minimale de son groupe seulement | S7 |
| R-S41 | **Rendez-vous de l'infirmerie et du psychologue** : ni motif ni auteur hors cercle, jamais dans l'EDT vu de la famille | S10 |
| R-S42 | **Aucune donnée de santé au prestataire** d'un séjour, sauf la consigne alimentaire par SAN-205 | S7 |
| R-S43 | **Mode sans infirmière** (§30.2) | S1–S3 |
| R-S44 | **Sauvegarde et restauration** : destructions rejouées, effets neutralisés, cabinet HDS sur chaque destination, export complet sans santé (§30.3) | S0 (classement), S1, S9 |
| R-S45 | **Sortie de données de santé** : export santé chiffré pour un destinataire nommé, approuvé sans contenu ; aucun autre export que SAN-205 ; impressions filigranées | S9 (export), S2 (impressions) |
| R-S46 | **Sécurité d'écran et journaux techniques** sans contenu santé | S11, S0 (journaux) |
| R-S47 | **Comptes et clés** : clé santé séparée avec identifiant ; aucun compte partagé ; support sans accès | S0 (inventaire), S1 |
| R-S48 | **IA : aucune donnée du 34**, à aucun niveau (§30.4) | S0 (garde), S14 |
| R-S49 | **Opposition d'un titulaire** : la partie urgence s'applique toujours ; mesures non urgentes contestées suspendues jusqu'à la décision du médecin et de la direction ; opposition visible du cercle et de la direction seulement | S3 |
| R-S50 | **Mouvement confidentiel dès N-1** : aucun lieu publié (statut neutre), aucun avis, exclu de la chronologie, des vues et exports familiaux, des statistiques nominatives ; s'applique aussi à une carte de sortie vers un lieu refuge | S2 |
| R-S51 | **Téléphone de suivi médical** : la consigne lue par la discipline interdit confiscation et sanction pour cet usage (L511-5) | S2 |
| R-S52 | **Pas d'affichage mural** d'une liste santé ; tablette à session | S2 |

Extension de règles existantes : **R-S4** (catalogue) interdit aussi tout code de « vigilance psychique » ; **R-S15** (enveloppe neutre) s'applique aussi aux notifications de l'**élève** ; **R-S8** : durée du journal des accès 3 ans **seulement si l'AIPD le justifie**, sinon 1 an (délibération CNIL 2021-122, `PERSONAS_34.md` §5 R15).

### 30.8 Cas limites ajoutés (CS-59 à CS-70)
| # | Cas | Réponse [R] |
|---|---|---|
| CS-59 | Médecin de secteur au service d'établissements de deux organisations clientes | une identité, une casquette par établissement ; restauration et export d'un établissement n'emportent rien de l'autre |
| CS-60 | Élève majeur malade qui n'a désigné aucun contact | ses contacts choisis, sinon le 15 ; jamais ses parents d'office |
| CS-61 | Démo de décembre 2026 | N-0 ; santé montrée sur maquette et données fictives ; N-1 jamais activé sur le labo partagé |
| CS-62 | Restauration à J-3 alors qu'un dossier a été détruit à J-1 | destruction rejouée avant réouverture (SAN-327) |
| CS-63 | L'infirmière démissionne un lundi | accès fermé le jour même ; consignes publiées maintenues ; nouvelles publications bloquées jusqu'au référent ou à la remplaçante |
| CS-64 | Remplaçante arrivée le matin même dans une classe avec trois PAI | audience R-S39 : elle voit les consignes dès sa première séance |
| CS-65 | Élève parti sans billet, noté absent, arrivé à l'infirmerie | l'arrivée remplace l'absence avant l'envoi de l'avis (socle 12) |
| CS-66 | Crise d'asthme alors que le père s'est opposé au traitement de fond | conduite à tenir appliquée, 15 appelé ; l'opposition ne s'affiche pas à l'adulte qui agit |
| CS-67 | Le téléphone de Tom est confisqué par un surveillant | consigne R-S51 lue à la saisie de la confiscation : refus avec explication, restitution |
| CS-68 | Capture d'écran d'une fiche hors ligne | Android : bloquée ; iOS : tracée et rappel ; filigrane nominatif |
| CS-69 | Export complet demandé par l'OGEC (D-041) | santé exclue ; procédure SAN-328 à part |
| CS-70 | Contrôle sur place de la CNIL qui demande un dossier infirmier | journal exporté sans contenu ; contenu seulement sous l'autorité et en présence d'un médecin |

### 30.9 Catalogue de consignes et audiences (ajouts)
- **Audiences** : `periscolaire` (garderie, étude, accueil du mercredi ; parité NXS-07), `internat_nuit` (NXS-01), `examens` (S-189, inchangé), `zone_ppms`.
- **Codes** (instructions, aucun nom de maladie, R-S4) : `pas_de_nourriture_partagee`, `eviter_latex`, `surveillance_rapprochee_piscine`, `pas_d_activite_en_hauteur`, `eviter_lumieres_clignotantes`, `telephone_suivi_medical_autorise`, `collation_autorisee_en_classe`, `passage_prioritaire_cantine`, `repas_a_heure_fixe`, `traitement_a_heure_fixe_en_sortie`, `sortie_libre_lieu_refuge`.
- **Trousse** : lieu « portée par l'élève (prévu au PAI) » ; champ « conservation » (ambiante, froid).

### 30.10 Lots révisés (le tableau du §24.1 reste ; ces lignes le complètent)
| Lot | Ajouts | Critères de fin ajoutés |
|---|---|---|
| **S0** (complété) | **SD-25** clés étrangères composites, aucune cascade sur les signatures ; **SD-28** statut neutre « sortie autorisée » en N-0 (R-S35) ; pastille du CPE (R-S38) ; **garde IA** : aucune donnée du 34 dans un contexte IA (R-S48) ; tables santé classées « particulière » au registre (E12) et **test « export complet sans santé »** ; journaux et rapports d'erreur sans contenu santé (R-S46) ; **inventaire** de la clé santé à créer (SD-27, E11) | test « N-0 sans donnée de santé » (aucune ligne, aucune route, aucun lieu) ; test « IA sans santé » ; test « export complet sans santé » |
| **S1** | clé santé par établissement et par niveau (SAN-329) ; accès d'urgence de la vie scolaire (SAN-307) ; fin de fonction et cercle vide (SAN-305) ; référent PAI (SAN-322) ; dossier d'avis du DPO (SAN-324) ; journal des destructions (`ent_san_destruction`) | aucune lecture d'urgence sans trace ni signalement |
| **S2** | statut neutre et mouvement confidentiel N-1 (R-S50, SAN-318) ; file, saisie différée, nuit d'internat (SAN-301, SAN-302) ; rappel d'un joignable (SAN-303) ; urgence en classe (SAN-308) ; audience R-S39 ; catalogue §30.9 ; impressions filigranées ; pas d'affichage mural | CS-63 à CS-65, CS-67 |
| **S3** | opposition R-S49 ; relevé de conformité (SAN-313) ; signature en série (SAN-314) ; adultes formés (SAN-321) | CS-66 |
| **S7-min** (nouveau, **avant le 16 avril 2027**) | récapitulatif de sortie, trousses emportées, fiches d'urgence hors ligne, bénévole limité à son groupe (R-S40), aucune donnée au prestataire (R-S42) | un séjour repris part avec ses trousses cochées |
| **S9** | export santé (SAN-328) ; destructions rejouées après restauration (SAN-327) ; périmètre d'une fuite (SAN-326) | CS-62, CS-69 ; test « restauration » |
| **S11** | sécurité d'écran (R-S46), chronomètre de crise (SAN-320) | CS-68 |
| **S14** | retrait des capacités IA sur les consignes (§30.4) | — |

**Parcours minimal révisé** : **N-0** dès la démo (S0) ; **N-1** après l'avis écrit du DPO, avec S0, S1, S2, S3, S4 et **S7-min avant le 16 avril 2027** (jalon de la bascule des voyages) ; N-2 après l'hébergement HDS, **chaque destination de sauvegarde comprise**.

**Normalisation D-036 et E2** (corrige SD-26) : `ent_san_consigne_audience (etablissement_id, consigne_id, audience)`, `ent_san_inaptitude_incapacite (etablissement_id, inaptitude_id, code)`, `ent_san_niveau_activateur (etablissement_id, niveau_journal_id, personne_id)` remplacent les tableaux du §22 ; `cab_pai_section (version_id, section_code, contenu_chiffre)` : une ligne par section, jamais un bloc chiffré qui cache une liste ; nouvelles tables `ent_san_destruction`, `ent_san_referent`, `ent_san_adulte_forme`, `ent_san_compteur_anonyme (etablissement_id, mois, categorie, nombre)`, `ent_san_crise_main_courante`. Toutes : `etablissement_id NOT NULL`, FK composites, UUID applicatifs, `cree_le`, RLS (E1–E3, E6, E13). Migrations dans les plages déjà réservées (S0 en 580 ; ajouts des personas en 590–597).

### 30.11 Réglages et questions
- **Réglages** : S-460 à S-466 (`PERSONAS_34.md` §8 bis).
- **Appliquées par défaut (D-031)** : Q-S29 (N-0 strict), Q-S32 (IA), Q-S35 (confidentiel et compteur dès N-1), Q-S36 (audience), Q-S37 (accès d'urgence de la vie scolaire), Q-S38 (opposition), Q-S39 (pas de réglage « chaque passage »), Q-S40 (sécurité d'écran).
- **Soumises à Florian** : Q-S1 précisée (HDS **et** seconde destination de sauvegarde HDS, coût), Q-S2 précisée (avis du DPO **avant fin janvier 2027**, y compris la question « N-1 est-il dans le champ HDS ? »), Q-S3 (référents, plus un établissement sans infirmière), Q-S6 (durées, effet dans les sauvegardes), Q-S30 (jalon de la bascule et repli papier), Q-S31 (destinataire d'une archive santé, au contrat), Q-S33 (référent PAI non soignant, DPO), **Q-S34** (module santé de Next.js : usage, restriction ou arrêt).
