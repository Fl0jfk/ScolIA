# Passe « personas » sur le socle 34 — Santé scolaire et infirmerie

**Date :** 2026-11-10 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§8.2 : hébergement HDS et coût, DPO, référents médicaux). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production. Le code Next.js a été **lu seulement** (`git show origin/main:<fichier>`, rien extrait, exécuté, construit ni modifié), de façon ciblée sur le module santé.
**Branche :** `ent-socle-sante-personas`, partie de `origin/ent-mc0hoe`.
**Éprouve :** `docs/socles/34_SANTE.md` (lu en entier), avec `CLAUDE.md`, `docs/DECISIONS.md` (D-031, **D-034**, **D-035**, **D-036**, propositions **D-037 à D-046**), `docs/PRA_SAUVEGARDES.md` (exigences **E1–E14**, §18.4), `docs/PORTABILITE_EXPORTS.md` (§4 « procédure particulière », §5), `docs/TRANSITION_T0_INVENTAIRE.md` (M13 santé, R-08), et les points santé de `PERSONAS_16.md` (P06, M-2336 à M-2339, §6.3) et `PERSONAS_42.md` (P08, M-2240 à M-2243, M-2261). Forme reprise de `PERSONAS_10.md`, `PERSONAS_16.md` et `PERSONAS_42.md`.
**Ajouts :** faits **directement** dans `34_SANTE.md`, marqués **« ajout personas »** (nouveau **§30**, plus des renvois aux §0, §6.1, §13, §14, §24, §27, §28). **Rien n'est supprimé** ; quand le §30 contredit le texte d'origine, il l'emporte et la contradiction est écrite au §28.
**Numérotation :** manques **M-2501 à M-2591** (plage libre : le plus haut numéro pris sur les branches distantes est M-2412) ; existant Next.js **NXS-01 à NXS-12** ; défauts **SD-25 à SD-28** ; actions **SAN-300 à SAN-332** ; règles **R-S35 à R-S52** ; cas limites **CS-59 à CS-70** ; questions **Q-S29 à Q-S40** ; réglages **S-460 à S-466** (le plus haut pris : S-456).

---

## 0. À lire en premier : cinq bloquants et leur recommandation (appliquée par défaut, D-031)

La conception est remarquable sur le fond : le secret y est pensé avant le partage, les défauts de l'existant sont trouvés ligne à ligne, le réglementaire est lu. En la faisant vivre par vingt-quatre personas, cinq trous apparaissent. Aucun ne remet en cause la thèse (§1 du 34) ; tous concernent ce qui se passe **avant** qu'un cabinet HDS existe, c'est-à-dire l'année qui vient.

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut, D-031) | Lots |
|---|---|---|---|---|
| **A** | **Il n'existe pas de vrai mode « sans donnée de santé ».** Le 34 dit lui-même que « repas sans arachide » est une donnée de santé (CJUE C-184/20) et en conclut que N-1 n'est que « sans dossier de soins ». Mais **N-0** garde dans l'appel « envoyé à l'infirmerie » : c'est aussi une donnée de santé indirecte. Il n'y a donc **aucun niveau** que l'établissement peut ouvrir sans avis du DPO, alors que le pilote en a besoin dès la démo de décembre 2026 et l'année de doublon. | Sans plancher clair, soit on montre « à l'infirmerie » sans base juridique, soit on attend l'avis du DPO pour la moindre sortie de cours. | **N-0 devient « sans donnée de santé »** au sens strict (R-S35) : ScolIA ne conserve **rien** qui révèle la santé d'un élève, pas même un lieu ; l'appel et la vie scolaire disposent d'un statut neutre **« sorti avec autorisation »** (sans lieu ni motif) ; contacts d'urgence, personnes autorisées et interdictions (socle 4) restent disponibles, car ce ne sont pas des données de santé. N-0 est le **défaut**, sans HDS ni avis du DPO. **N-1** reste « sans dossier de soins », **sans HDS**, après **avis écrit du DPO** (Q-S2 inchangé). Les données de santé indirectes des **autres** socles (motif d'absence « maladie » au 12, dispositifs au 35, dispenses au 21) sont listées dans un registre (§30.1 du 34) et couvertes par l'AIPD générale de l'ENT, pas par le mode du 34. | S0 ; Q-S29 |
| **B** | **La bascule des voyages d'avril 2027 dépend de N-1, et personne ne l'a daté.** Next.js fournit aujourd'hui aux encadrants des **extraits santé** par voyage (`app/api/travels/sante-extraits/route.ts`, NXS-08) ; le socle 42 bascule du 17 avril au 3 mai 2027 (`PERSONAS_42.md`). Si N-1 n'est pas actif à cette date (avis du DPO, lots S0 à S3, S7), le module voyages de ScolIA part **sans trousses ni consignes** : recul de parité sur un sujet de sécurité. | Un séjour à l'étranger sans fiche d'urgence ni trousse cochée, c'est le risque le plus grave du module voyages. | **Jalon « N-1 pour la bascule »** : avis écrit du DPO demandé **avant fin janvier 2027** ; lots **S0, S1, S2 (consignes et trousses), S3 (registre des PAI) et S7-min** (récapitulatif de sortie, trousses emportées, fiches d'urgence hors ligne) prêts **avant le 16 avril 2027**. **Repli** si ce n'est pas prêt : fiche sanitaire papier remise par l'infirmière à l'encadrant pour les séjours repris (**changement d'usage signalé**), et l'écran du 42 affiche « santé : voir l'infirmière » au lieu de rien. Les extraits Next.js **ne sont pas repris** (texte libre, aucun journal, NXS-07) : l'infirmière **recode** les consignes en catalogue fermé, élève par élève, à partir de ses PAI (SAN-300). | S0–S3, S7-min ; Q-S30 |
| **C** | **La santé n'est pas branchée sur les sauvegardes, la restauration et la portabilité (D-037 à D-046).** Quatre trous : (1) une **destruction** à l'échéance (R-S25) **ressuscite** si l'on restaure une sauvegarde antérieure ; (2) les sauvegardes de N-2 partent vers deux destinations (D-040), qui doivent **chacune** être certifiées HDS (la sauvegarde est une activité HDS à part entière, R1111-9 6°) ; (3) `PORTABILITE_EXPORTS.md` §4 fait approuver l'export santé par « la direction **et** le responsable santé » sans dire que la direction **ne voit pas** le contenu ni à qui l'archive est lisible ; (4) une seule clé, `ENT_SECRETS_KEY`, chiffre aujourd'hui IBAN, santé et psychologie (`PRA_SAUVEGARDES.md` §2.3) : impossible d'effacer la santé d'un établissement sans effacer le reste. | Restaurer ou exporter, c'est précisément le moment où un secret fuit ou réapparaît. | **Règles R-S44 à R-S47** (§30.3 du 34) : **journal des destructions rejoué** après toute restauration, avant réouverture ; **cabinet sauvegardé à part**, chaque destination certifiée HDS, jamais extrait par `ent-extraction` de la base principale ; **export santé** initié par l'infirmière titulaire (ou le DPO), approuvé par le DPO et par la direction **sur une vue sans contenu**, archive **chiffrée pour un destinataire nommé** (soignant successeur, nouvel hébergeur) et illisible des approbateurs comme de ScolIA ; **clés santé** propres par établissement et par niveau, avec identifiant (E11) et séquestre séparé (D-042) : leur destruction en fin de contrat **efface aussi les sauvegardes**. Toutes les tables `ent_san_*` sont classées **« particulière »** dans le registre (E12). | S0 (clé), S1, S9 ; Q-S31 |
| **D** | **L'IA lit encore des données de santé.** Le §13 du 34 donne à l'IA les **consignes N-1** (« comme la personne les voit ») : `sante.recapituler_sortie`, `sante.expliquer_consigne`. Or ces consignes sont des données de santé (C-184/20, le 34 le dit lui-même), et la consigne de Florian est : **l'IA ne touche jamais une donnée de santé**. | Contradiction directe avec une consigne du propriétaire du produit ; une IA qui récapitule « 2 trousses PAI : Léa, Tom » envoie des données de santé à un fournisseur. | **Aucune capacité IA sur une donnée du socle 34, quel que soit le niveau** (R-S48). L'IA garde la seule capacité **C0** : aider à rédiger une **information collective** à partir des modèles validés, sans aucune donnée d'élève. À une question sur la santé d'un élève, elle répond « je n'ai pas accès à la santé des élèves » et donne le **lien vers l'écran** qui répond (récapitulatif de sortie, fiche de l'élève). Les contextes IA **des autres socles** (voyages, emploi du temps, vie scolaire) filtrent les ports du 34 ; un test le vérifie. | S14 ; Q-S32 |
| **E** | **Un établissement sans infirmière ne peut pas utiliser le module.** Tout repose sur le « cercle de soins » (au moins un titulaire, SAN-107). Or beaucoup de collèges privés et de petits établissements n'ont **ni infirmière ni médecin** [H], et ont pourtant des **PAI** (signés par un médecin traitant ou de PMI), des trousses et des consignes de cantine. | Ce sont justement les établissements où une consigne mal transmise est la plus dangereuse : personne n'est là pour rattraper. | **Mode « sans infirmière »** (R-S43, §30.2 du 34) : en N-1 seulement, le chef désigne un **référent PAI** non soignant (fonction du socle 6, souvent CPE ou secrétariat) qui tient le **registre des PAI** et publie les **consignes** à partir d'un PAI ou d'un certificat **signé par un médecin** (pièce vue, non stockée) ; pas de mouvements d'infirmerie (il n'y en a pas) mais des **premiers secours** saisis par les adultes formés ; liste des **adultes formés** (PSC1, SST) présents aujourd'hui. Jamais de soins, jamais de pli. **Soumis au DPO** avec l'avis N-1. | S1, S2, S3 ; Q-S33 |

**Un risque de production à signaler (Next.js reste intouchable).** L'audit en lecture du module santé de Next.js (§3, NXS-10) montre que **direction, administrateurs, secrétariat, CPE, surveillants, psychologue et infirmerie** ont accès aux mêmes routes `/api/sante/*`, en lecture et en écriture, que les antécédents, notes de soins, médicaments et PAI sont stockés **en clair**, sans journal des accès, et que leur suppression suit celle de l'élève (`ON DELETE CASCADE`). **Si ce module est utilisé aujourd'hui**, des données de soins sont hébergées hors HDS et lisibles hors du cercle de soins. Le pilote ne touche à rien ; **Florian décide** (Q-S34 : le module est-il utilisé ? faut-il le restreindre ou l'arrêter ?).

---

## 1. Méthode

**Démarche.** Celle de `PERSONAS_42.md` §1 et `PERSONAS_16.md` §1 : vingt-quatre personas réalistes, prises dans la consigne et dans le tableau « acteurs » du §3 du 34, couvrant le **lycée pilote** (privé sous contrat, OGEC, une infirmière salariée), un **collège public** fictif (infirmier de l'Éducation nationale qui trace dans LIEN) et un **collège privé sans infirmière**. Pour chacune : sa journée ou son année, ses **irritants**, ses **peurs**, ce qui la **ravirait** ; les **manques** et **contradictions** ; les cas limites.

**Grille transversale** posée à chaque persona : **(1) niveau** — ce qu'elle obtient en N-0 (sans donnée de santé), en N-1 (sans dossier de soins, après avis du DPO), en N-2 (cabinet sous HDS) ; **(2) partage minimal** — ce qu'elle voit, ce qu'elle ne doit jamais voir, ce qui se lit dans un journal ; **(3) trace** — qui sait qu'elle a lu ; **(4) temps** — ce qui change à une date (rentrée, majorité, départ, fin de contrat, restauration d'une sauvegarde) ; **(5) D-036** — ce qu'elle saisit est-il une colonne, une table, ou un tableau ou un bloc chiffré qui cache une liste ? **(6) IA** — l'assistant voit-il quelque chose ?

**Audit de parité [F].** Lecture ciblée de `origin/main` : migrations `drizzle/0052` (table `sante_extrait`), `0054`, `0055`, `0056` ; routes `app/api/sante/*` et `app/api/travels/sante-extraits/route.ts` ; `app/lib/intranet-modules.ts` (rôles du module `sante`), `app/lib/intranet-auth.ts` (`requireModule`), `app/lib/sante-*-db.ts`, `app/lib/infirmerie-*-db.ts`, `app/lib/travels-sante-extraits.ts`. Aucune donnée lue (base interdite). Résultat au §3.

**Audit de l'existant Rust [F].** Relecture des migrations 040, 045, 052, 053, 210 à 216 de la branche (`ent-rust/migrations/`) sous l'angle E1–E14 et D-036 : pas de `JSONB` ; des tableaux `TEXT[]` (`211:14`) ; un `BYTEA` hérité (`053:11`) ; une clé étrangère **non composite avec `ON DELETE CASCADE`** sur les signatures du PAI (`210:22`). Résultat au §3.3.

**Vérification réglementaire.** Un sous-agent de recherche a contrôlé les points neufs le 2026-11-10 (Légifrance, BO, eduscol, cnil.fr, esante.gouv.fr, sites des hébergeurs). Résultats au §5, versés au §5 du 34 par renvoi.

**Gravité** (identique aux passes précédentes) : **bloquant** · **important** · **confort** · **différenciant**.

**Conventions.** [V] lu sur une source officielle ; [Ve] extrait officiel vu ; [NT] non trouvé ; [NV] non vérifié ; [H] hypothèse ; [R] recommandation ; [F] fait vérifié dans le code.

**Limites.** Les personas sont construites, pas interviewées : la pratique de l'infirmerie (file d'attente, appels, cahier papier, fêtes de classe, carte de sortie) est une hypothèse fondée sur l'usage courant [H], à corriger par l'infirmier et le médecin référents (Q-S3, toujours soumis à Florian). Aucune donnée réelle n'a été lue.

---

## 2. Les personas en un tableau

| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **Mme Lefèvre**, infirmière salariée de l'OGEC, **seule pour 800 élèves** ; **M. Caron**, infirmier de l'EN (LIEN) | Lycée pilote ; collège public | File d'attente, urgences, appels, renvois, double saisie, fin de fonction |
| P02 | **Dr Martin**, médecin scolaire de secteur, **une demi-journée** par mois | Collège public (et lycée pilote : médecin traitant ou PMI) | Préparer sa vacation, signer, ne voir que ce qu'il faut |
| P03 | **Mme Nguyen**, psychologue de l'Éducation nationale | Collège public | Hors du cercle de soins, coordination consentie, rendez-vous discrets |
| P04 | **Mme Benali**, CPE, et **Yanis**, AED | Lycée pilote | « Il est où ? », malaise en permanence quand l'infirmerie est fermée, curiosité |
| P05 | **M. Haddad**, professeur de mathématiques, et **Mme Kessler**, sa remplaçante pour trois semaines | Lycée pilote | Connaître la conduite à tenir sans le diagnostic ; la remplaçante qui ne sait rien |
| P06 | **Mme Roche**, professeure d'EPS | Lycée pilote | Inaptitudes en incapacités, certificat remis en main propre, piscine, accident |
| P07 | **M. Lemaire**, organisateur du séjour à Séville, et **M. Perrin**, parent bénévole | Lycée pilote | Trousses, fiches hors ligne, traitement en voyage, étranger, bascule d'avril |
| P08 | **Mme Duval**, cheffe d'établissement | Lycée pilote | Méningite (ARS), gastro-entérite, confinement PPMS, prouver que le PAI a été respecté |
| P09 | **Mme Garcia**, secrétariat | Lycée pilote | Enveloppes fermées, certificats d'absence, statut vaccinal, perte d'accès |
| P10 | **M. Robin** et **Mme Aubert**, séparés ; le père **refuse** le traitement de fond de Jules (asthme) | Lycée pilote | Opposition d'un titulaire, urgence, chacun pour soi |
| P11 | **Mme Vasseur**, mère qui **veut tout voir** | Lycée pilote | Avis à chaque passage, notes de l'infirmière, qui a lu |
| P12 | **Chloé**, 15 ans, **contraception d'urgence** | Lycée pilote, **en N-1** | Secret par construction alors qu'il n'y a pas de cabinet |
| P13 | **Hugo**, 18 ans | Lycée pilote | Majorité, contacts choisis, partir seul |
| P14 | **Tom**, 13 ans, **diabète de type 1** (pompe, capteur lu sur téléphone) | Collège public | Téléphone autorisé, collation en classe, cantine, sortie au froid |
| P15 | **Inès**, 16 ans, **allergie sévère** (adrénaline), et **M. Faure**, chef de cuisine | Lycée pilote | Menu du jour, fêtes de classe, deux stylos, prestataire |
| P16 | **Nathan**, 14 ans, **épilepsie** | Collège public | Chronomètre de crise, piscine, lumière clignotante, adultes formés |
| P17 | **Sarah**, 16 ans, **troubles anxieux et alimentaires** | Lycée pilote | Carte de sortie, aucune « liste des élèves à risque », retour d'hospitalisation |
| P18 | **Mme Diallo**, AESH mutualisée sur trois élèves | Lycée pilote | Ne voir que ses élèves, gestes prévus par le PAI, remplacement |
| P19 | **M. Ferrand**, enseignant référent (MDPH) et référent handicap | Secteur public | Aucune donnée médicale, avis seulement, PAI / PAP / PPS confondus |
| P20 | **Collège Sainte-Thérèse**, 350 élèves, **sans infirmière** : **Mme Morel** (cheffe), **M. Roux** (CPE, référent PAI) | Privé sous contrat | Cercle de soins vide, PAI du médecin traitant, armoire à pharmacie |
| P21 | **M. Blanc**, DPO de l'OGEC, et une **contrôleuse de la CNIL** | — | Avis N-1, AIPD, durées, journal, fuite, contrôle sur place |
| P22 | **Les attaquants** : l'AED curieux, la professeure qui exporte, l'élève qui photographie l'écran du self, l'ex-infirmière, le faux parent au téléphone, le support | — | Toutes les portes : pastille, accès d'urgence, impression, capture, compte partagé, journal |
| P23 | **Florian**, administrateur : **restauration** d'un établissement, **export** de portabilité, fin de contrat | Plateforme | Destructions ressuscitées, sauvegardes HDS, clés, archive lisible de qui |
| P24 | **Équipe produit** pendant la **démo de décembre 2026** | Labo | Montrer la santé sans donnée de santé |

---

## 3. Audit de parité avec Next.js et de l'existant Rust (faits de code)

### 3.1 Ce que fait le module santé de Next.js (NXS, ajouté au §2.4 du 34)

| # | Fonction [F] | Où | Devient dans ScolIA |
|---|---|---|---|
| NXS-01 | **Passages** : ouverture, recherche d'élève, clôture avec **suite** (`repos`, `retour_cours`, `renvoi_famille`, `urgence`, `autre`), **notes de soins** en texte libre, **contexte** `journee` / `nuit_internat` | `drizzle/0054`, `0056` ; `app/api/sante/passages/route.ts` ; `app/lib/infirmerie-passages-db.ts` | mouvements N-1 (SAN-121, SAN-122) ; soins en N-2 seulement (SAN-130) ; **contexte « nuit d'internat »** à ajouter au mouvement (SAN-301) |
| NXS-02 | **Fiche infirmerie** : antécédents, **personnes à prévenir** et notes, en texte libre | `0054` ; `infirmerie-fiche-db.ts` | antécédents et notes : dossier N-2 ; personnes à prévenir : **socle 4** (joignables), jamais du texte libre |
| NXS-03 | **Prises de médicaments** (médicament, dose, heure, auteur, pièce) | `0054` | N-2 (SAN-162) |
| NXS-04 | **Accidents** (date, circonstances, soins, suite, lieu) | `0054` | déclaration sans diagnostic (SAN-137) ; soins en N-2 |
| NXS-05 | **PAI** : statut `brouillon` / `valide` / `expire` / `revoque`, protocole et traitements autorisés **en clair**, pièce, dates, validé par | `0055` ; `sante-pai-db.ts` | registre N-1 (SAN-190) ; contenu N-2 ; versions figées (S3) |
| NXS-06 | **Inaptitudes EPS** : dates, **motif** texte, libellé publié, pièce | `0056` | incapacités du modèle de 1989, **sans motif** (SAN-240) |
| NXS-07 | **Extraits santé** par **portée** : `cantine`, `eps`, `voyage`, `internat`, **`periscolaire`** ; **libellé en texte libre** | `0052` ; `sante-extraits-shared.ts` | consignes du catalogue fermé ; **audience « périscolaire » absente du 34** (ajoutée, M-2507) |
| NXS-08 | **Extraits « voyage »** lus par l'écran du séjour pour ses participants | `app/api/travels/sante-extraits/route.ts`, `travels-sante-extraits.ts` | récapitulatif de sortie (SAN-220) ; **jalon de la bascule** (bloquant B) |
| NXS-09 | Désactivation « douce » (révocation, `actif = false`), jamais de `DELETE` applicatif | `sante-pai-db.ts:440`, `sante-extraits-db.ts:153` | conforme à R-S9 ; à garder |
| NXS-10 | **Accès** : module `sante` ouvert à **directions, `admin`, `administratif`, `cpe`, `surveillant`, `infirmerie`, `psychologue`** et à tout administrateur d'organisation ; une seule garde `requireModule("sante")` pour lire et écrire | `app/lib/intranet-modules.ts:922-931`, `:1158` ; `app/lib/intranet-auth.ts:196-219` | **défaut de production** (§0) ; dans ScolIA : cercle de soins seulement (R-S1) |
| NXS-11 | Extraits « voyage » lisibles de **tout** utilisateur qui entre dans le détail d'un voyage (`canEnterTravelsDetail`), pas seulement des encadrants de ce séjour | `travels/sante-extraits/route.ts:21-37` | audience « sorties » limitée aux encadrants **du séjour** et, pour un bénévole, à **son groupe** (M-2527) |
| NXS-12 | Aucun chiffrement applicatif, aucun journal des accès dans les bibliothèques santé ; clés étrangères `ON DELETE CASCADE` vers l'élève | `0054`, `0055`, `0056` ; `sante-*-db.ts` | R-S7, R-S8 ; aucune cascade sur une donnée de santé (§22 du 34) |

**Changements d'usage à annoncer** (Florian, D-035) : direction, secrétariat, CPE, surveillants et psychologue **perdent** l'accès au module santé qu'ils ont dans Next.js ; les extraits en texte libre deviennent des **codes** ; le motif d'une inaptitude n'est plus saisi ; les soins et les médicaments ne sont plus saisis dans ScolIA tant que N-2 n'existe pas. **Reprise** : aucune donnée de soins (passages, fiches, médicaments, accidents, contenu des PAI) n'est reprise sans hébergement HDS (`TRANSITION_T0_INVENTAIRE.md` R-08, inchangé) ; à l'arrêt du module Next.js, l'infirmière **exporte pour elle** ce qu'elle doit garder (papier, LIEN dans le public), puis Florian fait détruire (Q-S34) ; les consignes sont **recodées** à la main (SAN-300).

### 3.2 Défauts nouveaux de l'existant Rust (ajoutés au §2.2 du 34)

| # | Gravité | Défaut | Où [F] | Lot |
|---|---|---|---|---|
| SD-25 | Moyenne | Signatures du PAI liées par une **clé étrangère non composite** avec **`ON DELETE CASCADE`** : supprimer un PAI efface la preuve des signatures ; contraire à E2 et au §22 du 34 | `210_sante_pai_complet.sql:22` | S0 |
| SD-26 | Faible | **Tableaux sans clé étrangère** : `audiences TEXT[]` des contraintes ; le §22 du 34 en ajoute (`incapacites TEXT[]`, `active_par[]`) ; la conduite à tenir et les sections du PAI sont prévues « chiffrées » sans dire si c'est un bloc unique (qui cacherait une liste, D-036) | `211_sante_contrainte.sql:14` ; §22 du 34 | S2, S3, S5 |
| SD-27 | Élevée | **Une seule clé** (`ENT_SECRETS_KEY`, format `enc:v1:` sans identifiant) chiffre IBAN, santé et psychologie : pas de clé par établissement ni par niveau, pas d'effacement cryptographique possible pour la seule santé | `PRA_SAUVEGARDES.md` §2.3 (fait relevé par la mission PRA) | S0 (inventaire), S1 (clé santé) |
| SD-28 | Moyenne | Le statut N-0 « envoyé à l'infirmerie » conservé dans l'appel est une donnée de santé indirecte : N-0 n'est pas « sans donnée de santé » | §6.1 du 34 | S0 |

---

## 4. Parcours et manques, persona par persona

### P01 — Mme Lefèvre (lycée pilote, seule pour 800 élèves) et M. Caron (collège public, LIEN)

**Situation.** Un lundi de décembre : 47 passages, six élèves qui attendent en même temps à 10 h, deux renvois au domicile, un appel au 15 pour une crise d'asthme, le téléphone qui sonne pendant qu'elle soigne. M. Caron, lui, saisit déjà chaque acte dans LIEN.
**Irritants.** Taper deux fois ; « il est où ? » de la vie scolaire ; un « père » au téléphone qu'elle ne connaît pas. **Peurs.** Rendre un enfant à la mauvaise personne ; qu'une remplaçante ne sache rien ; qu'à son départ son compte reste ouvert. **Ce qui la ravirait.** Arriver, scanner, soigner ; que l'appel se mette à jour tout seul ; qu'à 17 h le registre soit fait.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2501 | **File d'attente** : plusieurs élèves scannés à l'arrivée, ordre de prise en charge, « signalé urgent » par l'adulte qui envoie (case, sans motif) | Six élèves à la porte, un seul en détresse | important | 34, 12 | SAN-302 |
| M-2502 | **Saisie différée** : mouvement saisi après coup avec **heure déclarée** distincte de l'heure de saisie, tracée | Pendant une urgence, personne ne saisit | important | 34 | SAN-301, R-S9 (inchangée) |
| M-2503 | **Appel entrant non identifié** : ne jamais confirmer la présence d'un élève à un numéro inconnu ; bouton « rappeler un joignable » (numéros du socle 4) | Un parent interdit ou un inconnu peut savoir où est l'enfant | important | 34, 4 | SAN-303, R-S36 |
| M-2504 | **Public** : copie du registre du jour **à l'écran** pour la recopier dans LIEN (aucun fichier), ou choix « mouvements seulement » | Pas de double saisie imposée (R-S31) | important | 34 | SAN-304 |
| M-2505 | **Fin de fonction** : l'accès de l'infirmière se ferme le jour de sa fin de fonction (socle 6) ; alerte « cercle de soins vide » à la direction, bascule en mode « sans infirmière » | Un compte qui survit au contrat ; un module sans soignant | important | 34, 6 | SAN-305, R-S43 |
| M-2506 | **Conflit appel / infirmerie** : élève parti sans billet, noté absent par le professeur, arrivé à l'infirmerie ; l'arrivée remplace l'absence **avant** l'envoi de l'avis d'absence (délai de l'avis au socle 12) | Les parents reçoivent « absent » alors qu'il est soigné | important | 12, 34 | §6 (socle 12) |
| M-2507 | **Audiences « périscolaire » et « nuit d'internat »** (NXS-01, NXS-07) | Parité avec Next.js ; garderie, étude du soir, internat | important | 34 | §30.9 du 34 |

### P02 — Dr Martin, médecin scolaire de secteur

**Situation.** 12 établissements ; au collège, une demi-journée par mois. Dans le lycée pilote, pas de médecin scolaire : les PAI sont signés par le médecin traitant ou de PMI.
**Peurs.** Signer une version qui change ; que l'établissement lise ce qu'il écrit. **Ce qui le ravirait.** Arriver avec la liste prête et repartir avec tout signé.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2508 | **Préparer la vacation** : file « à voir » préparée par l'infirmière (PAI à signer, inaptitudes de plus de 3 mois, avis PAP, élèves à recevoir) avec créneaux (socle 19) | Une demi-journée par mois ne se perd pas | important | 34, 19 | SAN-306 |
| M-2509 | **Plusieurs organisations** : un même médecin pour des établissements de clients différents ; une identité, des casquettes par établissement, aucune vue croisée (CS-50, CS-51 précisés) | Identité partagée (`PRA_SAUVEGARDES.md` §2.4) ; restauration et export d'un établissement ne doivent rien emporter de l'autre | important | 34, 6 | CS-59 |
| M-2510 | Médecin **externe** (traitant, allergologue) : jamais de compte en N-1 ; signature consignée d'après le document (SAN-174) par l'infirmière **ou le référent PAI** (mode sans infirmière) | P20 | important | 34 | SAN-174 (précisé) |

### P03 — Mme Nguyen, psychologue de l'EN

**Situation.** Dans Next.js, le rôle `psychologue` a accès au module santé (NXS-10). Dans ScolIA, elle n'est pas dans le cercle de soins (socle 36).
**Peurs.** Que son dossier se mélange à celui de l'infirmière ; perdre ce qu'elle voyait. **Ce qui la ravirait.** Savoir, avec l'accord de l'élève, qu'il passe souvent à l'infirmerie avant les contrôles.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2511 | **Perte d'accès annoncée** : changement d'usage signalé (NXS-10) ; coordination par échange consenti (SAN-232), y compris **en N-1** pour les seuls mouvements (dates, sans motif) | Elle voyait tout dans Next.js ; le consentement doit exister avant le cabinet | important | 34, 36 | SAN-232 (N-1) |
| M-2591 | **Protocole « santé mentale » de l'établissement** (circulaire interministérielle DGOS/DGESCO/2026/58 du 29/06/2026 [V, résumé]) : repérage par les seuls médecin, infirmier, psychologue et assistant social ; orientation « coupe-file » avec accord des parents ; ScolIA porte le **protocole générique** (comme SAN-211) et l'**accord parental**, jamais une liste d'élèves | Texte nouveau, obligation par établissement | important | 34, 36 | SAN-332 |
| M-2512 | **Rendez-vous partagés** dans un même local : créneaux « rendez-vous » sans auteur ni motif pour la vie scolaire et la famille | Le lieu révèle la consultation | important | 19, 34, 36 | R-S41 |

### P04 — Mme Benali (CPE) et Yanis (AED)

**Situation.** Malaise d'un élève en permanence à 12 h 40 ; l'infirmerie est fermée (pause). Yanis, curieux, aimerait savoir « qui a un PAI » dans la classe de son petit frère.
**Ce qui les ravirait.** Avoir la fiche d'urgence de l'élève en deux gestes, à n'importe quelle heure. **Peur (CPE).** Être accusée d'avoir lu un dossier.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2513 | **Accès d'urgence de la vie scolaire à la fiche minimale** de n'importe quel élève (contacts autorisés, consignes, lieu de la trousse, numéros d'urgence) : motif en un geste, tracé, signalé à l'infirmière, alerte de lecture massive | Le malaise n'attend pas que l'élève soit « dans l'audience » | **bloquant** | 34 | SAN-307, R-S37 |
| M-2514 | **Pastille PAI** : le CPE ne la voit que pour les élèves dont il a la charge (secteur, niveaux, socle 6) ; ailleurs, seulement par l'accès d'urgence | La matrice §9.1 du 34 lui donne « ✔ » pour tous : liste des élèves à PAI de fait | important | 34 | R-S38 |

### P05 — M. Haddad (mathématiques) et Mme Kessler (remplaçante)

**Situation.** Dans sa 4ᵉ B : Inès (allergie), Nathan (épilepsie), Tom (diabète). Mme Kessler arrive lundi pour trois semaines ; mardi, une AED surveille la classe en permanence.
**Peur.** Ne pas savoir quoi faire, ou savoir trop. **Ce qui le ravirait.** Une icône dans l'appel, la conduite à tenir en un geste, un bouton « urgence en classe ».

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2515 | **Audience calculée sur « qui a l'élève maintenant »** : séance du jour, **remplacement**, permanence, salle d'étude, intervenant, sortie (socle 10), et pas seulement « professeurs de la classe » | La remplaçante et l'AED de permanence ne savent rien ; c'est le jour où l'accident arrive | **bloquant** | 34, 10 | R-S39 |
| M-2516 | **Bouton « urgence en classe »** dans l'appel : alerte U3 à l'infirmerie et à la vie scolaire (salle, élève), fiche minimale, « appeler le 15 » avec le numéro | Le professeur ne quitte pas sa classe pour courir | important | 34, 12 | SAN-308 |
| M-2517 | **Codes manquants** : `pas_de_nourriture_partagee` (fêtes de classe, goûters), `eviter_latex` (travaux pratiques) | Les accidents allergiques arrivent souvent hors de la cantine [H] | important | 34 | §30.9 du 34 |

### P06 — Mme Roche, professeure d'EPS

**Situation.** L'élève lui tend le certificat en main propre au gymnase. Nathan a la piscine en mars. Une cheville tordue au hand.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2518 | **Certificat reçu par l'EPS** : elle saisit les incapacités (modèle de 1989) et le papier part à l'infirmerie sous enveloppe (« certificat à transmettre », suivi) ; aucun scan en N-1 | Le certificat arrive là où on ne l'attend pas | important | 34 | SAN-309 |
| M-2519 | **Déclaration d'accident** commencée par le **témoin** (professeur, EPS), sans diagnostic | Le §14.9 ne la donne qu'à l'infirmière et à la direction | important | 34 | SAN-137 (droit élargi) |
| M-2520 | **Codes** `surveillance_rapprochee_piscine`, `pas_d_activite_en_hauteur` | Épilepsie, malaises | important | 34 | §30.9 du 34 |

### P07 — M. Lemaire (Séville) et M. Perrin (parent bénévole)

**Situation.** Séjour repris de Next.js le 17 avril 2027, départ le 10 mai. Inès emporte deux stylos d'adrénaline ; un élève prend un traitement chaque soir.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2521 | **Jalon N-1 avant la bascule** (bloquant B) et repli papier | NXS-08 fournit les extraits aujourd'hui | **bloquant** | 34, 42 | Q-S30 |
| M-2522 | **Bénévole** : fiche minimale de **son groupe** seulement (VOY-276 du 42), jamais des autres | NXS-11 : tout le détail du voyage aujourd'hui | important | 34, 42 | R-S40 |
| M-2523 | **Conduite à tenir traduite** pour l'étranger : version traduite **validée par le médecin**, jointe à la version du PAI ; jamais de traduction automatique (R-S20 inchangée) | Un secouriste espagnol doit comprendre | important | 34 | SAN-310 |
| M-2524 | **Traitement pendant le séjour** en N-1 : consigne `traitement_a_heure_fixe_en_sortie` (sans le médicament), remise du médicament et de l'ordonnance à l'encadrant sur papier ; trousse avec **conditions de conservation** (froid) | Le traitement du soir est la vraie charge de l'encadrant [H] | important | 34, 42 | §30.9, SAN-214 (précisé) |
| M-2525 | **Aucune donnée de santé au prestataire** (agence, hôtel), sauf la consigne alimentaire nécessaire au restaurant, par l'export N3 (SAN-205) | `PERSONAS_42.md` P11 ; transferts hors UE | important | 34, 42 | R-S42 |

### P08 — Mme Duval, cheffe d'établissement

**Situation.** Février : un cas de méningite, l'ARS demande les contacts sous 24 h ; mars : gastro-entérite ; mai : exercice PPMS de confinement. Septembre : 60 PAI à signer.
**Peur.** Être accusée d'avoir lu un dossier ; ne pas pouvoir prouver que l'établissement a respecté un PAI. **Ce qui la ravirait.** Une preuve sans contenu.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2526 | **Main courante de crise** horodatée (décisions, appels, courriers), sans donnée médicale nominative hors cercle | La chronologie se reconstitue toujours mal après coup | important | 34, 14 | SAN-311 |
| M-2527 | **Fiche PPMS par zone** : pour chaque zone de confinement, élèves à consigne présents (d'après l'EDT) et lieu des trousses ; imprimée à la rentrée, à jour sur mobile ; le responsable de zone ne voit que sa zone | SAN-285 donne une liste, pas une organisation par zone | important | 34, 14 | SAN-312 |
| M-2528 | **Relevé de conformité d'un PAI**, sans contenu : dates des signatures, consignes publiées et audiences, **lectures** par les adultes désignés, trousses cochées | Après un accident, c'est la seule défense honnête | différenciant | 34 | SAN-313 |
| M-2529 | **Signature en série** des PAI de rentrée : liste, vue d'organisation de chacun, confirmation par PAI | 60 signatures en septembre | confort | 34 | SAN-314 |

### P09 — Mme Garcia, secrétariat

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2530 | **Suivi des enveloppes papier** (« pli papier reçu de la famille X, remis à l'infirmerie le … ») sans contenu | En N-1, les plis numériques sont refusés (Q-S9) : le papier circule, il doit être suivi | important | 34 | SAN-315 |
| M-2531 | **Certificats médicaux d'absence** (socle 12) : « justifié par certificat médical (vu) », document non conservé ou conservé C3 avec durée courte | Le socle 12 stocke aujourd'hui des pièces médicales sans le dire | important | 12 | §6 |
| M-2532 | **Perte d'accès** au module santé de Next.js annoncée (NXS-10) | Changement d'usage | confort | 34 | §3.1 |

### P10 — M. Robin et Mme Aubert, séparés : le père refuse le traitement de fond

**Situation.** Jules, 12 ans, asthme sévère. La mère a signé le PAI (traitement de fond à l'infirmerie, conduite à tenir en cas de crise). Le père écrit qu'il **s'oppose** au traitement quotidien.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2533 | **Règle de l'opposition** : la **partie urgence** (appeler le 15, geste prévu) s'applique **toujours** ; les mesures non urgentes contestées sont **suspendues** jusqu'à la décision du médecin et de la direction ; l'opposition n'est visible que du cercle et de la direction, **jamais** des adultes qui appliquent la conduite à tenir | Un encadrant n'arbitre pas un conflit parental au moment d'une crise | important | 34 | R-S49 |
| M-2534 | Chaque titulaire reçoit sa **propre** information de l'opposition et de la décision, sans les coordonnées de l'autre (R-S17) | — | important | 34, 4 | SAN-179 (précisé) |

### P11 — Mme Vasseur, la mère qui veut tout voir

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2535 | **Pas de réglage famille « chaque passage »** ; l'infirmière peut, pour un élève et une période (suivi d'un traitement), informer à chaque passage, jamais pour l'écoute ni un acte confidentiel | Un adolescent cesse de venir si chaque visite est rapportée [H] | important | 34 | SAN-316, S-460 |
| M-2536 | **Page « ce que l'établissement sait de la santé de votre enfant »** : consignes, registre du PAI, mouvements à issue utile, niveau du module, et comment demander l'accès au dossier (L1111-7) | Transparence qui désamorce les demandes | différenciant | 34 | SAN-317 |
| M-2537 | **Qui a lu** : réponse par le DPO (catégories de destinataires, dates et finalités des consultations), sans nommer les agents par défaut [NV] | Droit d'accès RGPD | important | 34 | SAN-292 (précisé) |

### P12 — Chloé, 15 ans, contraception d'urgence, dans un lycée en N-1

**Situation.** Pas de cabinet (pas d'HDS) : l'infirmière tient l'acte sur papier. Chloé quitte le cours de maths à 10 h 10.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2538 | **Mouvement confidentiel dès N-1** : aucun lieu ni avis ; le professeur voit « sortie autorisée » (statut neutre du N-0) ; exclu de la chronologie, des vues et exports familiaux, des statistiques nominatives ; le §10.10 du 34 ne le prévoyait qu'en N-2 | Le pilote sera en N-1 : sans cette règle, l'appel la trahit | **bloquant** | 34, 12 | R-S50, SAN-318 |
| M-2539 | **Compteur anonyme** de l'état chiffré annuel (D5134) : mineure / majeure, mois ; aucune donnée personnelle, donc permis en N-0 et N-1 | L'infirmière en a besoin chaque juin | important | 34 | SAN-319 |
| M-2540 | **Notifications neutres pour l'élève aussi** (« Nouveau message ») sur l'écran verrouillé | R-S15 ne vise que les familles | important | 34, 8, 16 | R-S15 (étendue) |
| M-2541 | **Rendez-vous à l'infirmerie absent de l'emploi du temps vu par la famille** | Les parents lisent l'EDT de l'enfant | important | 10, 19 | §6 |

### P13 — Hugo, 18 ans

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2542 | **Majeur qui ne désigne personne** : en cas de malaise, appel à ses contacts choisis ; sans contact, le 15 ; jamais ses parents d'office | R-S13 ne dit pas qui appeler | important | 34, 4 | CS-60 |

### P14 — Tom, 13 ans, diabète de type 1

**Situation.** Capteur de glycémie lu sur son téléphone ; collège en « pause numérique » (téléphones rangés) [H] ; L511-5 exempte les équipements liés à un trouble de santé invalidant [Ve, §5 R17].

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2543 | **Consigne `telephone_suivi_medical_autorise`**, lue par le socle de discipline et le règlement intérieur : aucune confiscation, aucune sanction pour cet usage | La règle générale d'interdiction s'appliquerait à tort | important | 34, 13 | R-S51 |
| M-2544 | **Codes** `collation_autorisee_en_classe`, `passage_prioritaire_cantine`, `repas_a_heure_fixe` | Les hypoglycémies se préviennent par l'organisation | important | 34, 40 | §30.9 du 34 |
| M-2545 | **Trousse à conserver au froid** (insuline) : champ « conservation » non médical sur la trousse, rappel à l'encadrant | Une insuline chaude ne marche plus | important | 34, 42 | SAN-214 (précisé) |

### P15 — Inès (allergie sévère) et M. Faure, chef de cuisine

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2546 | **Croisement menu × consignes** pour le seul chef : le plat du jour déclare ses allergènes (information obligatoire en restauration collective [NV]) ; ScolIA signale « consigne arachide : plat B contient des fruits à coque, prévoir un substitut » | Le changement de menu du jour est le moment du danger [H] | différenciant | 40, 34 | §6 |
| M-2547 | **Trousse « portée par l'élève »** quand le PAI l'autorise (deuxième stylo), en plus de la trousse rangée | La circulaire range les trousses hors de portée des élèves ; le PAI peut prévoir autrement [H] | important | 34 | SAN-214 (précisé) |
| M-2548 | **Écran de la cuisine** : liste du jour sur une tablette à session, prénom, photo et consigne ; jamais d'affichage mural | Une liste affichée se photographie (P22) | important | 34, 40 | R-S52 |

### P16 — Nathan, 14 ans, épilepsie

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2549 | **Chronomètre de crise** dans la fiche d'urgence (web, mobile) : départ, fin, enregistrés comme premier secours sans texte libre | La conduite à tenir dépend de la durée de la crise [H] | différenciant | 34 | SAN-320 |
| M-2550 | **Adultes formés à un geste du PAI** (nom, date de formation), lus par la fiche d'urgence : « formés présents aujourd'hui » | En urgence, on cherche qui sait faire | important | 34, 6 | SAN-321 |
| M-2551 | **Code** `eviter_lumieres_clignotantes` | Écrans, spectacles | confort | 34 | §30.9 du 34 |

### P17 — Sarah, 16 ans, troubles anxieux et alimentaires

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2552 | **Aucun code de « vigilance psychique »** dans le catalogue ; la vigilance passe par un échange humain (socle 36), jamais par une liste | Une « liste des élèves à risque » serait la donnée la plus dangereuse de l'ENT | important | 34, 36 | R-S4 (étendue) |
| M-2553 | **Carte de sortie** vers un lieu refuge (`sortie_libre_lieu_refuge`) : l'appel montre « sortie autorisée » (statut neutre), jamais le lieu | Elle doit pouvoir sortir sans se justifier devant la classe | important | 34, 12 | §30.9, R-S50 |
| M-2554 | **Signal de passages répétés dès N-1** (mouvements sans motif), pour l'infirmière seule | CS-19 n'existe qu'en N-2 | important | 34 | SAN-138 (N-1) |
| M-2555 | **Retour après hospitalisation** : motif d'absence neutre (« raison médicale »), jamais le service ; aménagement de retour au socle 35 | Le motif d'absence est la fuite la plus banale | important | 12, 35 | §6 |

### P18 — Mme Diallo, AESH

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2556 | Accès **daté** par affectation (socle 6, PIAL) ; remplacement d'AESH : l'accès suit l'affectation du jour, pas le nom saisi en texte libre (`212`, SD-09) | Une AESH change en cours d'année | important | 34, 35, 6 | R-S39 |
| M-2557 | Gestes du PAI : l'AESH n'exécute qu'un geste **prévu par le PAI** et pour lequel elle est **formée** (SAN-321) ; stylo auto-injecteur et glucagon nasal possibles, autre injection non [V, §5 R8] | — | important | 34 | SAN-321 |

### P19 — M. Ferrand, enseignant référent et référent handicap

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2558 | **Vue « accompagnement » unique** qui montre PAI (34), PAP, PPS (35), aménagements, avec le propriétaire de chaque ligne, sans diagnostic | Les personnels confondent PAI, PAP et PPS [H] | important | 34, 35 | §6 |

### P20 — Collège Sainte-Thérèse, sans infirmière

**Situation.** 350 élèves, 9 PAI signés par des médecins traitants, une armoire à pharmacie, trois adultes formés au PSC1. M. Roux (CPE) tient les PAI dans un classeur.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2559 | **Mode « sans infirmière »** (bloquant E) | Le module est inutilisable sans cercle | **bloquant** | 34, 6 | R-S43, SAN-322 |
| M-2560 | **Adultes formés présents aujourd'hui** (PSC1, SST) d'après l'EDT et les habilitations (socle 6) | Le premier secours est leur affaire | important | 34, 6 | SAN-323 |
| M-2561 | **Armoire à pharmacie** collective tenue par le référent (inventaire, péremptions), sans médicament d'élève hors PAI | Protocole de 2000 | important | 34 | SAN-216 (droit élargi) |

### P21 — M. Blanc (DPO) et une contrôleuse de la CNIL

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2562 | **Dossier d'avis du DPO** généré : niveaux, matrice de partage, rapport des tests de séparation, hébergement, sauvegardes (où sont les données de santé, combien de temps), sous-traitants (SMS neutres), durées, liste des données de santé indirectes des autres socles | L'avis écrit N-1 est le premier jalon de B | important | 34 | SAN-324 |
| M-2563 | **Contrôle sur place** : procédure écrite — journal des accès exporté sans contenu par le DPO ; données médicales individuelles communiquées seulement « sous l'autorité et en présence d'un médecin » (loi 78-17, art. 19 III [V, §5 R16]) | Un contrôle sans procédure finit en ouverture de dossiers | important | 34 | SAN-325 |
| M-2564 | **Périmètre d'une fuite** : pour un export, un fichier ou un appareil perdu, la liste des élèves dont des données de santé ont pu fuir (notification CNIL 72 h, information des personnes) | RGPD art. 33 et 34 | important | 34 | SAN-326 |
| M-2565 | **Durées dans les sauvegardes** : une donnée détruite à l'échéance reste jusqu'à 12 mois dans les sauvegardes (D-043) ; la notice le dit ; le journal des destructions est rejoué après restauration | Bloquant C | **bloquant** | 34, PRA | R-S44 |

### P22 — Les attaquants

| ID | Porte | Ce qui la ferme | Gravité | Ajout |
|---|---|---|---|---|
| M-2566 | **AED curieux** : accès d'urgence (M-2513) utilisé pour « voir » | motif, signalement à l'infirmière, seuil de lecture massive bas pour cet accès (S-461 : 3 fiches par jour), revue hebdomadaire par l'infirmière | important | R-S37 |
| M-2567 | **Professeure qui exporte** sa liste avec les consignes vers un tableur | aucun export de consignes hors SAN-205 (N3) ; impression **filigranée** (nom du lecteur, date, « document de santé — ne pas diffuser ») | important | R-S45 |
| M-2568 | **Élève qui photographie** l'écran du self | pas d'affichage mural (R-S52) ; tablette à session, verrouillage 2 min | important | R-S52 |
| M-2569 | **Capture d'écran** sur l'appli mobile (fiche hors ligne) | Android : écran sécurisé (capture bloquée) sur les vues santé ; iOS : masquage dans le sélecteur d'applis, détection de la capture tracée et rappel ; filigrane nominatif | important | R-S46 |
| M-2570 | **Compte partagé** de l'infirmerie sur la tablette | interdit ; déverrouillage rapide par passkey ou badge personnel ; verrouillage automatique | important | R-S47 |
| M-2571 | **Ex-infirmière** ou remplaçante expirée | fin d'accès au jour de fin de fonction (M-2505) ; alerte si tentative | important | SAN-305 |
| M-2572 | **Faux parent au téléphone** | M-2503 | important | R-S36 |
| M-2573 | **Export complet** (D-041) qui emporterait la santé | tables `ent_san_*` en catégorie « particulière », test qui échoue si un export complet en contient | important | R-S44 |
| M-2574 | **Journaux et rapports d'erreur** qui capturent une requête santé | aucun corps de requête santé dans les journaux et rapports d'erreur (E14 étendue) | important | R-S46 |
| M-2575 | **Support ou opérateur** avec accès à l'infrastructure | N-1 : clé santé dans le KMS, jamais lisible du support ; N-2 : périmètre HDS séparé ; limite assumée écrite (D-041) | important | R-S47 |

### P23 — Florian, administrateur : restauration, export, fin de contrat

**Situation 1.** Une erreur de reprise oblige à restaurer le lycée pilote à J-3. **Situation 2.** Un établissement quitte la plateforme et demande ses données. **Situation 3.** Fin du contrat HDS.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2576 | **Journal des destructions rejoué** après toute restauration (globale ou d'un établissement), avant réouverture ; destructions jamais annulées par une restauration | Une donnée de santé détruite qui revient, c'est une faute | **bloquant** | 34, PRA | R-S44, SAN-327 |
| M-2577 | **Effets de bord neutralisés** : aucun avis santé renvoyé après restauration (E9) ; journal des accès restauré avec les données, jamais tronqué | Un « Léa est rentrée chez elle » envoyé une seconde fois | important | 34, PRA | R-S44 |
| M-2578 | **Cabinet restauré à part**, dans le périmètre HDS ; chaque destination de sauvegarde du cabinet certifiée HDS ; `ent-extraction` n'y touche pas | R1111-9 6° | **bloquant** (pour N-2) | 34, PRA | R-S44 |
| M-2579 | **Export santé** : initié par l'infirmière titulaire ou le DPO ; approuvé par le DPO et la direction sur une vue **sans contenu** ; archive chiffrée **pour un destinataire nommé** ; actes confidentiels exclus sauf accord de l'élève | `PORTABILITE_EXPORTS.md` §4 ne dit pas qui lit l'archive | **bloquant** | 34, EXP | SAN-328, R-S45 |
| M-2580 | **Clés santé** par établissement et par niveau, avec identifiant (E11), séquestre séparé (D-042) ; destruction en fin de contrat = effacement des sauvegardes aussi | SD-27 | important | 34, PRA | SAN-329 |
| M-2581 | **Registre de classement** (E12) : `ent_san_*` « particulière » ; tables du cabinet hors registre principal (registre propre) | Une table non classée bloque l'export | important | 34 | §30.3 du 34 |
| M-2582 | **Normalisation D-036 et E2** : `audiences`, `incapacites`, acteurs d'activation en tables filles ; sections du PAI en lignes ; SD-25 corrigé | §3.2 | important | 34 | §30.7 du 34 |

### P24 — L'équipe produit pendant la démo de décembre 2026

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-2583 | **Démo en N-0** : « sortie autorisée » dans l'appel, contacts d'urgence, remise sécurisée avec interdictions (socle 4) ; la santé montrée par une **maquette** sur données fictives, jamais en activant N-1 sur le labo partagé | S0 n'est pas encore livré ; R-S28 | important | 34 | CS-61 |

### Manques transverses : mobile, tests, documentation

| ID | Ce qui manque | Gravité | Ajout |
|---|---|---|---|
| M-2584 | **Mobile (D-003)** : vues santé en écran sécurisé, aucune copie locale hors fiches de sortie, chronomètre de crise, bouton « urgence en classe » | important | S11 |
| M-2585 | **Test « mode sans donnée de santé »** : en N-0, aucune table du 34 ne reçoit de ligne, aucune route ne répond, l'appel n'a pas de lieu | important | §30.10 du 34 |
| M-2586 | **Test « IA sans santé »** : aucun contexte IA d'aucun socle ne contient une donnée du 34 | important | §30.10 du 34 |
| M-2587 | **Test « restauration »** : détruire, restaurer une sauvegarde antérieure, vérifier que la destruction est rejouée et qu'aucun avis ne part | important | §30.10 du 34 |
| M-2588 | **Test « export complet sans santé »** | important | §30.10 du 34 |
| M-2589 | **Notice aux familles** générée par niveau (ce qui est conservé, par qui, combien de temps, sauvegardes comprises) | important | SAN-330 |
| M-2590 | **Formation** : écran d'accueil du module pour chaque rôle (« voici ce que vous voyez, et pourquoi pas plus ») | confort | SAN-331 |

---

## 5. Vérification réglementaire (2026-11-10)

Sous-agent de recherche, 47 consultations. **Limites** : le BO (education.gouv.fr) a refusé l'accès (403) aux circulaires 2015-118, 2017-079, 2017-084, 2019-090 ; plusieurs articles de Légifrance n'ont pas pu être lus ; la liste ANS des hébergeurs certifiés s'affiche vide ; la page HDS de Scaleway a refusé l'accès. Les points marqués [Ve] doivent être relus sur la source officielle avant d'être opposés à quelqu'un.

| # | Point | Réponse | Source | Statut | Effet sur la conception |
|---|---|---|---|---|---|
| R1 | CSP R1110-2 (non-soignants autorisés à échanger) | **psychologues** (qui ne sont pas professionnels de santé) et **assistants de service social** y figurent ; **ni la direction, ni le CPE, ni les professeurs** | traduction du texte en vigueur (27/12/2021) : https://french-business-law.com/french-legislation-art/article-r1110-2-of-the-french-public-health-code/ | [Ve] (non lu sur Légifrance) | confirme R-S1 ; le psychologue peut **échanger** avec l'infirmière (L1110-4 : strict nécessaire, information de la personne) sans entrer dans le cercle de soins : SAN-232 reste la bonne voie |
| R2 | Circulaire 2015-118 (missions des médecins de l'EN) | aucune abrogation trouvée ; contenu non lu | https://www.education.gouv.fr/bo/15/Hebdo42/MENE1517120C.htm | [Ve] statut, [NT] contenu | inchangé |
| R3 | Psychologue de l'EN et secret | le titre ne soumet pas par lui-même à l'art. 226-13 ; l'agent public est tenu au secret par son statut [NT ici] | réponse ministérielle : https://www.assemblee-nationale.fr/dyn/15/questions/QANR5L15QE25960.pdf | [Ve] | dossier du psychologue au socle 36, inchangé |
| R4 | AESH | secret et discrétion ; gestes « ne requérant pas une qualification médicale » et aide à la prise de médicaments **dans le cadre d'un PAI** | guide AESH 2020 : https://cache.media.education.gouv.fr/file/3.BEP-Scolarisation_et_handicap/00/2/Guide_AESH_2020_(2)_1256002.pdf | [Ve] | M-2557, SAN-321 |
| R5 | **Santé mentale** : circulaire interministérielle **DGOS/DGESCO/2026/58 du 29/06/2026** | repérage et évaluation par les seuls médecins, infirmiers, psychologues et assistants sociaux de l'EN ; **un protocole « santé mentale » par collège, lycée et circonscription** ; orientation « coupe-file » autorisée par les parents | https://bulletins-officiels.social.gouv.fr/circulaire-interministerielle-ndeg-dgosp3dgesco202658-du-29-juin-2026-relative-lamelioration-de-lorientation-et-la-prise-en-charge-des-eleves-presentant-des-signes-de-souffrance-psychique | [V] (résumé officiel) | **nouveau** : M-2591, SAN-332 ; confirme M-2552 (aucune liste d'élèves) |
| R6 | C. éduc. L541-1 modifié par la **loi 2026-668 du 27/07/2026** | actions de prévention « une fois par an dès l'école élémentaire », décret à venir | commentaire d'avocat : https://kohenavocats.fr/?p=2057539 | [Ve] (JO non lu) | campagnes (SAN-253) : à suivre au décret |
| R7 | Protocole national des soins et des urgences (BO HS n° 1 du 6/01/2000) | aucun remplacement trouvé ; **sans infirmier**, soins et urgences assurés par des **personnels formés (PSC1 ou SST)**, consignes affichées, pharmacie fermée à clé ; contraception d'urgence exclue | protocole départemental qui reprend le texte : https://lons1.circo39.ac-besancon.fr/wp-content/uploads/sites/8/2018/11/protocole_departemental_d_organisation_des_soins1.pdf | [Ve] | fonde le mode « sans infirmière » (R-S43, SAN-323) |
| R8 | Gestes d'un non-soignant en exécution d'un PAI | la plupart possibles ; **professionnel de santé** seulement pour une injection autre que par **stylo auto-injecteur** ; « aucune décision médicale à prendre » ; glucagon **nasal** administrable par tout adulte, injection par une personne habilitée ; midazolam buccal : [NT] | réponse ministérielle du 07/12/2023 : https://www.senat.fr/questions/base/2022/qSEQ221204556.html | [V] | SAN-321 : le geste est rattaché au PAI et à la formation de l'adulte |
| R9 | Parents séparés | C. civ. 372-2 : présomption d'accord pour un **acte usuel** vis-à-vis des tiers de bonne foi ; une consultation a été jugée acte usuel (TA Paris, 13/12/2011) ; aucune jurisprudence sur le PAI | https://affairesjuridiques.aphp.fr/textes/tribunal-administratif-de-paris-13-decembre-2011-n10028226-3-prise-en-charge-de-mineurs-situation-conflictuelle-entre-les-titulaires-de-lautorite-parentale/ | [Ve] | Q-S10 et R-S49 tiennent : une **opposition écrite connue** fait tomber la bonne foi pour la partie contestée ; d'où la suspension des mesures non urgentes, la partie urgence restant appliquée *(analyse)* |
| R10 | Contraception d'urgence | gratuite et **anonyme** pour les mineures (pharmacie, infirmière scolaire), sans ordonnance ; l'infirmier propose d'associer un parent, la mineure peut refuser (décret 2016-683) | https://www.service-public.gouv.fr/particuliers/vosdroits/F1100 | [V] | R-S50, SAN-319 |
| R11 | **HDS et sauvegarde** | ANS : « chaque acteur doit être certifié pour la ou les activités qu'il réalise » ; la sauvegarde externalisée est l'activité 6° ; activités 1, 2 et 6 : sites dans l'EEE | atelier ANS, SantExpo, 20/05/2026 : https://esante.gouv.fr/sites/default/files/media/document/Atelier%20-%20SantExpo%20-%20Certification%20HDS%2020260520.pdf | [Ve] | **chaque destination de sauvegarde du cabinet est certifiée HDS** (R-S44) ; **si** le DPO juge N-1 dans le champ HDS, la même règle vaut pour ses sauvegardes et « N-1 sans HDS » tombe : à poser explicitement au DPO (Q-S2) |
| R12 | OVHcloud HDS | Object Storage (dont 3-AZ), Cold Storage, Block et File, instances Public Cloud, bases managées ; support Business ou Enterprise et option HDS requis | https://docs.ovhcloud.com/fr/guides/account-and-service-management/account-information/hds-certification | [V] | la seconde destination de D-040 peut être HDS (option et support à chiffrer, Q-S1) |
| R13 | Scaleway HDS | Object Storage HDS en **région Paris** seulement ; option fixée à la création du bucket ; **pas de règles de cycle de vie** ; Managed Database non confirmée | https://www.scaleway.com/en/docs/object-storage/how-to/create-hds-certified-bucket | [Ve] | la purge par rétention des sauvegardes du cabinet est faite **par notre code** ; base du cabinet sur **instance** autogérée (cohérent avec D-039, option B) |
| R14 | Conservation | aucune recommandation CNIL propre à l'infirmerie scolaire ; LIEN : base active pendant la présence de l'élève, puis archives ; le référentiel CNIL santé (20 ans, 28 ans pour un mineur) vise les établissements de santé | https://www.cnil.fr/sites/cnil/files/atoms/files/referentiel_-_traitements_dans_le_domaine_de_la_sante_hors_recherches.pdf | [Ve] | ne pas reprendre 20 ans sans le DPO ; §18 du 34 inchangé |
| R15 | Journal des accès | délibération CNIL 2021-122 : traces conservées **6 mois à 1 an**, jusqu'à **3 ans** si c'est justifié et documenté | https://www.cnil.fr/sites/cnil/files/atoms/files/recommandation_-_journalisation.pdf | [Ve] | les 3 ans du §18 du 34 exigent une **justification écrite** dans l'AIPD (preuve d'accès aux données de santé de mineurs) ; à défaut, 1 an |
| R16 | Contrôle de la CNIL | loi 78-17, art. 19 III : secret médical opposable ; données médicales individuelles communiquées « **sous l'autorité et en présence d'un médecin** » | https://www.cnil.fr/fr/la-loi-informatique-et-libertes | [V] | M-2563, SAN-325 |
| R17 | Téléphone (C. éduc. L511-5) | l'interdiction ne s'applique pas aux équipements des élèves en situation de handicap ou atteints d'un **trouble de santé invalidant** | https://www.education.gouv.fr/interdiction-du-telephone-portable-dans-les-ecoles-colleges-et-lycees-455181 | [Ve] | R-S51, M-2543 |
| R18 | PPMS | circulaire du 08/06/2023 (BO n° 26) : PPMS unifié en trois parties ; rôle de l'infirmerie et trousse : [NT] | https://www.education.gouv.fr/bo/2023/Hebdo26/MENE2307453C | [Ve] | SAN-312 |
| R19 | Élève majeur et dossier infirmier | aucun texte propre trouvé ; principe général du secret | https://www.senat.fr/questions/base/1994/qSEQ940204717.html | [NT] | R-S13 inchangée |

**Non vérifiés dans cette passe** : information obligatoire sur les allergènes en restauration collective (M-2546) ; droit d'accès RGPD aux dates et finalités des consultations (M-2537) ; midazolam buccal par un non-soignant.

---

## 6. Manques qui relèvent d'autres socles

| Socle | Manque | Persona |
|---|---|---|
| 4 | « rappeler un joignable » : numéros de rappel exposés au port `joignables` sans les montrer à l'appelant | P01 |
| 6 | fin de fonction le jour même ; casquettes par établissement et par organisation ; habilitations PSC1 / SST et formation aux gestes d'un PAI | P01, P02, P16, P20 |
| 10 | audience « qui a l'élève maintenant » (remplacements, permanences, intervenants) ; rendez-vous infirmerie jamais dans l'EDT vu de la famille | P05, P12 |
| 12 | statut neutre « sortie autorisée » (sans lieu) ; arrivée à l'infirmerie qui annule une absence avant l'avis ; motifs d'absence neutres (« raison médicale ») ; certificats médicaux d'absence non conservés ou C3 à durée courte | P01, P09, P12, P17 |
| 13 (discipline, règlement) | lire `telephone_suivi_medical_autorise` : pas de confiscation ni de sanction | P14 |
| 14 | main courante de crise ; fiche PPMS par zone | P08 |
| 16 | notifications neutres pour l'élève ; demandes structurées (M-2337, inchangé) | P12 |
| 19 | rendez-vous de l'infirmerie et du psychologue sans motif ni auteur visibles | P03, P12 |
| 35 | vue « accompagnement » unique ; aménagement de retour après hospitalisation | P17, P19 |
| 40 | allergènes du menu du jour ; croisement avec les consignes pour le chef seul ; tablette à session | P15 |
| 42 | jalon N-1 de la bascule ; bénévole limité à son groupe ; aucune donnée de santé au prestataire | P07 |
| IA (30) | filtrer les ports du 34 dans tous les contextes | — |
| PRA / EXP | journal des destructions rejoué ; cabinet hors `ent-extraction` ; export santé chiffré pour un destinataire ; clé santé séparée | P23 |

---

## 7. Synthèse

### 7.1 Manques par gravité

| Gravité | Nombre | Exemples |
|---|---|---|
| Bloquant | 10 | M-2513, M-2515, M-2521, M-2538, M-2559, M-2565, M-2576, M-2578, M-2579 (et le mode N-0 strict du bloquant A, SD-28) |
| Important | 73 | M-2501, M-2503, M-2533, M-2543, M-2552, M-2562, M-2567, M-2580 |
| Confort | 4 | M-2529, M-2532, M-2551, M-2590 |
| Différenciant | 4 | M-2528, M-2536, M-2546, M-2549 |
| **Total** | **91** | M-2501 à M-2591 |

### 7.2 Ajouts dans la conception (`34_SANTE.md`)

- §2.2 : SD-25 à SD-28 ; §2.4 : NXS-01 à NXS-12 (renvoi au §3 de ce document).
- §6.1 : N-0 redéfini « sans donnée de santé » (renvoi au §30.1).
- §13 : capacités IA sur les consignes **retirées** (renvoi au §30.4).
- **§30 (nouveau, « ajout personas »)** : N-0 strict et registre des données indirectes (§30.1), mode sans infirmière (§30.2), sauvegardes, restauration, export et clés (§30.3), IA (§30.4), Next.js et jalon de bascule (§30.5), actions SAN-300 à SAN-332 (§30.6), règles R-S35 à R-S52 (§30.7), cas limites CS-59 à CS-70 (§30.8), catalogue (§30.9), lots révisés dont **S0 complété** (§30.10), réglages et questions (§30.11).
- §24.1 : renvoi vers le §30.10 en tête du tableau ; §27 : renvoi vers les questions Q-S29 à Q-S40 ; §28 : contradictions 11 à 16.

### 7.3 Les dix ajouts les plus importants

1. **N-0 « sans donnée de santé »** au sens strict, activable sans HDS ni avis (R-S35).
2. **Mouvement confidentiel dès N-1** et statut neutre « sortie autorisée » (R-S50).
3. **Audience « qui a l'élève maintenant »** : remplaçants, permanences, intervenants (R-S39).
4. **Accès d'urgence de la vie scolaire** à la fiche minimale, tracé et signalé (R-S37).
5. **Mode « sans infirmière »** avec référent PAI (R-S43).
6. **Jalon N-1 pour la bascule des voyages** d'avril 2027, avec repli papier (Q-S30).
7. **Destructions rejouées** après restauration ; cabinet sauvegardé en HDS sur chaque destination (R-S44).
8. **Export santé chiffré pour un destinataire nommé**, approuvé sans contenu (R-S45).
9. **IA exclue de toute donnée du 34** (R-S48).
10. **Règle de l'opposition d'un titulaire** : l'urgence s'applique toujours (R-S49).

### 7.4 Ce que la passe a montré sur la conception

- Le fond est juste : le secret est tenu par construction, pas par réglage. Le trou était **le plancher** : la conception ne savait pas dire ce que l'on montre quand on ne doit rien montrer.
- Le pilote vivra en **N-0 puis N-1** pendant au moins un an : tout ce que la conception réservait à N-2 et qui protège l'élève (acte confidentiel, signal de passages répétés) doit exister, sous une forme sans soin, **dès N-1**.
- La santé n'a pas de régime de **sauvegarde ni de restauration** propre ; c'est là que le secret et la destruction se jouent, et D-037 à D-046 ne pouvaient pas le deviner.

---

## 8. Décisions : appliquées par défaut (D-031) et soumises à Florian

### 8.1 Appliquées par défaut (D-031)

| N° | Décision appliquée |
|---|---|
| Q-S29 | N-0 = « sans donnée de santé » au sens strict ; statut neutre « sortie autorisée » ; registre des données de santé indirectes des autres socles |
| Q-S32 | Aucune capacité IA sur une donnée du 34 ; seule la rédaction d'informations collectives (C0) reste |
| Q-S35 | Mouvement confidentiel et compteur anonyme de l'état chiffré dès N-1 |
| Q-S36 | Audience des consignes calculée sur qui a l'élève à l'instant (socle 10) |
| Q-S37 | Accès d'urgence de la vie scolaire à la fiche minimale (motif, trace, signalement, seuil bas) |
| Q-S38 | Opposition d'un titulaire : la partie urgence s'applique toujours ; le reste suspendu jusqu'à la décision du médecin et de la direction |
| Q-S39 | Pas de réglage famille « chaque passage » ; décision de l'infirmière par élève et par période |
| Q-S40 | Sécurité d'écran (capture bloquée sur Android, masquage et trace sur iOS), filigrane des impressions, pas d'affichage mural, pas de compte partagé |
| — | Journal des destructions rejoué après restauration ; tables `ent_san_*` « particulière » ; clé santé séparée (inventaire en S0, clé en S1) ; normalisation D-036 (§30.7 du 34) |

### 8.2 Soumis à Florian (engagent l'établissement à l'extérieur, ou sont des faits que seul Florian connaît)

| N° | Question | Pourquoi Florian |
|---|---|---|
| Q-S1 (inchangée, précisée) | **Hébergement HDS et coût** : certification de ScolIA ou infogérance certifiée, contrat Scaleway HDS ; **la seconde destination de sauvegarde du cabinet** doit aussi être certifiée HDS (OVHcloud ou autre, §5) | contrat, budget |
| Q-S2 (inchangée, précisée) | **Avis écrit du DPO** sur N-1 sans HDS, **à demander avant fin janvier 2027** (jalon B), avec le mode sans infirmière et la boîte infirmerie en demandes structurées | engagement extérieur |
| Q-S3 (inchangée) | **Infirmier et médecin référents** du pilote ; ajouter un **référent d'un établissement sans infirmière** | personnes réelles |
| Q-S6 (inchangée, précisée) | **Durées** (§18 du 34) **et leur effet dans les sauvegardes** (jusqu'à 12 mois, D-043) | DPO |
| Q-S30 | Jalon N-1 pour la bascule des voyages ; à défaut, repli papier pour les séjours repris (**changement d'usage**) | calendrier et usage des personnels |
| Q-S31 | Export santé : qui est le **destinataire** d'une archive santé au départ d'un établissement (son infirmière, son nouvel hébergeur HDS) ; à écrire au contrat | contrat |
| Q-S33 | Mode « sans infirmière » : un référent PAI non soignant est-il acceptable pour le DPO de l'établissement ? | DPO |
| Q-S34 | **Module santé de Next.js** : est-il utilisé aujourd'hui, par qui, avec quelles données ? Faut-il le restreindre à l'infirmerie ou l'arrêter ? Export pour l'infirmière puis destruction à l'arrêt | Next.js intouchable ; production |

---

## 8 bis. Réglages signalés (D-023)

| N° | Réglage | Défaut |
|---|---|---|
| S-460 | Information de la famille à chaque passage, décidée par l'infirmière pour un élève et une période | désactivé |
| S-461 | Seuil d'alerte de l'accès d'urgence de la vie scolaire | 3 fiches par personne et par jour |
| S-462 | Délai de verrouillage automatique de la tablette de l'infirmerie ou de la cuisine | 2 min |
| S-463 | Mode « sans infirmière » | désactivé ; proposé quand le cercle est vide |
| S-464 | Statut neutre « sortie autorisée » visible des professeurs | oui |
| S-465 | Fiche PPMS par zone imprimée à la rentrée | oui |
| S-466 | Croisement menu × consignes pour le chef de cuisine | désactivé tant que le socle 40 ne publie pas les allergènes |

---

## 9. Faits, hypothèses, contradictions

**Faits [F]** (lecture seule) : module santé de Next.js — tables `infirmerie_passage`, `infirmerie_fiche`, `sante_medicament_prise`, `sante_accident`, `sante_pai`, `sante_inaptitude_eps`, `sante_extrait` (`drizzle/0052`, `0054`–`0056`), textes en clair, `ON DELETE CASCADE` vers l'élève, rôles `intranet-modules.ts:922-931`, garde unique `requireModule`, extraits voyage lisibles de tout utilisateur du détail d'un voyage, aucun journal des accès dans `sante-*-db.ts` ; existant Rust — pas de `JSONB` dans 040–216, `TEXT[]` en 211, `BYTEA` en 053, FK non composite en cascade en 210.

**Hypothèses [H]** : usage réel du module santé de Next.js (inconnu) ; organisation des établissements sans infirmière ; pratique des fêtes de classe, des cartes de sortie, du deuxième stylo d'adrénaline ; chronomètre de crise utile au terrain.

**Contradictions relevées** (écrites au §28 du 34) :
1. §6.1 du 34 (N-0 « ne contient jamais » de donnée de santé) contre le même §6.1 (l'appel garde « envoyé à l'infirmerie ») : tranché par R-S35.
2. §13 du 34 (l'IA lit les consignes N-1) contre la consigne de Florian (jamais de donnée de santé) et contre le §5.3 du 34 (les consignes sont des données de santé) : tranché par R-S48.
3. §10.10 du 34 (acte confidentiel en N-2 seulement) contre le parcours du pilote (N-1 pendant l'année de doublon) : tranché par R-S50.
4. `PORTABILITE_EXPORTS.md` §4 (approbation « direction et responsable santé ») contre R-S1 du 34 (la direction ne lit pas la santé) : compatible si la direction approuve sur une vue sans contenu (R-S45) ; à préciser au §4 de `PORTABILITE_EXPORTS.md`.
5. D-040 (deuxième destination de sauvegarde chez OVHcloud) contre R1111-9 6° pour le cabinet : la destination doit être certifiée HDS pour N-2 (§5).
6. La consigne de cette passe renvoie au « §17 de `PORTABILITE_EXPORTS.md` » pour la procédure santé ; ce document n'a que 12 sections, la procédure particulière est au **§4** (et le §17 de `PRA_SAUVEGARDES.md` traite des alternatives rejetées) : renvoi corrigé ici.
