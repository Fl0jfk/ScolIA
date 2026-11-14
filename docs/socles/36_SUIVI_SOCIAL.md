# Socle 36 — Suivi social, psychologique et protection de l'enfance

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian), rédigée **après** le code existant (migrations 041, 046, 051 ; `domain-eleve/src/psy.rs`, `psy_pdf.rs`, `ent-db/src/psy_repo.rs`, fonctions `*_psy_for_actor` de `service.rs`) pour le rattraper, et après lecture ciblée (lecture seule) du module **« bien-être / signalements »** de l'application Next.js de production. Aucun code écrit ni modifié ; aucun accès à une base ni à un secret.
**Branche :** `ent-socle-35-36-conception`, partie d'`origin/ent-mc0hoe` (`afeb8c4`).
**Passe personas :** `PERSONAS_36.md` ; ajouts au **§30**, qui l'emporte en cas de contradiction.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` (D-017, D-019, D-031, D-034, D-035, D-036, D-037 à D-046) ; `docs/SOCLES.md` §36 ; **`34_SANTE.md`** (§5.1 secret, §5.8 signalements, §10.12 pli vers le 36, R-S1, R-S41, R-S48, R-S50 ; cabinet HDS §6.2) et `PERSONAS_34.md` ; `35_INCLUSION.md` ; `04_ADMINISTRATIF.md` (autorité parentale, interdictions, adresse protégée) ; `08_NOTIFICATIONS.md` (R-N58 : aucun avis automatique) ; `12_ASSIDUITE.md` (RA-21, absentéisme, signalement au DASEN) ; `13_DISCIPLINE.md` (RG-38, DIS-256, DIS-293, CD-54, R470) ; `14_ACCUEIL_SURETE.md` (ACS-228, ACS-241, CAS-29) ; `16_MESSAGERIE.md` (CM-30, MSG-229, M10) ; `18_FORMULAIRES.md` (questionnaires confidentiels) ; `19_AGENDA_RDV.md` (natures restreintes, CAG-12, Q-AG9) ; `24_FINANCES.md` et socle 26 (bourses) ; `IA_COEUR.md` ; `TRANSITION_T0_INVENTAIRE.md` (M18, Q-T0-11) ; `PRA_SAUVEGARDES.md` §18.4.
**Conventions.** [F], [V], [Ve], [NV], [H], [R] comme au socle 35. Numérotation propre : actions **SOC-100** et suivantes ; défauts **DSOC-01** ; règles **R-SOC1** ; cas limites **CSOC-01** ; questions **Q-SOC1** ; réglages **S-2720 à S-2739** ; lots **SO0** et suivants ; constats Next.js **NXB-01**.
**Plage de migrations imposée :** **2720–2739**, vérifiée libre sur **les 134 branches distantes** le 2026-11-11. Registre : `MIGRATIONS.md`. Le **cabinet** (base séparée, périmètre HDS, §6) a sa **propre série** de migrations, comme celle du 34.
**Méthode et limite.** Comme pour le 35 : la recherche réglementaire déléguée a été interrompue par une limite d'usage ; les textes du 34 (§5.1, §5.8, lus [V] le 2026-11-10) sont repris, complétés par des recherches ciblées le 2026-11-11 (extraits officiels, d'où des **[Ve]**). Les points à relire sur le texte entier sont au §28.

Sommaire : §0 En bref · §1 Thèse · §2 Existant (Rust et Next.js) · §3 Acteurs · §4 Réglementaire · §5 Trois cercles, trois secrets · §6 Niveaux et hébergement · §7 Frontières · §8 Concepts · §9 Partage · §10 Domaines · §11 Familles, mineur, majeur · §12 Cas limites · §13 IA · §14 Actions SOC · §15 Règles · §16 Droits · §17 Chiffrement et journal · §18 Conservation · §19 Événements · §20 Notifications · §21 Écrans et mobile · §22 Modèle de données · §23 Plan de migration · §24 Lots · §25 Migrations · §26 Tests · §27 Réglages et questions · §28 Faits, hypothèses, contradictions · §29 Priorités · §30 Ajouts personas.

---

## 0. En bref

1. **L'existant expose le psychologue à la direction et les familles au psychologue.** Le dossier, les entretiens et les rendez-vous du psychologue sont **lus et écrits par la direction et l'administrateur** (`roles.rs:180-181`, `actions.rs:1176-1179`, modèles de rôles `actions.rs:1796-1797` et `:1851-1852`) ; le **« courrier d'orientation »** — un PDF qui contient les **axes de suivi** en texte libre (« anxiété scolaire »…) — est **envoyé par courriel en pièce jointe** aux parents, **sans vérifier** que le dossier est « signalé à la famille » (`service.rs:16789-16838`, `psy_pdf.rs:27-52`) ; **tout parent au lien actif**, y compris sans autorité parentale ou pour un élève **majeur**, lit les axes de suivi et les entretiens marqués (`service.rs:16668-16700`, `foyer_repo.rs:594-616`) ; les événements publient la **nature** de l'entretien (« crise ») et sa durée (`service.rs:16540-16553`). Il n'existe **ni service social, ni information préoccupante, ni cellule**. Un **lot SO0** ferme cela avant toute démonstration.
2. **Next.js en production a un module « bien-être » à risque [F, lecture ciblée]** : un **agent conversationnel** pour élèves (Mistral) qui écoute la détresse de mineurs, en **résume** la conversation par l'IA, stocke les signalements en **fichiers JSON sur le stockage objet**, les ouvre à la **direction, au secrétariat, aux surveillants et au CPE** — pas au psychologue, qui reçoit le **résumé par courriel en clair** (NXB-01 à NXB-10). **Risque de production signalé à Florian** (Q-SOC1) ; aucune action du pilote sur Next.js. **Recommandation : ne pas reprendre l'agent conversationnel** ; reprendre le **besoin** (« je veux parler à quelqu'un ») avec un canal **humain**.
3. **La thèse (§1).** Trois professions, **trois secrets distincts** — le psychologue, l'assistant de service social, la protection de l'enfance — qui ne se mélangent pas, et **une seule obligation commune** : **protéger l'enfant**. ScolIA rend la **coordination** possible (qui suit, quand, quelle action, quelle échéance) **sans jamais déplacer un contenu** d'un cercle à l'autre, et rend la **protection** rapide, tracée, sûre : une information préoccupante se rédige, se valide, se transmet à la CRIP en quelques minutes, sans que la famille soit prévenue par une machine.
4. **Ce que tranche le réglementaire [V/Ve]** (§4) : l'**information préoccupante** est transmise **sans délai** au président du conseil départemental (CRIP), les parents étant **préalablement informés sauf intérêt contraire de l'enfant** (CASF L226-2-1) ; le **secret partagé** entre professionnels de la protection de l'enfance est limité au **strictement nécessaire** (L226-2-2) ; le **signalement au procureur** (CPP 40, danger grave) et la **levée du secret** (C. pén. 226-14) ; l'**assistant de service social** est tenu au **secret professionnel** (CASF L411-3, C. pén. 226-13) ; le **psychologue** est tenu au secret par son code de déontologie (CNCDP) [Ve] et son corps est régi par le **décret 2017-120** ; la **délibération CNIL 2018-327** impose une **AIPD** pour la gestion des alertes et signalements sociaux ; la lutte contre le **harcèlement** (circulaire du 2/02/2024, programme pHARe) relève du **socle 13** pour les situations ; le **GPDS** est placé sous l'autorité du chef d'établissement [Ve].
5. **Trois niveaux d'activation (§6)**, comme au 34 : **N-0** (défaut) : rien de psychologique ni de social, **mais l'information préoccupante et le signalement existent déjà** — c'est une obligation légale qui ne peut pas attendre un hébergeur ; **N-1 « coordination »** : rendez-vous sans motif, suivis actifs visibles du seul professionnel, aides du fonds social, cellule de veille et GPDS **sans contenu clinique**, préoccupations entrantes — après **avis du DPO** ; **N-2 « cabinet »** : notes cliniques du psychologue, bilans, comptes rendus sociaux — **uniquement dans le cabinet HDS** (base séparée du 34, cercles et clés distincts), car le CSP vise aussi le **suivi social et médico-social** (L1111-8 [NV, formulation à relire]).
6. **Ce que ScolIA apporte de plus que le marché [H]** : une préoccupation repérée par un AED, une CPE, un professeur, un message (16), une main courante (14) ou une situation de harcèlement (13) **arrive au bon cercle en un geste**, sans copie du contenu dans le socle d'origine ; l'élève peut **demander à parler à un adulte** (psychologue, assistante sociale, infirmière, CPE) **sans que ses parents en soient avertis** et sans qu'une absence ne le trahisse ; la commission du **fonds social** examine des **dossiers anonymisés** et la remise s'applique sur la facture **sans motif visible** ; l'IP est **horodatée, validée, transmise, suivie**, et la CRIP répond dans le même dossier.
7. **IA : exclue de tout le socle** (aligné R-S48 du 34). Aucune lecture, aucun résumé, aucune détection de détresse, aucun classement d'élèves « à risque ». Seule capacité : rédiger des **informations collectives** à partir de modèles validés (C0, sans donnée d'élève).
8. **Le plan.** **153 actions** (121 de SOC-100 à SOC-286, numérotation à trous, puis 32 de SOC-300 à SOC-331, §30), **50 cas limites**, **40 règles**, **14 lots** : **SO0** correctifs urgents ; **SO1** niveaux, cercles, journal ; **SO2** information préoccupante et signalement ; **SO3** préoccupations entrantes ; **SO4** rendez-vous et demandes de l'élève ; **SO5** psychologue (coordination) ; **SO6** service social et fonds social ; **SO7** cellule de veille et GPDS ; **SO8** cabinet (N-2) ; **SO9** liaisons (12, 13, 14, 16, 19, 24, 26, 37) ; **SO10** mobile ; **SO11** conservation, droits, restauration ; **SO12** reprise Next.js ; **SO13** RLS stricte et retraits. **Parcours minimal** : SO0, SO1, **SO2 dès la V1** (protection), SO3, SO4 ; N-1 après l'avis du DPO.
9. **Soumis à Florian** : risque Next.js « bien-être » et sort de l'agent conversationnel (Q-SOC1, Q-SOC2) ; avis du DPO (N-1 sans HDS, champ HDS du suivi social, AIPD, durées) ; coordonnées réelles de la CRIP du département et protocole départemental (Q-SOC5) ; référents métier (une PsyEN, une assistante sociale, un chef d'établissement) ; hébergement HDS (commun avec Q-S1 du 34).

---

## 1. Thèse : trois secrets, une obligation de protéger

1. **Le psychologue, l'assistant social et la protection de l'enfance sont trois mondes.** Ce que l'élève confie au psychologue n'est pas connu de l'assistante sociale, et inversement, sauf **échange consenti** ou **secret partagé** dans le cadre de la protection de l'enfance (L226-2-2) ; ScolIA ne fournit **aucune vue transversale** des trois (R-SOC1).
2. **La coordination sans contenu.** Pour aider un élève, l'équipe a besoin de savoir **qui s'en occupe** et **ce qui est décidé** (« l'assistante sociale reçoit la famille jeudi », « la cellule propose un tutorat »), pas **ce qui a été dit**. La coordination est codée, datée, sans texte clinique (R-SOC4).
3. **La protection d'abord, et vite.** Un enfant en danger n'attend pas un hébergeur certifié ni un réglage : l'information préoccupante et le signalement **existent dès N-0** ; ScolIA guide, trace, transmet, rappelle les numéros (119, 17, 15) et **ne remplace jamais l'appel immédiat** en cas de danger grave (R-SOC8).
4. **Personne n'est prévenu par une machine.** Ni la famille, ni l'auteur présumé, ni un camarade : aucune notification automatique d'une IP, d'un signalement, d'un rendez-vous de l'élève avec le psychologue ou l'assistante sociale (R-SOC6, R-N58 du socle 8).
5. **L'élève peut parler sans être trahi.** Une demande de rendez-vous, un entretien, une sortie de cours pour un entretien ne laissent **aucune trace** visible des parents ni des camarades : statut neutre « sortie autorisée » à l'appel (R-S35, R-S50 du 34) (R-SOC7).
6. **Le majeur décide ; le mineur est protégé.** À 18 ans, rien n'est communiqué aux parents sans l'accord de l'élève ; avant 18 ans, l'information des parents est la règle **sauf intérêt contraire de l'enfant**, décision **humaine** et motivée (R-SOC9).
7. **Ce qui n'est pas nécessaire n'est pas gardé.** Durées courtes, destructions tracées et rejouées après restauration (R-SOC23).
8. **L'IA ne lit rien de ce socle** (R-SOC20).

---

## 2. Évaluation franche de l'existant

### 2.1 ENT Rust : ce qu'il y a [F] (`origin/ent-mc0hoe`, `afeb8c4`)
| Élément | Où | Ce que ça fait |
|---|---|---|
| Entretiens | `migrations/041_psy_entretien.sql` | jour, nature (`individuel, famille, orientation, crise`), durée, **note confidentielle** (chiffrée, `champs_chiffres.rs:46`), `signale_famille` |
| Rendez-vous | `046_psy_rdv.sql` | jour, heures, nature, **motif et note interne** (chiffrés, `champs_chiffres.rs:47`), statut, `signale_famille` |
| Dossier | `051_psy_dossier.sql` | **une ligne par élève** : axes de suivi, contacts externes, note confidentielle (chiffrés, `champs_chiffres.rs:42-45`), `suivi_actif`, `signale_famille` |
| Courrier d'orientation | `domain-eleve/src/psy_pdf.rs`, `service.rs:16700-16838` | PDF des **axes de suivi** ; téléchargeable par le psychologue et par le parent si le dossier est signalé ; **envoi par courriel** aux parents en pièce jointe |
| Vues famille | `psy.rs:280-286`, `service.rs:16610-16625` | dossier sans note ni contacts ; entretiens « signalés » réduits |
| Routes | `ent-api/src/routes.rs:691-706`, `:9437-9720` | `/psy/entretiens`, `/psy/rdvs`, `/eleves/{id}/dossier-psy`, courrier PDF et envoi |
| Droits | `roles.rs:180-181`, `actions.rs:81-84`, `:1176-1179`, `:1969-1976` | `psy.lire`, `psy.saisir` : **Admin, Direction, Psy** ; le modèle « psy » a aussi `dossier.lire@etablissement` et `journal.lire@etablissement` |
| Trace | `service.rs:4134-4185` | `tracer_acces_sensible` **après** la lecture (« Consultation du dossier psychologue ») |
| Chronologie | `dossier_personnes.rs:425, 459` | « Suivi psychologique », visible du seul rôle Psy |
| RLS | `100_rls_pilote.sql:30` | ENABLE + FORCE sur les trois tables |
| Registre E12 | `registre_donnees.rs:519-521` | domaine `sante`, **`.export()`** (incluses dans l'export de portabilité de la personne) |
| Service social, IP, cellule, GPDS, fonds social | — | **rien** |

### 2.2 Défauts (à corriger avant toute démonstration)
| # | Gravité | Défaut | Conséquence | Où [F] | Lot |
|---|---|---|---|---|---|
| **DSOC-01** | **Critique** | **Direction et administrateur ont `psy.lire` et `psy.saisir`** par défaut | le chef, son adjoint, l'administrateur technique lisent les notes cliniques, les contacts externes (« CMP »), écrivent dans le dossier, envoient des courriers ; violation du secret du psychologue | `roles.rs:180-181`, `actions.rs:1176-1179`, `:1796-1797`, `:1851-1852` | SO0 |
| **DSOC-02** | **Critique** | **Courrier d'orientation envoyé par courriel en pièce jointe** (PDF avec les axes de suivi en texte libre, objet « Courrier d'orientation — Prénom Nom ») ; l'envoi ne vérifie **pas** `signale_famille` | contenu psychologique d'un mineur chez le prestataire de courriel et dans des boîtes partagées ; envoi possible d'un dossier non destiné à la famille | `service.rs:16700-16736`, `:16789-16838`, `psy_pdf.rs:27-52` | SO0 |
| **DSOC-03** | **Élevée** | **Parent : tout lien actif suffit** (ni autorité parentale, ni interdiction, ni majorité) pour lire les axes de suivi, les entretiens « signalés » et télécharger le courrier ; un seul booléen pour tout le dossier ; aucune opposition du mineur possible | un parent sans autorité ou visé par une interdiction lit le suivi psychologique ; les parents d'un élève majeur aussi | `service.rs:16584-16625`, `:16685-16698`, `:16751-16768`, `foyer_repo.rs:594-616` | SO0 |
| **DSOC-04** | Élevée | **Événements qui parlent** : nature (`crise`), jour, heures, durée, `signale_famille`, avec `eleve_id` | tout consommateur de l'outbox (chronologie, notifications, futures capacités IA, webhooks) apprend qu'un élève a eu un entretien de crise | `service.rs:16540-16553`, `:16900-16915` | SO0 |
| **DSOC-05** | Élevée | **Trace « fail-open » hors transaction**, et le libellé « Consultation du dossier psychologue » est lisible par `journal.lire` (direction, secrétariat…) | lecture sans preuve ; **l'existence** d'un suivi psychologique fuit par le journal | `service.rs:4164-4185` | SO0 |
| **DSOC-06** | Élevée | **Contenu clinique dans la base principale** (notes, axes, contacts) hors HDS, alors que le suivi psychologique est une donnée de santé (RGPD art. 4.15) et que l'HDS vise aussi le suivi médico-social [NV] | même question que N-2 au 34 ; sauvegardes et restaurations ordinaires | `041`, `046`, `051` | SO0 (gel), SO8 |
| **DSOC-07** | Moyenne | **Dossier écrasé** (`UPSERT` d'une ligne) sans historique | ce qui a été écrit puis effacé disparaît ; aucune preuve | `psy_repo.rs` (upsert), `service.rs:16627-16666` | SO0 (instantané), SO8 |
| **DSOC-08** | Moyenne | **Le psychologue lit tout le dossier élève et tout le journal** de l'établissement (`dossier.lire@etablissement`, `journal.lire@etablissement`) | lecture des sanctions, de la santé indirecte, du journal de tous les élèves sans besoin | `actions.rs:1969-1976` | SO0 |
| **DSOC-09** | Moyenne | **Registre : tables psy « exportées »** dans l'export de portabilité de la personne | risque de livrer la note clinique à la famille ou à l'élève par l'export RGPD [H : export non encore écrit] | `registre_donnees.rs:519-521` | SO0 |
| **DSOC-10** | Moyenne | **FK non composites** (`REFERENCES ent_eleve (id)`, `ent_user (id)` en `NOT NULL`) | E2 ; suppression d'un compte bloquée ou orpheline | `041:7,12`, `046:7,17`, `051:7,13` | SO0 |
| **DSOC-11** | Moyenne | **Ni rôle assistant social, ni IP, ni cellule, ni fonds social** | le circuit le plus grave (protection) n'existe pas ; il se fait par courriel | — | SO2, SO6, SO7 |
| **DSOC-12** | Faible | **Nature « crise »** stockée comme une catégorie lisible | un intitulé de crise dans un tableau d'activité | `041:15-16`, `046:17-18` | SO5 |

### 2.3 Next.js de production : module « bien-être » (lecture ciblée, aucune modification)
| # | Constat [F] | Où | Risque |
|---|---|---|---|
| NXB-01 | **Agent conversationnel** pour élèves (« assistant bien-être »), modèle Mistral appelé à chaque message ; **classification** de la détresse (niveau, catégories, suggestion de signalement) par un second appel | `app/api/bien-etre/chat/route.ts:97-110`, `app/lib/bien-etre-mistral.ts:1` | contenu de santé mentale de **mineurs** envoyé à un tiers, sans garantie contractuelle vérifiée (CLAUDE.md : Mistral « sous réserve de garanties contractuelles réelles ») ; profilage automatisé de la détresse |
| NXB-02 | Le message d'accueil dit « ce que tu dis ici n'est pas enregistré » alors que chaque message part chez le fournisseur d'IA | `bien-etre-config.ts:14`, `bien-etre-prompt.ts:17` | information trompeuse d'un mineur (RGPD art. 12–13) |
| NXB-03 | Le signalement est **résumé par l'IA** (« Résume … pour un psychologue scolaire ») | `bien-etre-storage.ts:98-118` | contenu transformé par une machine, transmis comme fait |
| NXB-04 | **Stockage en fichiers JSON** (`bien-etre/signalements/{id}.json` + `index.json`) sur le stockage objet ; index réécrit à chaque création (concurrence) | `bien-etre-storage.ts:11-45` | D-036 ; perte d'un signalement en cas d'écritures simultanées ; aucun chiffrement applicatif visible |
| NXB-05 | **Lecteurs** : directions, administratif, surveillants, CPE, administrateurs — **pas le psychologue** | `bien-etre-auth.ts:10`, `:35-50` | lecture par des non-soignants d'une détresse confiée « au psychologue » |
| NXB-06 | **Courriel au psychologue** avec prénom, classe, gravité, catégories et **résumé en clair** | `bien-etre-notify.ts:25-44` | contenu sensible chez le prestataire de courriel |
| NXB-07 | **Note du référent** en texte libre, **écrasée** à chaque modification, sans journal | `bien-etre-storage.ts:73-96`, `api/bien-etre/signalements/route.ts:28-50` | aucune trace de qui a fait quoi |
| NXB-08 | **Purge** « paresseuse » à la consultation (90 jours par défaut, 7 à 365) | `bien-etre-config.ts:12`, `api/bien-etre/signalements/route.ts:15` | rien n'est purgé si personne ne consulte ; aucune durée fondée |
| NXB-09 | Le prompt cite « le **116** (écoute enfants) » | `bien-etre-prompt.ts:15` | numéro incomplet : le bon numéro de l'enfance en danger est le **119** ; 116 111 est le numéro européen d'écoute des enfants [NV] |
| NXB-10 | Identité saisie par l'élève (prénom, classe), sans lien vérifié au compte | `api/bien-etre/signaler/route.ts:28-49` | impossibilité de retrouver un élève en danger, ou usurpation |

**Verdict D-035** : module **en usage** [H, à confirmer : Q-T0-11], mais **ni fiable ni conforme** : on **ne reprend pas** l'agent conversationnel ni le résumé par IA ; on reprend le **besoin** : (a) l'élève peut **demander à parler** à un adulte désigné (SOC-150) ; (b) une **préoccupation** arrive au bon cercle (SOC-130) ; (c) les numéros **3018** et **119** sont affichés partout où l'élève écrit. Les signalements existants sont **repris à la main** par le psychologue, s'il le décide, puis détruits par décision de Florian (SO12). **Changement d'usage signalé** (Q-SOC2).

### 2.4 Verdict sur l'existant Rust
- **Garder** : le chiffrement par champ ; les vues famille réduites ; la RLS ; la chronologie limitée au rôle Psy ; la séparation rendez-vous / entretiens.
- **Corriger (SO0)** : DSOC-01 à DSOC-05, DSOC-07 (instantané), DSOC-08, DSOC-09, DSOC-10 ; **geler** l'écriture des notes cliniques dans la base principale (DSOC-06) : les entretiens restent saisissables **sans note** (date, durée, nature neutre) jusqu'au cabinet.
- **Remplacer** : le booléen `signale_famille` par des **communications** explicites, datées, à un titulaire désigné ; le courrier par courriel par une **mise à disposition dans l'ENT**, enveloppe neutre ; la ligne de dossier par le **cabinet** (N-2).

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Psychologue de l'EN (EDO au second degré)** [V, décret 2017-120] — ou psychologue salarié dans le privé [H] | entretiens, bilans, conseil en orientation, participation aux équipes | plusieurs établissements, agenda papier, demandes par couloir | que la direction lise ses notes ; que son bilan circule | un agenda discret, des demandes qui arrivent sans que l'élève ait à se justifier, un cabinet à lui |
| **Assistant(e) de service social** | entretiens élèves et familles, aides financières, absentéisme, protection | dossiers de fonds social papier ; relances | une liste d'élèves « aidés » qui circule | dossier de fonds social en ligne, commission anonymisée, remise appliquée sans tableau partagé |
| **Chef d'établissement** | responsable de la transmission des IP et des signalements ; préside la commission du fonds social, la cellule, le GPDS | rédaction d'IP dans l'urgence, modèles introuvables | une IP non transmise ; une famille prévenue à tort | un circuit guidé : rédiger, valider, transmettre, suivre ; un tableau des situations sans contenu |
| **CPE** | repère, alerte, participe au GPDS et à la cellule | « je ne sais pas si quelqu'un s'en occupe » | être le dernier à savoir ; trahir un élève | voir **qu'un suivi existe** (oui / non, par qui) pour les élèves dont il a la charge, jamais pourquoi |
| **Infirmière (34)** | repère, rédige des IP, coordonne avec la psychologue | — | que son pli devienne un dossier partagé | un **pli** vers le 36 sans copie (34 §10.12) |
| **Professeur, AED, AESH** | repère, recueille une révélation | ne savoir à qui le dire | mal faire, être cité | un bouton « signaler une préoccupation », puis **plus rien à porter** |
| **Accueil, agents** | voient des choses (bleus, parent menaçant) | — | — | idem, depuis la main courante (14) |
| **Élève mineur** | souffre, hésite, demande de l'aide | se justifier pour sortir de cours | que ses parents ou ses camarades sachent | **demander discrètement** un rendez-vous ; savoir ce qui sera dit à ses parents et ce qui devra être signalé |
| **Élève majeur** | idem | — | que ses parents soient informés | décider seul |
| **Titulaires de l'autorité parentale** | demandent un rendez-vous, sont reçus, demandent une aide | formulaires papier du fonds social | être jugés ; qu'on leur cache quelque chose | demande d'aide en ligne, confidentielle, et une réponse claire |
| **Parent visé par une IP** | — | — | — | rien ne doit lui parvenir par ScolIA (R-SOC6) |
| **CRIP du département, parquet** | reçoivent, évaluent, répondent | IP incomplètes, illisibles | IP sans faits datés, mêlées d'interprétations | une IP structurée (faits, propos rapportés entre guillemets, dates, contexte, mesures prises), transmise par le canal du département |
| **Gestionnaire, comptable (24)** | applique les aides sur les factures | savoir pourquoi | une remise sans pièce | une **décision** de commission, numérotée, sans motif, appliquée sur la facture |
| **DPO** | registre, AIPD | données éparpillées (mails, Excel, JSON) | une fuite d'IP | cercles testés, journal, durées, aucune IA |

---

## 4. Ce qui est réglementaire (vérifié le 2026-11-11)

### 4.1 Protection de l'enfance
- **CASF L226-2-1** (en vigueur depuis le 22/03/2015) [Ve, fragment Légifrance et guide éduscol] : les personnes qui mettent en œuvre la politique de protection de l'enfance et celles qui y concourent **transmettent sans délai** au président du conseil départemental, ou au responsable désigné, toute information préoccupante sur un mineur en danger ou risquant de l'être (art. 375 C. civ.) ; **sauf intérêt contraire de l'enfant**, les titulaires de l'autorité parentale ou le tuteur sont **préalablement informés** de cette transmission, selon des modalités adaptées.
- **CASF L226-3** [Ve] : la **cellule départementale** (CRIP) recueille, traite et évalue les IP. **D226-2-6** [Ve] : l'IP est une information transmise à la cellule pour alerter le président du conseil départemental sur la situation d'un mineur.
- **CASF L226-2-2** [Ve, 34 §5.8] : **secret partagé** entre personnes soumises au secret qui mettent en œuvre ou concourent à la protection de l'enfance, **strictement limité** à ce qui est nécessaire ; information préalable des titulaires sauf intérêt contraire.
- **CPP art. 40** al. 2 [V via 34 §5.8 ; abrogé au 1/01/2029 par l'ordonnance 2025-1091] : tout fonctionnaire qui acquiert la connaissance d'un crime ou d'un délit en avise sans délai le procureur. **C. pén. 226-14** [V via 34] : levée du secret pour signaler des sévices ou privations ; **434-3** [NV] : non-dénonciation de mauvais traitements sur mineur.
- **C. éduc. L542-1, L542-3** [V via 34] : formation et information sur la protection de l'enfance.
- **Éduscol, « Enfants en danger : comment les repérer et que faire ? »** [Ve] : repérage, IP à la CRIP, signalement au procureur en cas de danger grave et immédiat.
- **119** (Allô enfance en danger, SNATED) [NV, numéro notoire] ; **3018** (harcèlement et cyberharcèlement) [Ve].

### 4.2 Assistant de service social
- **CASF L411-3** et **C. pén. 226-13** [Ve, documents académiques] : secret professionnel de l'assistant de service social.
- **Circulaire 2017-055 du 22/03/2017** (BO n° 12 du 23/03/2017) [Ve] : service social en faveur des élèves, service spécialisé de l'Éducation nationale sous l'autorité de l'IA-DASEN ; priorités : prévention de l'échec, de l'**absentéisme et du décrochage**, protection de l'enfance, accès aux droits ; écoute des élèves qui le souhaitent **dans le respect du secret professionnel**.
- **Fonds sociaux** (collégien, lycéen, cantine) [NV, circulaire de référence non relue] : commission présidée par le chef d'établissement, avis de l'assistante sociale [H].

### 4.3 Psychologue
- **Décret 2017-120 du 1/02/2017** [Ve] : corps des psychologues de l'Éducation nationale (spécialités EDA et EDO), missions à l'article 3.
- **Code de déontologie des psychologues** (révisé, avis de la CNCDP) [Ve] : le secret professionnel « doit être préservé par le psychologue quel que soit son cadre de travail » (art. 7) ; il apprécie avec discernement la conduite à tenir entre secret et assistance à personne en péril. **Statut juridique** du code (non réglementaire) et soumission à l'art. 226-13 : [NV] → **à faire confirmer par un juriste** (Q-SOC9).
- **Accès des parents** au dossier psychologique, consentement à un bilan : **aucun texte trouvé** [NV] → règles prudentes §11 (Q-SOC10).

### 4.4 Décrochage, absentéisme
- **GPDS** : placé sous l'autorité du chef d'établissement, présenté au conseil d'administration, dispositif permanent de repérage [Ve, fiches académiques] ; circulaires académiques 2025-2026 demandant un fonctionnement actif [Ve].
- **Absentéisme** : **C. éduc. L131-8** (motifs légitimes) ; circulaire interministérielle **2014-159 du 24/12/2014** [Ve] ; seuil de signalement au DASEN (quatre demi-journées dans le mois) confirmé pour le **premier degré** seulement dans les sources vues [Ve] — **à vérifier pour le second degré** (socle 12, RA-21).

### 4.5 Harcèlement
- **Circulaire du 2/02/2024** (abroge la circulaire 2013-100) : programme **pHARe** et protocole national dans chaque école, collège et lycée ; **équipe ressource** (cinq personnes par collège) [Ve] ; délit de harcèlement scolaire (loi 2022-299, C. pén. 222-33-2-3) [NV]. *Conséquence* : les **situations** sont au **socle 13** ; le 36 reçoit les **transmissions** (IP, besoin de suivi) sans dupliquer la situation.

### 4.6 Données
- **Délibération CNIL 2018-327** [V via 34] : AIPD pour l'accompagnement social ou médico-social et pour la **gestion des alertes et signalements** ; élèves = personnes vulnérables → **AIPD obligatoire** [R : certitude forte].
- **RGPD art. 9** (santé, y compris santé mentale) ; **art. 10** (infractions) pour les signalements au procureur [H] ; **art. 22** (pas de décision fondée exclusivement sur un traitement automatisé) — contre l'agent de Next.js qui classe la détresse.
- **CSP L1111-8** [V via 34 §5.2] : certificat HDS pour l'hébergement de données de santé ; le champ de l'article viserait les données « recueillies à l'occasion d'activités de prévention, de diagnostic, de soins **ou de suivi social et médico-social** » [NV : formulation à relire] → **question au DPO** (Q-SOC3).

### 4.7 Ce que les textes ne disent pas (à faire trancher)
1. Le dossier du psychologue scolaire relève-t-il de l'HDS ? → **traité comme tel** (N-2 dans le cabinet), avis du DPO.
2. La **copie** d'une IP transmise doit-elle être gardée par l'établissement, combien de temps, et où ? → copie chiffrée, accès restreint, durée courte (§18), avis du DPO.
3. Consentement au **suivi** psychologique régulier d'un mineur (un titulaire ? les deux ?) → un titulaire informé suffit pour un suivi, les deux pour un bilan psychométrique, sauf urgence [H, Q-SOC10].
4. Privé sous contrat : à qui transmet le chef d'établissement (même CRIP), qui sont les psychologues et assistants sociaux (salariés, conventions) [NV].

---

## 5. Trois cercles, trois secrets [R]

| Cercle | Membres | Ce qu'il voit | Ce qu'il ne voit jamais |
|---|---|---|---|
| **Cercle psy** | psychologue(s) de l'établissement, remplaçant désigné pour une période | ses rendez-vous, ses entretiens, son cabinet (N-2), les demandes qui lui sont adressées | le social, la santé (34), les IP qu'il n'a pas rédigées ou reçues |
| **Cercle social** | assistant(e)(s) de service social | ses entretiens, les dossiers d'aide, son cabinet (N-2), les demandes | le psy, la santé |
| **Cellule protection** | chef d'établissement (ou adjoint désigné), **une à trois personnes** désignées (CPE, ASS, infirmière, psychologue) | les IP et signalements de l'établissement, leur circuit, les retours | les cabinets |
| **Cellule de veille / GPDS** | membres désignés (direction, CPE, PP, ASS, infirmière, psychologue, COP, MLDS) | les **situations** inscrites (élève, motif codé non clinique, actions, échéances, référent) | tout contenu clinique ou social ; les IP |
| **Commission du fonds social** | chef, gestionnaire, ASS, représentants désignés | les **dossiers anonymisés** (numéro, composition du foyer, ressources par tranches, demande, avis de l'ASS) | l'identité, sauf pour l'ASS et le gestionnaire au moment de l'application |

**Règle** : une personne peut appartenir à plusieurs cercles, mais **chaque écran ne montre qu'un cercle à la fois**, avec la casquette active (R-SOC1).

---

## 6. Niveaux d'activation et hébergement [R]

| Niveau | Contient | Hébergement | Qui active |
|---|---|---|---|
| **N-0 — par défaut** | **information préoccupante et signalement** (rédaction, validation, transmission, retours, copie chiffrée), préoccupations entrantes vers la cellule protection, numéros d'urgence, demande « parler à un adulte » **sans motif** | base principale, **clé « protection »** distincte, RLS stricte | — (actif par défaut : obligation légale) |
| **N-1 — coordination** | rendez-vous psy et social **sans motif**, entretiens **sans note** (date, durée, nature neutre), suivis actifs (oui / non) lisibles du seul professionnel et, en pastille, de la cellule de veille, cellule de veille et GPDS à motifs codés **non cliniques**, fonds social (demandes, commission anonymisée, décisions), demandes de l'élève, communications aux familles | base principale ; **avis écrit du DPO** avant activation | direction + attestation DPO (★ N3) |
| **N-2 — cabinet** | notes cliniques, bilans psychologiques, comptes rendus d'entretiens sociaux, écrits, pièces | **cabinet HDS** du 34 (base séparée, service séparé), **cercles et clés distincts** (`cab_psy_*`, `cab_soc_*`) ; refus serveur sans déclaration HDS | direction + professionnel titulaire + attestation HDS et DPO (★ N3) |

**Public / privé (D-019)** : dans le public, les PsyEN et ASS peuvent avoir leurs propres outils académiques [NV] : N-2 désactivé par défaut dans un EPLE (comme R-S31), réglage S-2720. Dans le privé, N-2 est l'offre complète après HDS.

**Existant** : à SO0, les notes cliniques existantes (041, 051) restent chiffrées, **lisibles du seul psychologue**, et ne peuvent plus être enrichies ; elles sont **transférées au cabinet** en SO8 puis effacées de la base principale, ou **détruites** si l'établissement n'active jamais N-2 (décision du psychologue, procès-verbal sans contenu).

---

## 7. Frontières

### 7.1 Qui possède quoi
| Donnée | 36 | Autres |
|---|---|---|
| IP, signalements, circuit, retours | **possède** | 34, 13, 14, 16, 12 **proposent** par pli ou préoccupation |
| Rendez-vous psy / social (contenu, motif) | **possède** | 19 ne lit que l'**occupation** (Q-AG9) |
| Cabinet psy, cabinet social | **possède** (N-2) | — |
| Cellule de veille, GPDS (situations, actions) | **possède** | 12 fournit des indicateurs d'assiduité ; 21 des résultats ; 13 des faits **sans contenu** |
| Fonds social (demandes, commission, décisions) | **possède** | 24 applique la remise ; 26 bourses (lien d'éligibilité) |
| Situations de harcèlement (pHARe) | — | **13 possède** ; transmet au 36 si protection |
| Absentéisme et signalement au DASEN | — | **12 possède** ; GPDS au 36 |
| Orientation | — | **37 possède** ; le psychologue EDO y contribue (entretiens d'orientation sans contenu clinique) |
| Bilans psychologiques pour une orientation ULIS / SEGPA | cabinet (36) | 35 ne reçoit que la **décision** (notification) |

### 7.2 Règles de frontière
1. **Aucun socle ne copie le contenu d'une préoccupation** : le socle d'origine garde son fait (main courante, message gelé, situation de harcèlement) et transmet un **pli** (référence + texte de transmission) au 36 (R-SOC10).
2. **Le 36 ne renvoie jamais un contenu** au socle d'origine : seulement « pris en compte » (sans détail) à l'émetteur s'il le demande (R-SOC11).
3. **Le 19 n'affiche que « occupé »** pour un rendez-vous psy / social / protection (19 §559).
4. **L'appel ne connaît qu'une sortie autorisée neutre** (R-S35, R-S50 du 34).

### 7.3 Ports exposés [R]
- `suivi::preoccupation(eleve, origine, texte_transmis) -> PliId` (12, 13, 14, 16, 34, 35) — ajout seul.
- `suivi::suivi_existe(eleve, lecteur) -> {oui(par_qui_fonction) | inconnu}` (CPE, PP dans la cellule) — **jamais** la nature psy / social au-delà de la fonction, et **jamais** pour une IP.
- `suivi::occupation(agent, periode)` (19).
- `suivi::sortie_entretien(eleve, seance) -> SortieAutorisee` (12).
- `suivi::aide_decidee(dossier_anonyme) -> Remise{montant, facture}` (24).
- `suivi::statistiques(etab, annee)` (47) — agrégats, petits effectifs masqués, **aucun** chiffre d'IP par classe.

### 7.4 Ports consommés
`eleves` (2) ; `responsables`, `autorite`, `interdictions`, `adresse_protegee`, `majorite` (4) ; `agents`, `fonctions`, `missions` (6) ; `documents.coffre` C4 (7) ; `notifications` (8, natures restreintes) ; `assiduite.indicateurs` (12) ; `discipline.faits_sans_contenu` (13) ; `rendez_vous` (19) ; `factures.remise` (24) ; `bourses.eligibilite` (26) ; `sante::pli` (34).

---

## 8. Concepts [R]
| Terme | Sens |
|---|---|
| **Préoccupation** | ce qu'un adulte a vu, entendu ou reçu et qui l'inquiète ; transmise à la cellule protection ou au professionnel choisi ; texte court, faits, sans interprétation |
| **Information préoccupante (IP)** | écrit structuré transmis à la CRIP ; versions, validation, transmission, retours ; copie chiffrée |
| **Signalement** | écrit transmis au procureur (danger grave, infraction) ; copie à la CRIP [H, pratique] |
| **Demande** | demande de rendez-vous d'un élève, d'une famille, d'un adulte, adressée à un cercle |
| **Entretien** | rencontre réalisée (date, durée, nature neutre : `individuel`, `famille`, `orientation`, `urgence`) ; contenu au cabinet seulement |
| **Suivi** | engagement d'un professionnel auprès d'un élève (actif / clos) |
| **Situation de veille** | inscription d'un élève à la cellule de veille ou au GPDS : motif codé non clinique, actions, référent, échéances |
| **Dossier d'aide** | demande au fonds social, anonymisée pour la commission |
| **Communication** | ce qu'un professionnel choisit de dire à un titulaire, explicitement, daté (remplace `signale_famille`) |

---

## 9. Partage strictement minimal

### 9.1 Matrice [R]
| Information | Psy | ASS | Cellule protection | Cellule veille / GPDS | CPE (ses élèves) | Prof / PP | Direction (hors cellule) | Secrétariat | Infirmière | Titulaires | Élève |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Cabinet psy (notes, bilans) | ✔ | — | — | — | — | — | — | — | — | sur demande, sous réserve (§11) | sur demande |
| Cabinet social | — | ✔ | — | — | — | — | — | — | — | idem | idem |
| Rendez-vous (motif) | le sien | le sien | — | — | — | — | — | — | — | s'il est convoqué | le sien |
| Rendez-vous (occupation) | ✔ | ✔ | — | — | — | — | — | — | — | — | — |
| Suivi existe (fonction) | ✔ | ✔ | — | ✔ (membres) | ✔ si l'élève l'accepte ou s'il est en cellule [R] | — | — | — | — | — | ✔ |
| IP / signalement | rédacteur | rédacteur | ✔ | — | rédacteur | rédacteur (le sien, statut) | — | — | rédacteur | **jamais par ScolIA** (information humaine) | — |
| Situation de veille | membre | membre | — | ✔ | ✔ si membre | PP si membre | ✔ si membre | — | membre | informés de l'inscription [R] | ✔ |
| Dossier d'aide (identité) | — | ✔ | — | — | — | — | — | — | — | le sien | — |
| Dossier d'aide (anonyme) | — | ✔ | — | — | — | — | commission | — | — | — | — |
| Remise appliquée | — | — | — | — | — | — | gestionnaire | gestionnaire (montant, sans motif) | — | ✔ (sur la facture : « aide de l'établissement ») | — |

### 9.2 Règles
1. **Aucune vue transversale** des trois secrets ; aucune recherche par mot dans les cabinets hors du titulaire.
2. **Le rédacteur d'une IP** garde la lecture de **son** IP et de son **statut** (« transmise le … »), pas des suites détaillées (R-SOC12).
3. **L'élève peut refuser** que le CPE sache qu'un suivi existe, sauf situation de veille ou de protection (R-SOC13).
4. **Agrégats** : petits effectifs (< 5) masqués ; aucun chiffre d'IP par classe ou par niveau hors de la cellule protection.

---

## 10. Les domaines un par un [R]

### 10.1 Préoccupations entrantes (N-0)
Bouton « **Signaler une préoccupation** » partout (fiche élève, appel, message, main courante, situation de harcèlement, passage à l'infirmerie) ; formulaire court : faits datés, propos rapportés **entre guillemets**, contexte, mesures déjà prises, **sans** interprétation ni diagnostic (aide à la rédaction, non IA) ; destinataire : **cellule protection** par défaut, ou un professionnel choisi (psy, ASS, infirmière) ; accusé de réception ; l'émetteur garde « transmis le … » ; le socle d'origine **ne garde pas** le texte (R-SOC10). **Danger immédiat** : l'écran affiche d'abord « Appelez le 17 / le 15 / le 119 maintenant », puis enregistre.

### 10.2 Information préoccupante (N-0)
1. **Ouverture** depuis une préoccupation, un pli du 34, un fil gelé du 16, une situation du 13, une main courante du 14, ou directement.
2. **Rédaction** sur le modèle du département (S-2721 : modèle national par défaut, modèle départemental chargé par l'établissement) : identité de l'enfant et des titulaires (du socle 4, **adresse protégée jamais reprise automatiquement**), faits, propos rapportés, contexte, actions de l'établissement, rédacteur(s).
3. **Validation** par le chef d'établissement ou son délégué de la cellule (★ N3 : réauthentification passkey) ; **co-rédaction** possible ; aucun avis à l'auteur présumé.
4. **Information des titulaires** : décision **humaine**, tracée : « informés le … par … » / « non informés : intérêt contraire de l'enfant, motif codé » (L226-2-1) ; ScolIA n'envoie **rien** (R-SOC6).
5. **Transmission** à la CRIP par le **canal du département** (S-2722 : courriel sécurisé de la CRIP, formulaire en ligne, courrier) ; ScolIA produit le PDF signé (empreinte), le transmet par le canal choisi ou le met à disposition pour un dépôt manuel, enregistre la date et le moyen, et **joint l'accusé** reçu.
6. **Retours** de la CRIP (L226-5 [NV] : information de l'émetteur) : saisis, pièces au coffre C4 ; échéance de relance (S-2723).
7. **Copie** : chiffrée avec la clé « protection », lisible de la cellule et du rédacteur, conservée selon §18, **jamais** dans la chronologie de l'élève, **jamais** exportée hors procédure.

### 10.3 Signalement au procureur (N-0)
Même circuit, destinataire **procureur de la République** ; rappel des textes (CPP 40, 226-14, 434-3) ; en cas de danger grave et immédiat : appel au 17 et signalement **sans délai** ; copie à la CRIP [H] ; trace.

### 10.4 Demandes de l'élève et de la famille (N-0 sans motif, N-1 complet)
- **« Je voudrais parler à quelqu'un »** (élève) : choix du destinataire (psychologue, assistante sociale, infirmière, CPE, « un adulte de confiance » de la liste de l'établissement) ; **aucun motif exigé** ; aucun avis aux parents ; réponse sous délai (S-2724) ; l'élève voit avant d'envoyer **ce qui reste confidentiel et ce qui devra être signalé** (danger) ; numéros 3018 / 119 visibles.
- **Rendez-vous** : créneau proposé par le professionnel ; **sortie de cours** par billet neutre (« sortie autorisée ») envoyée au professeur de la séance (R-S50 du 34) ; agenda famille et camarades : rien.
- **Famille** : demande de rendez-vous à l'ASS ou au psychologue ; convocation par le professionnel.

### 10.5 Psychologue (N-1 coordination, N-2 cabinet)
Agenda (multi-établissements : une casquette par établissement) ; entretiens (date, durée, nature neutre) ; suivi actif ; demandes reçues ; bilans (consentement des titulaires enregistré, §11) ; **conseil en orientation** (avec le 37 : entretiens d'orientation sans contenu clinique, ressources) ; participation aux équipes (35, cellule) ; **communications** aux familles explicites (remplace `signale_famille`) dans l'ENT, enveloppe neutre ; **aucun courrier par courriel** ; cabinet en N-2.

### 10.6 Service social (N-1, N-2)
Entretiens (élève, famille) ; accompagnement à l'accès aux droits (liste de démarches codée) ; **fonds social** : demande (en ligne par la famille ou saisie par l'ASS), pièces (justificatifs au coffre), **anonymisation** pour la commission (numéro, tranches de ressources, composition du foyer, demande, avis), **commission** (ordre du jour, décisions, montants, procès-verbal), **application** par le gestionnaire (remise sur facture, socle 24, libellé neutre « aide de l'établissement »), **information** de la famille ; suivi de l'absentéisme avec le 12 (dossier de suivi) ; protection (IP).

### 10.7 Cellule de veille et GPDS (N-1)
**Inscription** d'un élève (motif codé non clinique : assiduité, résultats, comportement, isolement signalé, situation familiale difficile, risque de décrochage, autre ; **interdit** : tout code de vigilance psychique, aligné R-S4 étendu du 34) ; **référent** de situation ; **actions** (tutorat, entretien avec la famille, orientation vers le psychologue ou l'ASS, aménagement d'EDT, PPRE (35), stage de remobilisation, liaison MLDS / PSAD) ; **réunions** (ordre du jour, relevé de décisions codé, aucun compte rendu clinique) ; **indicateurs** (assiduité du 12, résultats du 21 que le membre voit déjà) ; information des titulaires de l'inscription [R] et association ; **clôture** (motif codé) ; rien ne suit l'élève hors de l'établissement.

### 10.8 Communications et documents
Une **communication** est un message explicite d'un professionnel à un **titulaire désigné** (ou à l'élève majeur) : contenu choisi par le professionnel, dans l'ENT, enveloppe neutre, accusé de lecture ; **jamais** une pièce jointe par courriel (corrige DSOC-02).

### 10.9 Pilotage
Volumes agrégés : demandes, délais de réponse, rendez-vous, aides (nombre, montant total), situations de veille, **IP transmises (nombre annuel, cellule seulement)** ; petits effectifs masqués ; aucun nom.

---

## 11. Familles, mineur, majeur [R]
1. **Titulaires** : reçoivent les convocations, les communications explicites, peuvent demander un rendez-vous ou une aide ; **chacun pour soi** ; jamais l'adresse de l'autre.
2. **Parent sans autorité, parent visé par une interdiction, auteur présumé d'une IP** : rien ; aucune convocation automatique ; **jamais** d'information du suivi.
3. **Information des titulaires avant une IP** : décision humaine motivée (§10.2).
4. **Mineur** : peut demander un entretien **sans information des parents** ; un **suivi régulier** est porté à la connaissance d'un titulaire par le professionnel quand il le juge conforme à l'intérêt de l'élève ; un **bilan** psychométrique requiert l'accord des titulaires (sauf urgence) [H, Q-SOC10] ; l'élève peut **s'opposer** à une communication à ses parents (enregistrée ; le professionnel décide, sauf obligation).
5. **Majeur** : décide seul ; aucune communication aux parents sans son accord ; peut rouvrir un accès (convocations), révocable.
6. **Droit d'accès** (RGPD art. 15) : traité par le professionnel titulaire avec le DPO ; communication du dossier **sous réserve des droits d'autrui** (propos de tiers, IP) ; une IP n'est pas communiquée au titulaire qui en est l'objet sans avis juridique [H, Q-SOC11].
7. **Décès** : gel, aucune relance.

---

## 12. Cas limites (CSOC-01 à CSOC-38 ; ajouts personas au §30)

| # | Cas | Réponse [R] |
|---|---|---|
| CSOC-01 | Révélation de violences par un élève à 16 h 50 un vendredi | écran « danger immédiat ? » → 17 / 119 ; sinon IP rédigée et validée par le délégué de la cellule ; transmission le soir même par le canal du département |
| CSOC-02 | Le chef d'établissement est l'auteur présumé | la cellule désigne un délégué (adjoint) ; S-2725 : « transmission directe à la CRIP par tout membre de la cellule » ; le chef n'est pas informé ; journal invisible de lui |
| CSOC-03 | L'auteur présumé est un membre du personnel | idem ; signalement au procureur et à l'autorité académique ; aucun accès de la personne au 36 (suspension de casquettes) |
| CSOC-04 | Le parent auteur présumé appelle pour savoir « ce qui se passe » | rien n'est confirmé (R-S36 du 34) ; l'accueil ne voit rien |
| CSOC-05 | Deux adultes rédigent chacun une IP pour le même enfant | regroupement proposé dans la cellule ; une IP, deux rédacteurs |
| CSOC-06 | Élève qui demande « ne le dites à personne » puis révèle un danger | l'écran de demande l'avait dit ; IP ; l'élève est informé de la transmission par un adulte |
| CSOC-07 | IP transmise, la CRIP ne répond pas | relance à l'échéance (S-2723) ; aucun envoi automatique à la CRIP |
| CSOC-08 | Élève qui change d'établissement après une IP | rien n'est transmis par ScolIA ; la CRIP assure la continuité ; la copie suit les durées du §18 |
| CSOC-09 | Élève majeur victime de violences | pas d'IP (majeur) ; orientation vers les dispositifs adultes ; signalement au procureur avec son accord sauf exceptions (226-14) [NV] |
| CSOC-10 | Psychologue partagé entre deux établissements de deux organisations | une identité, une casquette par établissement ; cabinets séparés |
| CSOC-11 | Psychologue absent trois mois | remplaçant désigné pour la période ; ancien accès suspendu ou maintenu selon le titulaire |
| CSOC-12 | Psychologue qui quitte l'établissement | cabinet remis au successeur (s'il est désigné) ou détruit à l'échéance ; jamais à la direction |
| CSOC-13 | Élève qui voit l'ASS pour une aide et le psychologue pour un mal-être | deux cercles, deux secrets ; aucun lien automatique |
| CSOC-14 | Famille qui demande une aide au fonds social et refuse que l'ASS voie ses ressources | dossier remis à la commission par l'ASS seule (circuit) ; refus → dossier incomplet, la famille est informée |
| CSOC-15 | Membre de la commission qui reconnaît une famille malgré l'anonymat | rappel de confidentialité ; retrait de la commission pour ce dossier (déclaration de lien) |
| CSOC-16 | Remise appliquée sur une facture partagée entre deux foyers | aide affectée au foyer demandeur seulement (24) |
| CSOC-17 | Élève inscrit au GPDS dont les parents s'opposent | inscription maintenue (dispositif de l'établissement) [H] ; actions avec la famille ; l'opposition est notée |
| CSOC-18 | Élève de 3e en décrochage absent depuis trois semaines | GPDS + 12 (signalement d'absentéisme) ; ASS ; si danger : IP |
| CSOC-19 | Préoccupation envoyée depuis un message (16) d'un élève sur un camarade | le fil reste gelé au 16 ; le 36 reçoit le pli ; aucune information au camarade |
| CSOC-20 | Situation de harcèlement (13) qui révèle des violences familiales | transmission au 36 (DIS-256) ; la situation reste au 13 |
| CSOC-21 | Passage à l'infirmerie (34) avec traces de coups | pli de l'infirmière (34 §10.12) → IP ; le 34 garde le passage |
| CSOC-22 | Enfant qui refuse de partir avec la personne autorisée (14, CAS-29) | main courante → proposition d'IP au 36 ; jamais automatique |
| CSOC-23 | Question à l'IA « quels élèves voient la psychologue ? » | refus ; aucun port du 36 dans un contexte IA |
| CSOC-24 | Restauration de sauvegarde après destruction d'une copie d'IP | destruction rejouée avant réouverture |
| CSOC-25 | Export complet demandé par l'OGEC (D-041) | 36 exclu ; procédure particulière à part, initiée par le professionnel titulaire ou le DPO |
| CSOC-26 | Contrôle de la CNIL | journal sans contenu ; contenu en présence du professionnel |
| CSOC-27 | Réquisition judiciaire d'une copie d'IP | procédure : chef d'établissement et DPO ; journal ; aucune extraction par le support ScolIA |
| CSOC-28 | Élève qui écrit à la psychologue à 23 h des idées suicidaires | message hors horaires : réponse automatique **humaine préécrite** affichée avant l'envoi (« si tu es en danger maintenant, appelle le 15 ou le 3114 [NV] ») ; aucune analyse automatique du contenu |
| CSOC-29 | Rendez-vous psy pendant un contrôle | billet neutre ; le professeur ne connaît pas le motif ; épreuve à rattraper (21) |
| CSOC-30 | Parent qui demande la liste des rendez-vous de son enfant avec la psychologue | communication par le professionnel seulement ; droit d'accès traité avec le DPO |
| CSOC-31 | Élève en internat (41) qui révèle quelque chose la nuit | surveillant de nuit : bouton « préoccupation » ; danger : 17 / 15 / astreinte de direction |
| CSOC-32 | Fausse IP malveillante dénoncée | l'IP reste (preuve) ; rectification par ajout ; aucune suppression |
| CSOC-33 | Erreur d'élève dans une préoccupation (homonyme) | correction par ajout ; transfert vers le bon élève ; trace lisible de la cellule |
| CSOC-34 | Le département change de modèle d'IP | modèle versionné (S-2721) ; les IP déjà transmises gardent leur version |
| CSOC-35 | Décès d'un élève par suicide | cellule de crise (14) ; soutien collectif (psychologue) ; aucune donnée du cabinet partagée ; information collective validée |
| CSOC-36 | Élève qui demande un rendez-vous pour un camarade | demande « pour un camarade » sans nom obligatoire ; le professionnel décide |
| CSOC-37 | Démo de décembre 2026 | aucune donnée réelle ; IP fictive ; N-1 et N-2 non activés sur le labo partagé |
| CSOC-38 | Ancien signalement bien-être de Next.js | repris à la main par le psychologue, si utile, en préoccupation ; destruction de l'original par décision de Florian |

---

## 13. Intelligence artificielle [R]
1. **Aucune capacité IA sur le socle 36**, à aucun niveau : ni lecture, ni résumé, ni classement, ni détection de détresse, ni suggestion d'IP (R-SOC20). Les contextes IA de **tous** les socles filtrent les ports du 36 ; un test échoue sinon.
2. **Seule capacité** : `suivi.rediger_information_collective` (C0 : modèles validés, aucune donnée d'élève, N3 pour diffusion) — par exemple une lettre aux familles sur l'existence du service social.
3. **Réponse type** : « je n'ai pas accès au suivi social, psychologique ni à la protection de l'enfance ; si un enfant est en danger, appelez le 119 ; voici l'écran qui permet de signaler une préoccupation ».
4. **Next.js** : l'agent « bien-être » n'est pas repris (Q-SOC2) ; son remplacement est humain (§10.4).

---

## 14. Actions métier (SOC-100 et suivantes)

Abréviations : PSY psychologue ; ASS assistant(e) de service social ; CP membre de la cellule protection ; CHEF chef d'établissement ou délégué ; CV membre de la cellule de veille / GPDS ; COM membre de la commission du fonds social ; GES gestionnaire ; CPE ; PRF professeur ; AED ; INF infirmière ; AGT tout personnel ; PAR titulaire ; ELV élève ; ELM élève majeur ; DPO ; SYS.

### 14.1 Paramétrage, cercles, niveaux
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SOC-100 | Composer la cellule protection (1 à 3 membres + chef ou délégué) | CHEF · `suivi.cellule_protection.gerer` | N-0 | ne peut être vide ; suppléant obligatoire | `suivi.cellule_modifiee` |
| SOC-101 | Désigner les membres du cercle psy et social (fonctions du 6) | CHEF · `suivi.cercles.gerer` | N-1 | la direction ne s'inscrit pas | `suivi.cercle_modifie` |
| SOC-102 | Désigner un remplaçant pour une période | titulaire, CHEF · idem | N-1 | daté | `suivi.cercle_modifie` |
| SOC-103 | Demander, attester, activer un niveau | CHEF · `suivi.niveau.gerer` ★ ; DPO · `suivi.niveau.attester` | — | refus serveur sans attestation | `suivi.niveau_*` |
| SOC-104 | Saisir la CRIP et le parquet du département (coordonnées, canal, modèle) | CHEF · `suivi.parametrer` | N-0 | S-2721, S-2722 | — |
| SOC-105 | Charger le modèle départemental d'IP (version) | CHEF · idem | N-0 | versionné | — |
| SOC-106 | Composer la cellule de veille / GPDS | CHEF · `suivi.veille.gerer` | N-1 | présentée au CA (rappel) | `suivi.veille_composee` |
| SOC-107 | Composer la commission du fonds social | CHEF · `suivi.fonds_social.gerer` | N-1 | — | — |
| SOC-108 | Régler la liste des « adultes de confiance » proposés aux élèves | CHEF · `suivi.parametrer` | N-0 | personnes volontaires | — |
| SOC-109 | Afficher l'écran « ce que vous voyez et pourquoi pas plus » | tous | N-0 | — | — |
| SOC-110 | Voir le journal de configuration | CHEF, DPO · `suivi.niveau.lire` | N-0 | — | — |

### 14.2 Préoccupations
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SOC-120 | Signaler une préoccupation (faits, propos entre guillemets, contexte) | AGT · `suivi.preoccupation.signaler` | N-0 | lint « faits, pas d'interprétation » ; destinataire | `suivi.preoccupation_recue` (identifiants) |
| SOC-121 | Signaler depuis un fil (16), une main courante (14), une situation (13), un passage (34), l'appel (12) | AGT · idem | N-0 | pli ; rien gardé à l'origine | idem |
| SOC-122 | Afficher « danger immédiat » : 17, 15, 119, astreinte | SYS | N-0 | avant tout formulaire | — |
| SOC-123 | Recevoir, accuser réception, prendre en charge | CP, PSY, ASS, INF · `suivi.preoccupation.traiter` | N-0 | délai S-2726 | `suivi.preoccupation_prise` |
| SOC-124 | Orienter : IP, signalement, suivi psy, suivi social, cellule de veille, sans suite (motif codé) | CP · idem | N-0 | décision humaine | `suivi.preoccupation_orientee` |
| SOC-125 | Regrouper deux préoccupations sur le même enfant | CP · idem | N-0 | — | — |
| SOC-126 | Corriger l'élève concerné (erreur) | CP · idem | N-0 | ajout seul | — |
| SOC-127 | Voir le statut de ma préoccupation (« prise en compte le … ») | émetteur · idem | N-0 | sans suite détaillée | — |
| SOC-128 | Voir la file des préoccupations ouvertes | CP · idem | N-0 | alerte si non prise après S-2726 | — |

### 14.3 Information préoccupante et signalement
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SOC-130 | Ouvrir une IP (depuis une préoccupation ou directement) | CP, PSY, ASS, INF, CPE · `suivi.ip.rediger` | N-0 | — | `suivi.ip_ouverte` |
| SOC-131 | Rédiger (modèle, champs pré-remplis de l'identité et des titulaires) | rédacteur · idem | N-0 | adresse protégée jamais reprise | — |
| SOC-132 | Inviter un co-rédacteur | rédacteur · idem | N-0 | membre du personnel désigné | — |
| SOC-133 | Joindre des pièces (photos de messages, écrits) | rédacteur · idem | N-0 | coffre C4 | — |
| SOC-134 | Valider ★ | CHEF ou délégué · `suivi.ip.valider` | N-0 | passkey | `suivi.ip_validee` |
| SOC-135 | Consigner l'information des titulaires (faite / non faite pour intérêt contraire, motif codé) | CHEF · idem | N-0 | humain ; aucune notification | `suivi.ip_titulaires` |
| SOC-136 | Transmettre à la CRIP (canal du département) | CHEF, CP · `suivi.ip.transmettre` | N-0 | PDF à empreinte ; trace du moyen | `suivi.ip_transmise` |
| SOC-137 | Déposer l'accusé de réception | CP · idem | N-0 | — | `suivi.ip_accusee` |
| SOC-138 | Enregistrer un retour de la CRIP | CP · idem | N-0 | pièce C4 | `suivi.ip_retour` |
| SOC-139 | Relancer à l'échéance | CP · idem | N-0 | S-2723 | — |
| SOC-140 | Rédiger un signalement au procureur | CHEF, CP · `suivi.signalement.rediger` | N-0 | rappel des textes | `suivi.signalement_ouvert` |
| SOC-141 | Transmettre le signalement (et copie CRIP) | CHEF · `suivi.signalement.transmettre` ★ | N-0 | — | `suivi.signalement_transmis` |
| SOC-142 | Compléter une IP transmise (fait nouveau) | rédacteur, CP · `suivi.ip.rediger` | N-0 | nouvelle version liée | `suivi.ip_complement` |
| SOC-143 | Voir l'historique des IP d'un élève | CP · `suivi.ip.lire` | N-0 | journal | — |
| SOC-144 | Clore le dossier de protection (motif codé) | CP · idem | N-0 | durée §18 | `suivi.ip_close` |
| SOC-145 | Imprimer l'IP (filigrane) | CP · idem | N-0 | — | — |
| SOC-146 | Déclarer un empêchement du chef (auteur présumé, absence) | CP · `suivi.cellule_protection.gerer` | N-0 | CSOC-02 | `suivi.cellule_empechement` |

### 14.4 Demandes et rendez-vous
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SOC-150 | Demander à parler à quelqu'un (élève) | ELV, ELM · lien | N-0 | sans motif ; écran « ce qui reste confidentiel » ; 3018 / 119 | `suivi.demande_recue` (identifiants, sans destinataire visible hors cercle) |
| SOC-151 | Demander un rendez-vous (famille) | PAR · lien | N-1 | chaque titulaire pour soi | idem |
| SOC-152 | Demander pour un camarade (sans nom obligatoire) | ELV · lien | N-0 | CSOC-36 | idem |
| SOC-153 | Recevoir et répondre à une demande | PSY, ASS, INF, CPE, adulte de confiance · `suivi.demandes.traiter` | N-0 | délai S-2724 | `suivi.demande_traitee` |
| SOC-154 | Proposer un créneau | PSY, ASS · `suivi.rdv.gerer` | N-1 | 19 : occupation | `suivi.rdv_propose` |
| SOC-155 | Émettre le billet de sortie neutre pour la séance | PSY, ASS · idem | N-1 | R-S50 ; professeur : « sortie autorisée » | `appel.sortie_autorisee` |
| SOC-156 | Confirmer, annuler, noter l'absence au rendez-vous | PSY, ASS · idem | N-1 | aucun avis à la famille | `suivi.rdv_etat` |
| SOC-157 | Convoquer une famille | PSY, ASS · idem | N-1 | titulaires seulement ; enveloppe neutre | `suivi.convocation` |
| SOC-158 | Voir mes rendez-vous (élève) | ELV · lien | N-1 | invisible des parents | — |
| SOC-159 | Gérer les créneaux de permanence | PSY, ASS · idem | N-1 | multi-établissements | — |
| SOC-160 | Afficher une réponse humaine préécrite hors horaires | SYS | N-0 | CSOC-28 | — |

### 14.5 Psychologue
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SOC-170 | Enregistrer un entretien (date, durée, nature neutre) | PSY · `suivi.psy.saisir` | N-1 | aucune note en N-1 | `suivi.entretien` (sans nature) |
| SOC-171 | Ouvrir / clore un suivi | PSY · idem | N-1 | — | `suivi.suivi_etat` |
| SOC-172 | Enregistrer le consentement des titulaires à un bilan | PSY · idem | N-1 | pièce C3 | — |
| SOC-173 | Noter une séance de conseil en orientation (37) | PSY · idem | N-1 | sans contenu clinique | — |
| SOC-174 | Envoyer une communication à un titulaire (dans l'ENT) | PSY · idem | N-1 | remplace `signale_famille` | `suivi.communication` |
| SOC-175 | Enregistrer l'opposition de l'élève à une communication | PSY · idem | N-1 | — | — |
| SOC-176 | Participer à une ESS ou à une équipe éducative (35) | PSY | N-1 | rien du cabinet au 35 | — |
| SOC-177 | Rédiger une note clinique, un bilan, un écrit | PSY · `suivi.cabinet_psy.saisir` | **N-2** | cabinet ; ajout seul | (cabinet) |
| SOC-178 | Joindre une pièce au cabinet | PSY · idem | N-2 | chiffrée par objet | — |
| SOC-179 | Lire son cabinet | PSY · `suivi.cabinet_psy.lire` | N-2 | journal | — |
| SOC-180 | Voir mon activité (agrégats) | PSY · idem | N-1 | — | — |
| SOC-181 | Produire le rapport annuel d'activité (anonyme) | PSY · idem | N-1 | — | — |
| SOC-182 | Lire les notes héritées (041, 051) | PSY · `suivi.psy.saisir` | N-1 | lecture seule, journal | — |
| SOC-183 | Décider du sort des notes héritées (transfert au cabinet ou destruction) | PSY | N-1 | PV sans contenu | `suivi.heritage_traite` |

### 14.6 Service social et fonds social
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SOC-190 | Enregistrer un entretien social (date, durée, nature : élève, famille, partenaire) | ASS · `suivi.social.saisir` | N-1 | aucune note en N-1 | `suivi.entretien` |
| SOC-191 | Ouvrir / clore un accompagnement (démarches codées) | ASS · idem | N-1 | — | — |
| SOC-192 | Rédiger une note sociale, un écrit | ASS · `suivi.cabinet_social.saisir` | N-2 | cabinet | — |
| SOC-193 | Déposer une demande d'aide (famille, en ligne) | PAR, ELM · lien | N-1 | pièces au coffre | `suivi.aide_demandee` |
| SOC-194 | Saisir une demande pour une famille | ASS · `suivi.fonds_social.instruire` | N-1 | — | idem |
| SOC-195 | Instruire (ressources par tranches, composition, avis) | ASS · idem | N-1 | — | — |
| SOC-196 | Anonymiser pour la commission (numéro) | SYS | N-1 | aucune identité | — |
| SOC-197 | Préparer l'ordre du jour de la commission | ASS, CHEF · `suivi.fonds_social.gerer` | N-1 | — | — |
| SOC-198 | Décider (montant, nature : cantine, internat, voyage, fournitures, transport) | COM · `suivi.fonds_social.decider` | N-1 | procès-verbal | `suivi.aide_decidee` |
| SOC-199 | Décision d'urgence par le chef (entre deux commissions) | CHEF · idem ★ | N-1 | ratifiée ensuite | idem |
| SOC-200 | Appliquer la remise sur la facture (24) | GES · `suivi.fonds_social.appliquer` | N-1 | libellé neutre | `facture.remise` |
| SOC-201 | Informer la famille de la décision | SYS | N-1 | enveloppe neutre | — |
| SOC-202 | Lier une aide à un voyage (42) sans motif visible de l'organisateur | GES | N-1 | « participation réduite » | — |
| SOC-203 | Voir le budget du fonds (crédits, engagés, versés) | CHEF, GES · `suivi.fonds_social.gerer` | N-1 | agrégats | — |
| SOC-204 | Contribuer au dossier de suivi de l'absentéisme (12) | ASS · `suivi.social.saisir` | N-1 | sans contenu social au 12 | — |
| SOC-205 | Orienter vers des partenaires (CMP, PMI, maison des adolescents) | ASS, PSY | N-1 | liste de partenaires de l'établissement, sans données élève | — |

### 14.7 Cellule de veille et GPDS
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SOC-210 | Inscrire un élève (motif codé non clinique) | CV · `suivi.veille.inscrire` | N-1 | liste noire « vigilance psychique » | `suivi.veille_inscription` |
| SOC-211 | Désigner le référent de situation | CV · idem | N-1 | — | — |
| SOC-212 | Ajouter une action (code, responsable, échéance) | CV · idem | N-1 | — | `suivi.veille_action` |
| SOC-213 | Voir les indicateurs (assiduité, résultats déjà visibles du membre) | CV · `suivi.veille.lire` | N-1 | aucun nouvel accès par ce biais | — |
| SOC-214 | Préparer et tenir une réunion (ordre du jour, relevé codé) | CV · idem | N-1 | aucun compte rendu clinique | `suivi.veille_reunion` |
| SOC-215 | Informer et associer les titulaires | référent · idem | N-1 | chaque titulaire | `suivi.veille_famille` |
| SOC-216 | Orienter vers le psychologue ou l'ASS (demande) | CV · idem | N-1 | pas d'accès au cabinet | `suivi.demande_recue` |
| SOC-217 | Proposer un PPRE, un tutorat (35) | CV · idem | N-1 | port 35 | — |
| SOC-218 | Lier à la MLDS / plateforme de suivi (PSAD) | CV · idem | N-1 | transmission avec accord | — |
| SOC-219 | Clore (motif codé) | CV · idem | N-1 | — | `suivi.veille_close` |
| SOC-220 | Voir le tableau des situations (échéances dépassées) | CV · idem | N-1 | — | — |
| SOC-221 | Pastille « suivi en cours » pour le CPE de l'élève | SYS | N-1 | R-SOC13 | — |

### 14.8 Conservation, droits, sécurité, pilotage
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SOC-240 | Lire le journal des accès (mon cercle) | titulaire du cercle, DPO · `suivi.journal.lire` | N-0 | — | — |
| SOC-241 | Traiter une demande d'accès RGPD | titulaire, DPO · `suivi.acces_droits.traiter` | N-0 | droits d'autrui | `suivi.acces_traite` |
| SOC-242 | Destruction à l'échéance (revue, PV sans contenu) | titulaire, CP · `suivi.conservation.gerer` | N-0 | journal des destructions | `suivi.donnees_detruites` |
| SOC-243 | Rejouer les destructions après restauration | SYS | N-0 | avant réouverture | `suivi.destructions_rejouees` |
| SOC-244 | Répondre à une réquisition judiciaire | CHEF, DPO · `suivi.requisition` ★ | N-0 | CSOC-27 | `suivi.requisition` |
| SOC-245 | Calculer le périmètre d'une fuite | DPO · `suivi.journal.lire` | N-0 | — | `suivi.fuite_evaluee` |
| SOC-246 | Alerte de lecture massive | SYS | N-0 | S-2727 | `suivi.lecture_massive` |
| SOC-247 | Statistiques anonymes (47) | CHEF · `suivi.statistiques.lire` | N-0 | < 5 masqué | — |
| SOC-248 | Produire le dossier d'avis du DPO | CHEF, DPO · `suivi.niveau.lire` | N-0 | — | — |
| SOC-249 | Générer la notice aux familles (par niveau) | CHEF, DPO · idem | N-0 | — | — |
| SOC-250 | Suspendre toutes les casquettes 36 d'un agent (auteur présumé, départ) | CHEF ou délégué · `suivi.cellule_protection.gerer` | N-0 | CSOC-03 | `suivi.casquettes_suspendues` |
| SOC-251 | Ouvrir l'accès des parents d'un élève majeur (convocations) | ELM · lien | N-1 | révocable | — |
| SOC-252 | Gel en cas de décès | SYS | N-0 | — | — |
| SOC-253 | Export particulier du 36 (successeur, nouvel hébergeur) ★ | titulaire ou DPO · `donnees.export.suivi` ; DPO et CHEF approuvent sur vue sans contenu | N-0 | chiffré pour un destinataire nommé | `suivi.export` |

### 14.9 Liaisons avec les autres socles
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SOC-260 | Recevoir un pli du 34 (infirmière) | CP · `suivi.preoccupation.traiter` | N-0 | 34 §10.12 | — |
| SOC-261 | Recevoir une transmission du 13 (DIS-256, DIS-293) | CP · idem | N-0 | la situation reste au 13 | — |
| SOC-262 | Recevoir une proposition du 14 (ACS-228, ACS-241) | CP · idem | N-0 | — | — |
| SOC-263 | Recevoir un fil gelé du 16 (MSG-229) | CP · idem | N-0 | référence au fil, pas de copie | — |
| SOC-264 | Recevoir une alerte d'absentéisme (12) dans la cellule de veille | CV | N-1 | indicateur | — |
| SOC-265 | Publier l'occupation des professionnels (19) | SYS | N-1 | « occupé » | — |
| SOC-266 | Appliquer une aide (24, 42, 40, 41) | GES | N-1 | libellé neutre | — |
| SOC-267 | Vérifier l'éligibilité aux bourses avant une aide (26) | ASS | N-1 | lecture du statut seulement | — |
| SOC-268 | Exclure les ports du 36 des contextes IA (garde) | SYS | N-0 | R-SOC20 | — |
| SOC-269 | Mettre en place une mesure de protection à l'accueil (interdiction de remise, 4) à la demande de la cellule | CP → 4 | N-0 | sur décision judiciaire ou mesure conservatoire du chef [H] | `interdiction.creee` |

### 14.10 Élève, famille
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SOC-280 | Voir les professionnels de l'établissement et comment les joindre | ELV, PAR | N-0 | — | — |
| SOC-281 | Voir les numéros 3018, 119, 15, 17 partout où l'élève écrit | ELV | N-0 | — | — |
| SOC-282 | Voir mes demandes et leur état | ELV, PAR · lien | N-0 | — | — |
| SOC-283 | Lire une communication du professionnel | PAR, ELM · lien | N-1 | accusé | — |
| SOC-284 | Voir l'état de ma demande d'aide | PAR · lien | N-1 | — | — |
| SOC-285 | Retirer une demande d'aide | PAR · lien | N-1 | destruction programmée | — |
| SOC-286 | Voir « ce que l'établissement sait du suivi de mon enfant » (catégories, sans contenu) | PAR, ELM · lien | N-1 | exclut IP et opposition de l'élève | — |

**Compte (conception)** : 121 actions numérotées de SOC-100 à SOC-286 ; **+ 32** de la passe personas (§30) = **153**.

---

## 15. Règles transverses (R-SOC1 et suivantes) [R]
| # | Règle | Lot |
|---|---|---|
| R-SOC1 | **Trois secrets, aucune vue transversale** ; casquette active unique par écran | SO1 |
| R-SOC2 | **Aucun rôle par défaut n'ouvre le 36** : ni direction, ni administrateur, ni `securite.politique`, ni bris de glace ; le chef n'accède qu'en membre de la cellule protection ou de la cellule de veille | SO0 |
| R-SOC3 | **Garde de niveau dans le service** (N-0, N-1, N-2) ; HTTP en défense en profondeur | SO0, SO1 |
| R-SOC4 | **Coordination codée, sans texte clinique** hors cabinet ; liste noire de mots en lint doux | SO1, SO7 |
| R-SOC5 | **Pas de contenu clinique hors du cabinet HDS** ; gel des notes héritées | SO0, SO8 |
| R-SOC6 | **Aucune notification automatique** d'une IP, d'un signalement, d'une préoccupation, d'une demande ou d'un rendez-vous de l'élève, à la famille, à l'auteur présumé ou à un camarade | SO0 |
| R-SOC7 | **Sortie neutre** : entretien = « sortie autorisée » à l'appel ; aucune absence motivée ; rien dans l'agenda famille | SO4 |
| R-SOC8 | **Danger immédiat d'abord** : numéros affichés avant tout formulaire ; l'outil ne retarde jamais un appel | SO2 |
| R-SOC9 | **Information des titulaires avant une IP : décision humaine motivée**, tracée | SO2 |
| R-SOC10 | **Pli sans copie** : le socle d'origine ne garde pas le texte de la préoccupation | SO3 |
| R-SOC11 | **Retour à l'émetteur limité** à « pris en compte » | SO3 |
| R-SOC12 | **Rédacteur** : son IP et son statut, pas les suites | SO2 |
| R-SOC13 | **Pastille de suivi pour le CPE** : seulement si l'élève l'accepte ou s'il est en cellule de veille ; jamais pour une IP | SO7 |
| R-SOC14 | **Autorité parentale, interdictions, majorité** ; chaque titulaire pour soi ; adresse protégée jamais reprise | SO0 |
| R-SOC15 | **Communication explicite** dans l'ENT, enveloppe neutre ; jamais de pièce jointe par courriel | SO0 |
| R-SOC16 | **Événements muets** : identifiants et états ; ni nature, ni durée, ni destinataire ; aucun événement pour une IP hors de la cellule | SO0 |
| R-SOC17 | **Journal des accès transactionnel et fail-closed**, sans contenu ; lisible du titulaire du cercle et du DPO ; jamais de `journal.lire` générique | SO0, SO1 |
| R-SOC18 | **Ajout seul** pour les IP, signalements, préoccupations, comptes rendus de cabinet ; corriger = ajouter | SO2, SO8 |
| R-SOC19 | **Version transmise = version figée** (empreinte) | SO2 |
| R-SOC20 | **IA exclue du 36** à tous niveaux ; garde testée dans tous les contextes | SO0 |
| R-SOC21 | **Fonds social anonymisé** pour la commission ; identité au seul ASS et, à l'application, au gestionnaire | SO6 |
| R-SOC22 | **Remise neutre** sur facture (« aide de l'établissement ») | SO6 |
| R-SOC23 | **Conservation bornée** (§18) ; destructions journalisées, rejouées après restauration | SO11 |
| R-SOC24 | **Export complet sans 36** ; export particulier chiffré pour un destinataire nommé | SO11 |
| R-SOC25 | **Cellule protection jamais vide** ; empêchement du chef géré (CSOC-02) | SO1 |
| R-SOC26 | **Agrégats** : < 5 masqué ; aucun chiffre d'IP par classe ou niveau hors cellule | SO11 |
| R-SOC27 | **Clés distinctes** : « protection », « psy », « social » par établissement, identifiant de clé (E11), séquestre séparé (D-042) | SO1 |
| R-SOC28 | **Public** : N-2 désactivé par défaut dans un EPLE (S-2720) | SO1 |
| R-SOC29 | **Seed et démo sans situation réelle** | SO0 |
| R-SOC30 | **Un seul service suivi** : historique, Leptos, mobile, labo passent par les mêmes commandes | SO1 |
| R-SOC31 | **Tests sur données fictives seulement** | toujours |
| R-SOC32 | **Psychologue : périmètre réduit** (pas de `dossier.lire@etablissement` ni `journal.lire@etablissement` ; identité, classe, EDT des élèves suivis ; **ajout personas** : résultats des seuls élèves suivis, sur demande tracée, Q-SOC23) | SO0 |

---

## 16. Droits (D-017) [R]
| Droit | Contenu | Portées | Modèles |
|---|---|---|---|
| `suivi.niveau.lire` / `.gerer` ★ / `.attester` | niveaux | établissement | Direction / Direction / DPO |
| `suivi.parametrer` | CRIP, parquet, modèles, adultes de confiance | établissement | Direction |
| `suivi.cellule_protection.gerer` | cellule, empêchement, suspension | établissement | Direction (chef) ; délégué en cas d'empêchement |
| `suivi.cercles.gerer` | cercles psy, social | établissement | Direction (ne s'y inscrit pas) |
| `suivi.preoccupation.signaler` | signaler | établissement | **tout personnel** |
| `suivi.preoccupation.traiter` | recevoir, orienter | établissement | cellule protection ; PSY, ASS, INF pour les leurs |
| `suivi.ip.rediger` / `.valider` / `.transmettre` / `.lire` | IP | élève / établissement | rédacteurs désignés (PSY, ASS, INF, CPE, cellule) / chef ou délégué / cellule / cellule |
| `suivi.signalement.rediger` / `.transmettre` ★ | procureur | établissement | cellule / chef ou délégué |
| `suivi.demandes.traiter` | demandes | moi | PSY, ASS, INF, CPE, adultes de confiance |
| `suivi.rdv.gerer` | rendez-vous | moi | PSY, ASS |
| `suivi.psy.saisir` | coordination psy | établissement | **cercle psy seulement** |
| `suivi.cabinet_psy.lire` / `.saisir` | cabinet psy | établissement | cercle psy |
| `suivi.social.saisir`, `suivi.cabinet_social.*` | social | établissement | cercle social |
| `suivi.fonds_social.instruire` / `.gerer` / `.decider` / `.appliquer` | fonds social | établissement | ASS / Direction / commission / Gestionnaire |
| `suivi.veille.inscrire` / `.lire` / `.gerer` | cellule de veille | établissement | membres / membres / Direction |
| `suivi.journal.lire` | journal | cercle | titulaire du cercle, DPO |
| `suivi.acces_droits.traiter`, `suivi.conservation.gerer` | droits, destruction | cercle | titulaire, DPO |
| `suivi.requisition` ★ | réquisition | établissement | Direction + DPO |
| `suivi.statistiques.lire` | agrégats | établissement | Direction |
| `donnees.export.suivi` ★ | export particulier | cercle | titulaire, DPO |

**Alias de transition** : `psy.lire` → `suivi.psy.saisir` (lecture) et `psy.saisir` → `suivi.psy.saisir`, **pour le seul rôle Psy** ; **retirés d'Admin et de Direction** (DSOC-01).

**Pièges à tester** : (1) direction hors cellule, administrateur, secrétariat, support : aucune route du 36 ; (2) psychologue : pas de lecture de l'IP qu'il n'a pas rédigée, pas du social ; (3) ASS : pas du psy ; (4) parent sans autorité, interdit, auteur présumé : rien ; parents d'un majeur : rien ; (5) événements : muets ; (6) journal générique : aucune entrée du 36 ; (7) IA : refus ; (8) appel : « sortie autorisée » seulement ; (9) agenda famille : rien ; (10) commission : aucune identité ; (11) export complet : sans 36 ; (12) rédacteur : son IP et son statut seulement.

---

## 17. Chiffrement et journal [R]
1. **Clés** par établissement : « protection » (IP, signalements, préoccupations), « suivi » (coordination N-1 : demandes, motifs de rendez-vous, actions de veille, dossiers d'aide), et, au cabinet, « psy » et « social » ; enveloppe KMS, identifiant `enc:v2:<id_cle>:`, séquestre séparé de celui d'`ENT_SECRETS_KEY` et de la clé santé (E11, D-042).
2. **Pièces** : coffre C4 (protection) ou C3 (aide sociale), chiffrement applicatif par objet ; URL pré-signée de 60 s après contrôle et trace.
3. **Journal** `ent_sui_acces` (et `cab_*_acces`) : qui, casquette, quand, catégorie, identifiant opaque, contexte ; **même transaction** ; ajout seul ; sans contenu ; lecture : titulaire du cercle, DPO.
4. **Alertes** : lecture massive (S-2727), lecture d'une IP par un rédacteur au-delà de la sienne (refus + trace), accès hors période d'un remplaçant.

---

## 18. Conservation [R, durées soumises au DPO]
| Donnée | Durée proposée | Raison |
|---|---|---|
| Copie d'IP et de signalement, retours, pièces | **jusqu'à la majorité de l'élève + 1 an**, puis destruction, **sauf** procédure judiciaire en cours connue | preuve de la transmission ; la CRIP détient l'original [R, à valider par le DPO et un juriste] |
| Préoccupation orientée « sans suite » | **1 an** | minimisation |
| Préoccupation transformée (IP, suivi, veille) | suit l'objet créé | — |
| Demandes et rendez-vous (coordination) | année scolaire + 1 an | — |
| Entretiens (coordination, sans contenu) | fin du suivi + 1 an | — |
| Cabinet psy / social (N-2) | **fin de scolarité dans l'établissement + 1 an** (aligné SAGESSE pour la santé, 34 §18) [R] | — |
| Situation de veille / GPDS | clôture + 1 an | — |
| Fonds social : dossier nominatif | décision + **1 an** ; pièces justificatives : décision + 6 mois [R] ; procès-verbaux et montants : durée comptable (10 ans, 48) sous forme anonyme | comptabilité publique / OGEC [NV] |
| Journal des accès | **3 ans** (AIPD : données de protection) [R] | preuve |
| Journal des destructions | hors rétention, rejoué | R-S44 du 34 |
| Signalements Next.js repris | destruction de l'original à la décision de Florian | D-035 |

---

## 19. Événements (outbox) [R]
**Émis** (identifiants opaques, états ; **aucun** nature, motif, destinataire, durée) : `suivi.niveau_*`, `suivi.cellule_modifiee`, `suivi.cercle_modifie`, `suivi.preoccupation_recue|prise|orientee`, `suivi.demande_recue|traitee`, `suivi.rdv_propose|rdv_etat`, `suivi.entretien`, `suivi.suivi_etat`, `suivi.communication`, `suivi.aide_demandee|aide_decidee`, `suivi.veille_*`, `suivi.donnees_detruites|destructions_rejouees`, `suivi.lecture_massive`, `suivi.casquettes_suspendues`.
**Événements de protection** (`suivi.ip_*`, `suivi.signalement_*`) : **file réservée à la cellule protection**, jamais publiés sur le bus commun ; aucun consommateur hors du 36.
**Consommateurs** : 12 (`appel.sortie_autorisee`), 19 (occupation), 24 (`facture.remise`), 8 (communications, convocations : natures restreintes), 47 (agrégats).
**Consommés** : `eleve.cree|majeur|decede|fusionne|sortie` (2) ; `lien.*`, `interdiction.*`, `personne.adresse_protegee` (4) ; `agent.fonction_*`, `agent.fin` (6) ; `sante.pli_suivi` (34) ; `discipline.situation_transmise` (13) ; `accueil.proposition_ip` (14) ; `messagerie.fil_gele` (16) ; `assiduite.seuil` (12).
**Historiques** `psy.entretien|rdv|dossier|courrier` : renommés ; charges utiles **réduites** dès SO0 (DSOC-04).

---

## 20. Notifications (socle 8) [R]
| Nature | Urgence · sensibilité | Destinataires | Contenu externe |
|---|---|---|---|
| `suivi.preoccupation_nouvelle` | U2 · C4 | cellule protection, ou le professionnel choisi | « Une demande vous attend » (in-app, push neutre) |
| `suivi.ip_a_valider` | U3 · C4 | chef ou délégué | « Une validation urgente vous attend » |
| `suivi.ip_relance` | U1 · C4 | cellule | in-app |
| `suivi.demande_recue` | U1 · C3 | professionnel | « Une demande vous attend » |
| `suivi.rdv_propose` | U1 · C3 | **élève** (pas les parents) | « Un rendez-vous t'est proposé — ouvre l'ENT » |
| `suivi.convocation` | U1 · C3 | titulaires convoqués | « Message de l'établissement concernant {prénom} » |
| `suivi.communication` | U1 · C3 | titulaire désigné | idem |
| `suivi.aide_decidee` | U0 · C3 | demandeur | « Une réponse à votre demande est disponible » |
| `suivi.veille_echeance` | U0 · C2 | référent de situation | in-app |
| `suivi.lecture_massive` | U2 · C2 | titulaire du cercle, DPO | in-app, courriel neutre |

**Jamais** : aucune notification aux titulaires d'une IP, d'une préoccupation, d'une demande ou d'un rendez-vous de l'élève ; aucun SMS pour le 36 ; aucun aperçu de contenu sur écran verrouillé.

---

## 21. Écrans et mobile [R]
| Écran | Où | Contenu |
|---|---|---|
| **Signaler une préoccupation** | partout (bouton) | danger immédiat → numéros ; formulaire court ; destinataire |
| **Cellule protection** | `/web/suivi/protection` | file des préoccupations, IP en cours (rédaction, validation, transmission, retours), échéances |
| **IP** | `/web/suivi/protection/ip/{id}` | modèle, versions, co-rédacteurs, information des titulaires, transmission, retours, pièces |
| **Psychologue — aujourd'hui** | `/web/suivi/psy` | demandes, rendez-vous, suivis, (N-2) cabinet |
| **Service social** | `/web/suivi/social` | demandes, entretiens, dossiers d'aide, commission |
| **Commission du fonds social** | `/web/suivi/fonds-social/commission/{id}` | dossiers anonymes, budget, décisions, PV |
| **Cellule de veille / GPDS** | `/web/suivi/veille` | situations, actions, échéances, réunions |
| **Élève — Parler à quelqu'un** | espace élève | adultes, « ce qui reste confidentiel », 3018 / 119, mes demandes, mes rendez-vous |
| **Famille — Aide et accompagnement** | espace famille | demander un rendez-vous, une aide ; communications ; état |

**Mobile (D-003)** : tout personnel — « signaler une préoccupation » (hors ligne : brouillon local **chiffré**, effacé à l'envoi) ; psychologue et ASS — agenda, demandes, rendez-vous ; cellule — validation d'IP par passkey ; élève — « parler à quelqu'un », rendez-vous. Écran sécurisé, aucun aperçu sur écran verrouillé, aucune copie locale d'une IP. **Ajout personas** : **mode discret** de l'élève (icône neutre de l'espace « Parler à quelqu'un », aucun aperçu, `PERSONAS_36.md` P15).

---

## 22. Modèle de données cible [R]

Principe : base principale pour N-0 et N-1 (`ent_sui_*`, `ent_ip_*`, `ent_fs_*`), cabinet séparé pour N-2 (`cab_psy_*`, `cab_soc_*`). E1–E14 : `etablissement_id NOT NULL`, FK composites, UUID applicatifs, aucun JSON, aucun tableau, aucun `BYTEA`, pièces en `ent_objet`, `cree_le` / `modifie_le`, ajout seul (déclencheurs) pour IP, préoccupations, versions, journal ; RLS ENABLE + FORCE ; aucun `ON DELETE CASCADE`.

```
-- Références globales (E12 « globale_reference »)
ref_sui_motif_veille            (code, libelle)          -- non clinique ; liste noire testée
ref_sui_action_veille           (code, libelle)
ref_sui_orientation_preoccupation (code)                -- ip, signalement, psy, social, veille, sans_suite
ref_sui_motif_non_information   (code)                   -- intérêt contraire de l'enfant : danger, risque de pression, auteur présumé…
ref_sui_demarche_sociale        (code, libelle)
ref_fs_nature_aide              (code, libelle)          -- cantine, internat, voyage, fournitures, transport, autre

-- Base principale : niveaux, cercles, paramètres
ent_sui_niveau                  (etablissement_id, niveau, depuis, attestation_dpo_objet_id, hebergement_hds_ref NULL)
ent_sui_niveau_journal          (etablissement_id, id, niveau, action, par, le, motif)        -- ajout seul
ent_sui_cercle_membre           (etablissement_id, id, cercle ('protection','psy','social','veille','commission'), agent_id, role_cercle ('titulaire','remplacant','delegue','membre'), du, au)
ent_sui_parametre               (etablissement_id, crip_libelle, crip_canal ('courriel_securise','formulaire','courrier'), crip_adresse, parquet_libelle, parquet_adresse, delai_prise_heures, delai_reponse_demande_heures, relance_ip_jours)
ent_sui_modele_ip               (etablissement_id, id, version, objet_id, actif_du)
ent_sui_adulte_confiance        (etablissement_id, id, agent_id, du, au)

-- Protection (clé « protection »)
ent_sui_preoccupation           (etablissement_id, id, eleve_id, emetteur_personne_id, origine ('direct','appel','message','main_courante','discipline','infirmerie','inclusion','internat'), origine_ref UUID NULL,
                                 destinataire_cercle, destinataire_agent_id NULL, texte_chiffre, le, prise_le, prise_par, orientation_code NULL, oriente_le)
ent_sui_preoccupation_ajout     (etablissement_id, id, preoccupation_id, par, le, texte_chiffre)     -- ajout seul (corrections, regroupements)
ent_ip_dossier                  (etablissement_id, id, eleve_id, nature ('ip','signalement'), ouvert_le, ouvert_par, etat ('redaction','validee','transmise','close'), clos_le, motif_cloture_code)
ent_ip_dossier_preoccupation    (etablissement_id, dossier_id, preoccupation_id)
ent_ip_redacteur                (etablissement_id, dossier_id, personne_id, depuis)
ent_ip_version                  (etablissement_id, id, dossier_id, numero, modele_id, contenu_objet_id (PDF chiffré), empreinte, cree_par, cree_le)  -- ajout seul
ent_ip_validation               (etablissement_id, id, version_id, valide_par, valide_le, preuve_passkey_ref)
ent_ip_information_titulaires   (etablissement_id, id, dossier_id, decision ('informes','non_informes'), motif_code NULL, par, le)
ent_ip_information_personne     (etablissement_id, information_id, personne_id, informe_le, moyen)
ent_ip_transmission             (etablissement_id, id, version_id, destinataire ('crip','parquet'), canal, transmis_le, transmis_par, accuse_objet_id NULL, accuse_le NULL)
ent_ip_retour                   (etablissement_id, id, dossier_id, recu_le, objet_id, saisi_par)
ent_ip_piece                    (etablissement_id, id, dossier_id, objet_id, ajoutee_par, le)
ent_ip_empechement              (etablissement_id, id, agent_id, motif_code, du, au, declare_par)

-- Coordination (clé « suivi »)
ent_sui_demande                 (etablissement_id, id, eleve_id NULL, demandeur_personne_id, pour_camarade BOOL, destinataire_cercle, destinataire_agent_id NULL, le, traitee_le, traitee_par, issue_code)
ent_sui_rdv                     (etablissement_id, id, eleve_id NULL, cercle, agent_id, debut, fin, motif_chiffre NULL, etat, sortie_seance_id NULL, cree_le, modifie_le)
ent_sui_rdv_participant         (etablissement_id, rdv_id, personne_id, qualite)
ent_sui_entretien               (etablissement_id, id, eleve_id, cercle, agent_id, le, duree_minutes, nature ('individuel','famille','orientation','urgence','partenaire'), cabinet_ref UUID NULL)
ent_sui_suivi                   (etablissement_id, id, eleve_id, cercle, agent_id, ouvert_le, clos_le, eleve_accepte_pastille BOOL)
ent_sui_consentement            (etablissement_id, id, eleve_id, objet ('bilan','suivi'), personne_id, donne_le, retire_le, objet_piece_id NULL)
ent_sui_opposition_eleve        (etablissement_id, id, eleve_id, cercle, le, portee, leve_le)
ent_sui_communication           (etablissement_id, id, eleve_id, cercle, auteur_agent_id, destinataire_personne_id, texte_chiffre, envoye_le, lu_le)
ent_sui_acces_majeur            (etablissement_id, id, eleve_id, personne_id, portee, du, revoque_le)

-- Cellule de veille, GPDS
ent_sui_veille_situation        (etablissement_id, id, eleve_id, motif_code, referent_agent_id, ouverte_le, close_le, motif_cloture_code)
ent_sui_veille_action           (etablissement_id, id, situation_id, action_code, responsable_agent_id, echeance, faite_le)
ent_sui_veille_reunion          (etablissement_id, id, prevue_le, tenue_le)
ent_sui_veille_reunion_point    (etablissement_id, reunion_id, situation_id, decision_code)
ent_sui_veille_famille          (etablissement_id, id, situation_id, personne_id, informe_le, opposition_le NULL)

-- Fonds social
ent_fs_exercice                 (etablissement_id, id, annee, credit_cantine, credit_general)   -- montants NUMERIC
ent_fs_demande                  (etablissement_id, id, numero_anonyme, foyer_id, eleve_id, nature_code, montant_demande, deposee_le, deposee_par, etat)
ent_fs_instruction              (etablissement_id, demande_id, tranche_ressources_code, composition_code, avis_ass ('favorable','defavorable','reserve'), instruit_par, le)
ent_fs_piece                    (etablissement_id, id, demande_id, objet_id, le)
ent_fs_commission               (etablissement_id, id, exercice_id, tenue_le, pv_objet_id)
ent_fs_decision                 (etablissement_id, id, demande_id, commission_id NULL, urgence BOOL, montant_accorde, nature_code, decide_le, ratifiee_le NULL)
ent_fs_application              (etablissement_id, id, decision_id, facture_ref UUID, montant, applique_par, le)

-- Transverse
ent_sui_acces                   (etablissement_id, id, personne_id, casquette, cercle, categorie, objet_id, contexte, le)   -- ajout seul
ent_sui_destruction             (etablissement_id, id, categorie, objet_id, le, pv_objet_id)                                -- hors rétention

-- Cabinet (base séparée HDS, série de migrations propre)
cab_psy_note, cab_psy_bilan, cab_psy_piece, cab_psy_acces
cab_soc_note, cab_soc_piece, cab_soc_acces
cab_heritage_psy                -- reprise des notes 041/051 (SO8)
```

Registre (E12) : toutes les tables `ent_sui_*`, `ent_ip_*`, `ent_fs_*` en catégorie d'export **« particulière »**, exclues de l'export complet (test) ; `ref_*` en `globale_reference` ; **aucune file d'effet de bord propre** (notifications par le 8, déclarées neutralisées) ; la file réservée des événements de protection est **déclarée « à neutraliser à la restauration »** (E9).

---

## 23. Plan de migration depuis l'existant
| Étape | Quoi | Garantie |
|---|---|---|
| MIG-1 (SO0) | Droits : `psy.*` retirés d'Admin et de Direction ; modèle « psy » sans `dossier.lire@etablissement` ni `journal.lire@etablissement` ; parents : contrôle d'autorité, d'interdiction, de majorité ; envoi du courrier par courriel **supprimé** (route répond 410) ; téléchargement famille seulement si une communication explicite existe | aucune donnée modifiée |
| MIG-2 (SO0) | Événements psy à charge réduite ; trace transactionnelle ; libellés de journal neutres et hors `journal.lire` générique ; registre : tables psy « particulière », sans export de portabilité automatique | test sentinelle |
| MIG-3 (SO0, 2720) | `ent_psy_dossier_instantane` (instantané chiffré à chaque écriture, rétroactif sur la ligne courante) ; FK composites ; CHECK gelant les nouvelles notes cliniques (colonne vide imposée) | aucune donnée perdue |
| MIG-4 (SO1–SO5) | `ent_psy_rdv` → `ent_sui_rdv` ; `ent_psy_entretien` → `ent_sui_entretien` (note → héritage) ; `ent_psy_dossier.suivi_actif` → `ent_sui_suivi` ; `signale_famille` → aucune reprise automatique en communication (le psychologue décide) | vues de compatibilité pour `ui.rs` |
| MIG-5 (SO8) | Héritage clinique → cabinet HDS, puis effacement dans la base principale ; ou destruction décidée | PV sans contenu |
| MIG-6 (SO12) | Signalements Next.js : export fourni par Florian (D-035 point 4), lecture par le psychologue, recodage manuel en préoccupations s'il le juge utile, destruction | aucune reprise automatique |
| MIG-7 (SO13) | Retrait de 041, 046, 051 et des routes `/psy/*` avec `ui.rs` (F-001) | 2739, sur décision (E8) |

---

## 24. Lots de code (équipe Sonnet)
| Lot | Contenu | Critères de fin | Dépend de |
|---|---|---|---|
| **SO0** correctifs urgents | DSOC-01 à DSOC-05, DSOC-07 (instantané), DSOC-08 à DSOC-10 ; gel des notes ; garde IA ; seed | matrice §16 testée ; test « direction sans psy » ; test « parent sans autorité » ; test « événements muets » ; test « IA sans 36 » | — |
| **SO1** niveaux, cercles, journal, clés | niveaux, cercles, cellule protection, empêchement, journal, clés | aucune lecture sans trace ; cellule jamais vide | SO0, 6 |
| **SO2** protection | IP, signalement, modèles, validation passkey, information des titulaires, transmission, retours, pièces | **V1** ; CSOC-01 à CSOC-07, CSOC-32 à CSOC-34 | SO1, 7 (coffre C4), 4 |
| **SO3** préoccupations | bouton partout, plis des 12, 13, 14, 16, 34, 35, 41 | rien gardé à l'origine ; CSOC-19 à CSOC-22 | SO2 |
| **SO4** demandes, rendez-vous | « parler à quelqu'un », créneaux, billet neutre, réponse hors horaires | R-SOC7 ; CSOC-28, CSOC-29, CSOC-36 | SO1, 12, 19 |
| **SO5** psychologue (N-1) | entretiens, suivis, consentements, communications, héritage | DSOC-02 corrigé ; communications sans courriel | SO4 |
| **SO6** social et fonds social | entretiens, démarches, demandes, anonymisation, commission, décisions, application (24) | CSOC-14 à CSOC-16 | SO4, 24, 26 |
| **SO7** cellule de veille, GPDS | situations, actions, réunions, familles, pastille CPE | CSOC-17, CSOC-18 | SO1, 12, 21, 35 |
| **SO8** cabinet (N-2) | notes, bilans, pièces, transfert de l'héritage | **après HDS** | SO5, SO6, 34 (cabinet) |
| **SO9** liaisons | 12, 13, 14, 16, 19, 24, 26, 37, 41 | ports testés | SO3 à SO7 |
| **SO10** mobile | préoccupation hors ligne chiffrée, agenda, validation, élève | écran sécurisé | SO2, SO4, 11 |
| **SO11** conservation, droits, restauration | durées, destructions rejouées, export particulier, réquisition, fuite | CSOC-24 à CSOC-27 | SO2 |
| **SO12** reprise Next.js | outil de lecture des signalements exportés, recodage, PV | Florian fournit l'export | SO3 |
| **SO13** RLS stricte, retraits | test strict ; retrait 041/046/051 | 2738, 2739 | tous |

**Parcours minimal (V1, fin janvier 2027)** : SO0, SO1, **SO2**, SO3, SO4 (N-0 complet : protection et demandes sans motif) ; N-1 (SO5–SO7) après l'avis du DPO, visé pour la rentrée 2027 ; N-2 après l'HDS.

---

## 25. Migrations (plage imposée 2720–2739)
| N° | Lot | Contenu |
|---|---|---|
| **2720** | SO0 | `ent_psy_dossier_instantane` ; FK composites sur 041/046/051 ; CHECK de gel des notes ; `ent_sui_acces` |
| **2721** | SO1 | `ref_sui_*`, `ref_fs_nature_aide` ; `ent_sui_niveau`, `_niveau_journal`, `_cercle_membre`, `_parametre`, `_modele_ip`, `_adulte_confiance` |
| **2722** | SO2 | `ent_ip_dossier`, `_redacteur`, `_version`, `_validation` (ajout seul) |
| **2723** | SO2 | `ent_ip_information_titulaires`, `_information_personne`, `_transmission`, `_retour`, `_piece`, `_empechement` |
| **2724** | SO3 | `ent_sui_preoccupation`, `_preoccupation_ajout`, `ent_ip_dossier_preoccupation` |
| **2725** | SO4 | `ent_sui_demande`, `_rdv`, `_rdv_participant` |
| **2726** | SO5 | `ent_sui_entretien`, `_suivi`, `_consentement`, `_opposition_eleve`, `_communication`, `_acces_majeur` |
| **2727** | SO6 | `ent_fs_exercice`, `_demande`, `_instruction`, `_piece`, `_commission`, `_decision`, `_application` |
| **2728** | SO7 | `ent_sui_veille_situation`, `_action`, `_reunion`, `_reunion_point`, `_famille` |
| **2729** | SO11 | `ent_sui_destruction` |
| **2730** | SO1–SO5 | reprise de 041/046/051 (MIG-4), idempotente |
| **2731** | SO12 | `ent_sui_reprise_nextjs` (journal du recodage : identifiant d'origine opaque, décision, date) |
| 2732–2737 | — | réserve (passe personas : §30.6) |
| **2738** | SO13 | RLS stricte (test) |
| **2739** | SO13 | retrait de 041, 046, 051 (**destructif, sur décision**, sauvegarde vérifiée, E8) |

**D-036** : aucune colonne JSON/JSONB, aucun tableau, aucun `BYTEA` ; contrôle `grep -iE "json|bytea|\[\]"` sur 2720–2739. Le cabinet a sa série propre (`cabinet/migrations`), comme le 34.

---

## 26. Tests et recette
Matrice de séparation (§16, douze pièges) ; test sentinelle des événements et de la file réservée ; test « IA sans 36 » ; test « appel : sortie autorisée seulement » ; test « agenda famille vide » ; test « commission sans identité » ; test « export complet sans 36 » ; test de restauration (destructions rejouées) ; test « cellule jamais vide » ; test « chef empêché » ; données fictives seulement. **Recette** avec une PsyEN, une assistante sociale et un chef d'établissement réels (Q-SOC6), sur une IP fictive de bout en bout et une commission du fonds social.

---

## 27. Réglages et questions

### 27.1 Réglages (S-2720 à S-2731)
| # | Réglage | Défaut |
|---|---|---|
| S-2720 | N-2 dans un EPLE | désactivé |
| S-2721 | modèle d'IP | national générique ; départemental chargé |
| S-2722 | canal de transmission à la CRIP | courriel sécurisé du département |
| S-2723 | relance d'une IP sans retour (jours) | 30 |
| S-2724 | délai de réponse à une demande d'élève (heures ouvrées) | 48 |
| S-2725 | transmission directe par tout membre de la cellule en cas d'empêchement | activé |
| S-2726 | délai de prise en charge d'une préoccupation (heures) | 4 (alerte), 24 (escalade au délégué) |
| S-2727 | seuil de lecture massive | 20 dossiers / heure |

### 27.2 Questions (recommandations appliquées par défaut, D-031, sauf mention)
| # | Question | Recommandation |
|---|---|---|
| Q-SOC1 | **Risque de production Next.js « bien-être »** (NXB-01 à NXB-10) : restreindre, corriger ou arrêter ? | **Soumis à Florian** (Next.js intouchable pour le pilote) ; recommandation : désactiver l'agent conversationnel en production au plus tôt, limiter la lecture des signalements au psychologue, corriger le numéro (119) |
| Q-SOC2 | Ne pas reprendre l'agent conversationnel ; le remplacer par « parler à un adulte » | **Soumis à Florian** (changement d'usage) ; recommandation : oui |
| Q-SOC3 | N-1 sans HDS ; champ HDS du suivi psychologique et social | **Soumis au DPO** (avec juriste) |
| Q-SOC4 | IP et signalement disponibles dès N-0, dans la base principale, clé « protection » | **Oui** (appliqué) ; **soumis au DPO** pour confirmation |
| Q-SOC5 | Coordonnées et protocole de la CRIP et du parquet du département pilote | **Soumis à Florian** (données réelles, convention éventuelle) |
| Q-SOC6 | Référents métier (PsyEN ou psychologue du privé, ASS, chef) | **Soumis à Florian** |
| Q-SOC7 | Durées §18, notamment la copie d'IP | **Soumis au DPO** ; défauts appliqués |
| Q-SOC8 | Lancer SO0 maintenant | **Oui** (appliqué) |
| Q-SOC9 | Secret du psychologue : code de déontologie et 226-13 | **Soumis à un juriste** (via Florian) ; défaut : traité comme un secret professionnel |
| Q-SOC10 | Consentements (entretien sans parents, suivi, bilan) | **Appliqué** (§11.4) ; avis juridique au premier conflit |
| Q-SOC11 | Droit d'accès d'un titulaire à une IP qui le concerne | **Soumis au DPO et à un juriste** ; défaut : refus motivé, orientation vers la CRIP |
| Q-SOC12 | Aucune notification automatique (R-SOC6) | **Oui** (appliqué) |
| Q-SOC13 | Cellule de veille et GPDS sans contenu clinique, liste noire de motifs | **Oui** (appliqué) |
| Q-SOC14 | Pastille de suivi pour le CPE avec accord de l'élève | **Oui** (appliqué) |
| Q-SOC15 | Fonds social anonymisé, remise neutre | **Oui** (appliqué) |
| Q-SOC16 | Courrier par courriel supprimé ; communications dans l'ENT | **Oui** (appliqué) |
| Q-SOC17 | IA exclue du 36 | **Oui** (appliqué ; conforme à la consigne de Florian) |
| Q-SOC18 | Trois clés distinctes, séquestre séparé | **Oui** (appliqué) ; inventaire E11 |
| Q-SOC19 | N-2 désactivé par défaut dans un EPLE | **Oui** (appliqué) |
| Q-SOC20 | Plage 2720–2739 | **Réservée** (imposée) |

---

## 28. Faits, hypothèses, contradictions
**Faits [F]** : §2.1–§2.2 (code lu sur `afeb8c4`, lignes citées) ; §2.3 (Next.js, lecture ciblée de `app/lib/bien-etre-*.ts` et `app/api/bien-etre/*`, aucune modification) ; plages vérifiées sur 134 branches.
**Vérifié en extrait [Ve], à relire sur le texte entier** : L226-2-1, L226-3, D226-2-6, L226-2-2 ; L411-3 ; circulaire 2017-055 ; décret 2017-120 ; code de déontologie (art. 7, CNCDP) ; circulaire du 2/02/2024 (harcèlement) ; GPDS ; circulaire 2014-159.
**Non vérifié [NV]** : L226-5 (retour à l'émetteur) ; 434-3 ; circulaire des fonds sociaux ; formulation exacte de L1111-8 sur le « suivi social et médico-social » ; numéro 3114 (prévention du suicide) et 116 111 ; seuil d'absentéisme au second degré ; organisation dans le privé.
**Hypothèses [H]** : usage réel du module bien-être (Q-T0-11) ; consentements (§11.4) ; inscription au GPDS malgré l'opposition ; copie de signalement à la CRIP.
**Contradictions** :
1. `SOCLE2_DOMAINES.md` SAN-07 met le psychologue dans la santé ; la carte et le 34 le placent au **36** : suivi.
2. Le code classe les tables psy en domaine `sante` du registre ; ce document crée le domaine **`suivi`** (reclassement en SO0).
3. `TRANSITION_T0_INVENTAIRE.md` M18 recommande « **à conserver** » les signalements bien-être : la conservation de la **fonction** est assurée (préoccupations, demandes), **pas** celle de l'agent conversationnel ni du stockage JSON (Q-SOC2).
4. `19_AGENDA_RDV.md` (Q-AG9) laisse les rendez-vous psy au 36 et lit l'occupation : confirmé.
5. `13_DISCIPLINE.md` R470 évoque des **questionnaires nominatifs** du socle 36 : ce document ne prévoit **aucun questionnaire nominatif** de repérage (risque de profilage) ; les questionnaires pHARe restent au 13 [R].
6. `IA_COEUR.md` §2 signale déjà le risque Next.js d'IA sur la santé mentale : précisé ici (NXB-01 à NXB-03).

---

## 29. Les cinq décisions prioritaires
1. **SO0 maintenant** : la direction lit le dossier du psychologue, un courrier psychologique part en pièce jointe par courriel, un parent sans autorité lit le suivi. **Appliqué par défaut.**
2. **La protection dès la V1, à N-0** (SO2) : IP et signalement guidés, validés, transmis, sans notification automatique. **Appliqué** ; confirmation du DPO.
3. **Next.js « bien-être »** : agent conversationnel à désactiver, lecture à restreindre. **Soumis à Florian.**
4. **Pas de contenu clinique hors HDS** ; cabinet commun avec le 34, cercles et clés distincts. **Soumis au DPO et à Florian** (coût HDS commun avec Q-S1).
5. **Référents et CRIP du département pilote**. **Soumis à Florian.**

---

## 30. Ajouts de la passe personas (2026-11-11, `PERSONAS_36.md`)

**Statut** : proposition ; ce paragraphe **l'emporte** en cas de contradiction.

### 30.1 Actions ajoutées (SOC-300 à SOC-331)
| ID | Action | Qui · droit | Niv. | Règles | Événement |
|---|---|---|---|---|---|
| SOC-300 | Recueillir une révélation en direct : guide « écouter, ne pas questionner, noter les mots exacts, ne rien promettre » | AGT | N-0 | affiché avec le formulaire | — |
| SOC-301 | Signaler depuis la tablette du surveillant de nuit (internat, 41) | surveillant · `suivi.preoccupation.signaler` | N-0 | astreinte prévenue si danger | — |
| SOC-302 | Joindre une capture de message d'un réseau social présentée par un élève | AGT · idem | N-0 | coffre C4 ; jamais dans la photothèque du téléphone | — |
| SOC-303 | Brouillon chiffré hors ligne effacé à l'envoi ou après 24 h | AGT | N-0 | mobile | — |
| SOC-304 | Escalader une préoccupation non prise (au délégué puis au chef) | SYS | N-0 | S-2726 | `suivi.preoccupation_escaladee` |
| SOC-305 | Prévenir l'émetteur qu'il n'a plus rien à porter (« pris en charge, vous pouvez en parler à … si besoin ») | SYS | N-0 | soutien de l'adulte | — |
| SOC-306 | Proposer à l'adulte émetteur un temps d'échange (psychologue, médecin du travail) | CP | N-0 | facultatif | — |
| SOC-307 | Préparer un entretien avec les titulaires avant une IP (guide) | CHEF | N-0 | — | — |
| SOC-308 | Enregistrer la remise en main propre d'une copie de l'IP aux titulaires (si décidé) | CHEF | N-0 | — | — |
| SOC-309 | Gérer une ordonnance de placement ou une mesure éducative connue (référent ASE, personne à contacter) | CP · `suivi.ip.lire` → 4 | N-0 | le 4 tient les personnes ; le 36 la mention | `suivi.mesure_connue` |
| SOC-310 | Afficher au secrétariat « contact ASE à joindre pour cet élève » sans motif | SYS → 4 | N-0 | rôle de contact seulement | — |
| SOC-311 | Recevoir une demande d'un élève pendant les vacances | SYS | N-0 | réponse humaine préécrite + numéros | — |
| SOC-312 | Proposer un « rendez-vous rapide » sur créneau de permanence (sans demande écrite) | PSY, ASS | N-1 | — | — |
| SOC-313 | Agenda multi-établissements consolidé (casquettes) | PSY, ASS | N-1 | aucun nom d'élève d'un autre établissement | — |
| SOC-314 | Bloquer un créneau sans élève (synthèse, formation) | PSY, ASS | N-1 | — | — |
| SOC-315 | Imprimer un billet de sortie neutre (sans compte du professeur remplaçant) | PSY, ASS, CPE | N-1 | — | — |
| SOC-316 | Clore les demandes restées sans réponse de l'élève (3 relances) | PSY, ASS | N-1 | — | — |
| SOC-317 | Saisir une aide d'urgence pour un repas du jour (cantine) | ASS, CHEF · `suivi.fonds_social.decider` | N-1 | 40 : repas servi sans facturation, libellé neutre | `suivi.aide_decidee` |
| SOC-318 | Déclarer un lien avec une famille et se retirer d'un dossier de commission | COM | N-1 | CSOC-15 | — |
| SOC-319 | Exporter le procès-verbal anonyme de la commission (CA, comptabilité) | CHEF, GES | N-1 | sans identité | — |
| SOC-320 | Suivre la consommation du fonds par nature | CHEF, GES | N-1 | agrégats | — |
| SOC-321 | Produire le bilan annuel du GPDS pour le CA (anonyme) | CHEF | N-1 | < 5 masqué | — |
| SOC-322 | Revue trimestrielle des situations de veille ouvertes depuis plus de 6 mois | CV | N-1 | rappel | — |
| SOC-323 | Revue mensuelle des membres des cercles (départs, remplacements) | CHEF | N-0 | — | — |
| SOC-324 | Produire le dossier d'avis du DPO du 36 | CHEF, DPO | N-0 | — | — |
| SOC-325 | Tracer la consultation par le DPO lui-même | SYS | N-0 | visible du titulaire du cercle | — |
| SOC-326 | Révoquer immédiatement toutes les sessions d'un compte de la cellule compromis | SYS, CHEF | N-0 | socle 1 | `suivi.sessions_revoquees` |
| SOC-327 | Afficher à l'élève l'historique de ses demandes (sans contenu de l'adulte) | ELV | N-0 | — | — |
| SOC-328 | Permettre à l'élève d'effacer sa demande non encore lue | ELV | N-0 | trace « retirée » sans contenu | — |
| SOC-329 | Lire les signalements Next.js exportés et décider pour chacun | PSY | — | SO12 | — |
| SOC-330 | Consigner la destruction des signalements Next.js | Florian (décision), PSY | — | PV | `suivi.reprise_close` |
| SOC-331 | Avertir la cellule quand un élève d'une IP récente demande un changement de responsable ou d'adresse (4) | SYS | N-0 | alerte à la cellule, sans bloquer le 4 | `suivi.alerte_dossier` |

### 30.2 Règles ajoutées
| # | Règle | Lot |
|---|---|---|
| R-SOC33 | **Révélation** : l'adulte n'enquête pas ; le formulaire n'a aucun champ « avez-vous vérifié » | SO3 |
| R-SOC34 | **Photo de preuve** : jamais dans la galerie du téléphone ; capture faite dans l'appli, chiffrée | SO10 |
| R-SOC35 | **Escalade automatique** d'une préoccupation non prise ; jamais de silence | SO3 |
| R-SOC36 | **Mesure de protection connue** : contact ASE au 4 sans motif ; détails au 36 | SO2 |
| R-SOC37 | **Aide d'urgence en nature** (repas) sans facture ni libellé révélateur | SO6 |
| R-SOC38 | **Compte de la cellule** : passkey obligatoire, sessions courtes (D-011), révocation immédiate | SO1 |
| R-SOC39 | **Questionnaires nominatifs de repérage interdits** dans le 36 | SO7 |
| R-SOC40 | **Changement de responsable ou d'adresse** d'un élève d'une IP récente : alerte à la cellule, jamais blocage silencieux | SO9 |

### 30.3 Cas limites ajoutés (CSOC-39 à CSOC-50)
| # | Cas | Réponse |
|---|---|---|
| CSOC-39 | Surveillant de nuit seul avec une révélation à 2 h | 15 / 17 si danger ; astreinte ; préoccupation enregistrée ; IP le matin |
| CSOC-40 | Parent qui se présente à l'accueil juste après une IP et exige son enfant | 14 : remise selon le 4 ; si interdiction ou mesure : consigne ; sinon remise et information de la cellule |
| CSOC-41 | La CRIP demande des compléments par téléphone | réponse par la cellule, consignée en complément (SOC-142) |
| CSOC-42 | Élève placé à l'ASE, famille d'accueil à contacter | SOC-309, SOC-310 |
| CSOC-43 | Professeur qui veut savoir « ce que ça a donné » | « pris en compte » ; rien de plus (R-SOC11) |
| CSOC-44 | Psychologue qui reçoit dans un bureau partagé | réglage d'agenda « lieu confidentiel » ; signalé à la direction (local) |
| CSOC-45 | Commission du fonds social sans quorum | décision d'urgence du chef, ratification |
| CSOC-46 | Aide votée mais famille partie | annulation, aucun versement |
| CSOC-47 | Compte d'un membre de la cellule compromis | SOC-326 ; périmètre de fuite (SOC-245) |
| CSOC-48 | Élève qui envoie une demande à 3 h du matin pendant les vacances | SOC-311 ; numéros ; lecture à la reprise |
| CSOC-49 | Signalement Next.js mentionnant un danger ancien non traité | le psychologue le traite comme une préoccupation nouvelle (SO12) |
| CSOC-50 | Restauration d'un établissement seul (PRA) | destructions rejouées ; file réservée de protection neutralisée ; aucune relance renvoyée |

### 30.4 Réglages ajoutés
S-2728 (escalade : délégué à 24 h, chef à 48 h), S-2729 (durée du brouillon hors ligne : 24 h), S-2730 (réponse préécrite hors horaires), S-2731 (revue des situations de veille : 6 mois).

### 30.5 Lots révisés
**SO0** ajoute R-SOC38 (passkey pour la cellule, dès que la cellule existe) ; **SO2** : SOC-307 à SOC-310, R-SOC36 ; **SO3** : SOC-300 à SOC-305, R-SOC33, R-SOC35 ; **SO6** : SOC-317 à SOC-320, R-SOC37 ; **SO7** : R-SOC39 ; **SO9** : SOC-331, R-SOC40 ; **SO10** : SOC-302, SOC-303, R-SOC34 ; **SO12** : SOC-329, SOC-330. Migrations de la passe : **2732** (`ent_sui_mesure_connue`, `ent_sui_preoccupation_escalade`), **2733** (`ent_fs_aide_nature` pour les aides en nature), 2734–2737 réserve.

**Précisions de la passe** : R-SOC32 (résultats des élèves suivis) ; mode discret de l'élève (§21) ; `12_ASSIDUITE.md` RA-21 « socle 3 » est une coquille pour « socle 36 » (à corriger au 12).

**Compte final** : **153 actions** (121 + 32), **50 cas limites**, **40 règles**, **12 réglages**, **14 lots**, **20 questions** (+ Q-SOC21 à Q-SOC24 au `PERSONAS_36.md` §6).
