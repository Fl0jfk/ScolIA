# Passe « personas » sur le socle 28 — Mise en service, reprise de données et support

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; recommandations appliquées par défaut (D-031), sauf ce qui engage l'établissement ou l'éditeur à l'extérieur (§7.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un service de production ; aucune donnée réelle.
**Branche :** `ent-socle-27-28-conception`, partie de `origin/ent-mc0hoe`.
**Éprouve :** `docs/socles/28_MISE_EN_SERVICE.md` (lu en entier), avec `CLAUDE.md`, D-009, D-013, D-018, D-031, **D-034**, **D-035**, D-036, D-037 à D-046, `TRANSITION_NEXT_RUST.md`, `TRANSITION_T0_INVENTAIRE.md`, `MISE_EN_SERVICE_DONNEES.md`, `A_FOURNIR_PAR_FLORIAN.md`, `PRA_SAUVEGARDES.md`, et `27_RGPD_ARCHIVES.md` (portes de conformité, journal des accès) ; forme reprise de `PERSONAS_42.md`.
**Ajouts :** faits **directement** dans `28_MISE_EN_SERVICE.md` au **§21** (« ajout personas »), qui l'emporte en cas de contradiction ; rien n'est supprimé.
**Numérotation :** manques **M-2800 à M-2832** ; actions **MSE-280 à MSE-299** ; règles **R-MSE37 à R-MSE45** ; cas limites **CL28-57 à CL28-66** ; questions **Q-MSE11 à Q-MSE13** ; réglages **S-574, S-575**.

---

## 0. À lire en premier : cinq bloquants et leur recommandation (appliquée par défaut, D-031)

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut) | Lot |
|---|---|---|---|---|
| **A** | **La garde « source locale » se contourne** : R-MSE10 refuse une adresse de base non locale. Un tunnel (`ssh -L 5432:prod:5432`) rend la base de production « locale » (P08). | La règle n° 1 de `CLAUDE.md` (aucun accès à la base de production) ne tiendrait qu'à la bonne foi de l'opérateur. | **`ent-reprise` ne reçoit jamais d'adresse de base** : il reçoit un **fichier d'export** (`pg_dump`, archive de fichiers, exports Charlemagne), vérifie son empreinte, et le **restaure lui-même** dans une base éphémère qu'il crée et détruit (conteneur ou grappe temporaire locale, sans réseau sortant) ; aucune option ne permet de lui passer une URL (R-MSE37). | MS4 a |
| **B** | **Le second accord d'une application en production tombe la nuit** (P01) : R-MSE11 demande l'opérateur et un responsable de l'établissement au moment d'appliquer ; la bascule se fait un vendredi à 23 h. | Soit on attend le lundi (la bascule déborde sur la rentrée), soit on contourne. | **Pré-approbation d'une simulation** : le responsable approuve, dans la journée, une **simulation numérotée** (rapport relu) ; l'application de la nuit n'est permise que si elle porte sur **le même export** (même empreinte) **et** produit les mêmes comptages à une marge près (S-574) ; sinon, nouvel accord (R-MSE38). | MS4 |
| **C** | **La zone de reprise garde des IBAN et des données sensibles en clair** : colonnes texte brutes (§13), alors que T0 R-12 demande de chiffrer les IBAN « dès le chargement ». | Une zone de reprise lisible est une copie en clair de toute la facturation (P14). | Chaque adaptateur **déclare** ses colonnes sensibles (IBAN, BIC, RUM, adresses protégées, motifs libres) ; elles sont **chiffrées au chargement** (`ENT_SECRETS_KEY`, format avec identifiant de clé, E11) ou **non chargées** si le socle cible n'en a pas besoin ; test : aucune colonne déclarée sensible n'est lisible en clair dans la zone (R-MSE39). | MS4 c |
| **D** | **Le support ne peut pas reproduire un défaut qui n'apparaît que pour un compte** (P07) : sans usurpation (R-MSE16), comment voir « ce que voit Mme Garnier » quand son accueil plante ? | Sans réponse, l'équipe sera tentée d'ajouter un « se connecter en tant que ». | **« Vue telle que la voit X »** : rendu calculé avec les **droits** de X, sous l'**identité** du support, lecture seule, données masquées par défaut, exigeant une autorisation qui **nomme X**, et **information de X** (avis à X : « le support a affiché votre page d'accueil le … pour corriger un problème ») ; jamais d'action au nom de X (R-MSE40). | MS8 |
| **E** | **L'année de doublon épuise les professeurs** (P04) : appel et notes en double toute l'année, pour 80 professeurs, c'est l'échec assuré du doublon et du projet. | D-034 vise à « repérer manques, erreurs et améliorations » ; une double saisie générale n'est pas nécessaire pour cela. | **Doublon allégé** : la direction choisit un **périmètre de doublon** par domaine (ex. appel : 4 classes pilotes et toute la vie scolaire ; notes : 3 classes ; le reste importé de Charlemagne par synchronisation) ; le comparateur ne compare que ce périmètre ; les autres professeurs ne saisissent que dans Charlemagne (R-MSE41, MSE-280). Q-MSE4 reste ouverte pour aller plus loin. | MS7 |

---

## 1. Méthode
Seize personas (établissement pilote privé sous contrat et futurs clients publics), dont un **attaquant** (opérateur malveillant et faux support) et une **auditrice RGPD** (DPO). Pour chacune : situation, irritants, peurs, ce qui la ravirait, manques (B bloquant, I important, C confort), contradictions, cas limites. Faits de code revérifiés sur `ent-mc0hoe` le 2026-11-11.

## 2. Les personas en un tableau
| N° | Persona | Contexte | Ce qu'elle éprouve |
|---|---|---|---|
| P01 | **Florian**, opérateur, vendredi 16 avril 2027 à 23 h | bascule des voyages et des salles | gel, reprise, second accord, retour arrière |
| P02 | **Mme Martin**, secrétaire | lycée pilote | 340 cas « à décider » |
| P03 | **Mme Duval**, cheffe d'établissement | lycée pilote | portes, formation avant la rentrée |
| P04 | **M. Haddad**, professeur de mathématiques | lycée pilote, doublon | double saisie |
| P05 | **Mme Garnier**, mère séparée | lycée pilote | activation du compte, méfiance |
| P06 | **Mme Lopez**, grand-mère tutrice, sans téléphone récent | lycée pilote | passkey impossible |
| P07 | **Thomas**, futur agent du support ScolIA | plateforme | défaut propre à un compte |
| P08 | **Les attaquants** : **« Max »**, opérateur malveillant ; **faux support** au téléphone | plateforme, lycée pilote | tunnel vers la production, ingénierie sociale |
| P09 | **Mme Aubert**, DPO (auditrice RGPD) | OGEC | répétition réelle, rapports, support |
| P10 | **M. Perrin**, responsable informatique de l'OGEC | lycée pilote | exports Charlemagne « maison » |
| P11 | **M. Arnaud**, principal d'un collège public venant de Pronote | futur client 2028 | bascule en cours d'année, historique |
| P12 | **Mme Roche**, référente numérique | lycée pilote | formation, notes de version quotidiennes |
| P13 | **Inès**, 12 ans, 5ᵉ (futur collège client) | collège | compte élève sans adresse électronique |
| P14 | **Mme Lambert**, comptable de l'OGEC | lycée pilote | payeurs, soldes, IBAN |
| P15 | **M. Benali**, CPE, lundi 6 septembre 2027 à 8 h 10 | lycée pilote | panne des avis d'absence |
| P16 | **Une prospecte**, directrice d'un lycée voisin | démonstration | démo, données, confiance |

---

## 3. Parcours et manques, persona par persona

### P01 — Florian, vendredi 16 avril 2027, 23 h
**Situation.** Gel des voyages et des salles dans Next.js à 18 h ; exports faits ; deux reprises à appliquer avant dimanche ; Mme Duval est partie en vacances à 19 h.
**Ce qui marche.** Tableau de bascule ; propriété unique ; simulation ; rapport.
**Manques.**
- **M-2800 (B)** → **bloquant B** (pré-approbation).
- **M-2801 (I)** : **ordre** de bascule entre modules qui partagent des données (salles réservées pour un voyage) : le tableau déclare les **dépendances** entre bascules (salles avant voyages) et refuse l'ouverture d'un module dont une dépendance n'est pas ouverte (MSE-281).
- **M-2802 (I)** : **runbook de bascule** imprimable, généré par module (étapes, gestes de Florian côté Next.js, critères de retour arrière, contacts), coché au fil de la soirée (MSE-282).
- **M-2803 (C)** : chronomètre par étape comparé aux répétitions ; alerte si une étape dépasse 150 % de sa durée répétée.
**Cas limite.** CL28-57 : l'export final est plus gros que prévu (un organisateur a travaillé dans Next.js après le gel, l'écran n'était pas vraiment en lecture seule) → la reprise différentielle détecte des objets modifiés après l'heure de gel : arrêt, liste, décision (R-MSE42).

### P02 — Mme Martin, 340 cas « à décider »
**Situation.** Première répétition réelle (T2) : 340 responsables à décider (adresses partagées, noms d'usage).
**Irritants.** Un par un, deux fiches côte à côte : six heures de travail.
**Manques.**
- **M-2804 (I)** : **décisions par lot** sur un motif homogène (« même nom, même adresse, même enfant, seule l'adresse électronique diffère ») avec aperçu et annulation (MSE-283) ; jamais de fusion de deux **élèves** par lot (règle du socle 2).
- **M-2805 (I)** : **priorité** : les cas qui bloquent la porte « familles » d'abord (responsables légaux), les autres ensuite.
- **M-2806 (C)** : les décisions d'une répétition anonymisée **ne valent pas** pour l'export réel (les pseudonymes diffèrent) : le dire avant qu'elle y passe du temps ; seules les **règles** (synonymes) se reportent.

### P03 — Mme Duval, cheffe d'établissement
**Situation.** Juin 2027 : elle veut former les professeurs avant la rentrée, mais la porte `ouvert_personnels` exige les données réelles de la nouvelle année, qui ne seront prêtes qu'en août.
**Manques.**
- **M-2807 (I)** : **établissement de formation** : copie **de la structure réelle** (classes, salles, emploi du temps, noms des personnels qui l'acceptent) **avec des élèves et familles fictifs**, dans l'environnement de démonstration ; chacun se forme sur « son » emploi du temps sans aucune donnée d'élève réelle (MSE-284, R-MSE43).
- **M-2808 (I)** : calendrier de mise en service **pré-rempli** à partir du calendrier T0–T4 et des vacances de l'établissement (MSE-285).
- **M-2809 (C)** : bilan d'une page pour le conseil d'administration ou l'OGEC (état des portes, des reprises, des retours).

### P04 — M. Haddad, professeur pendant le doublon
**Situation.** Septembre 2027 : on lui demande l'appel dans Charlemagne (officiel) **et** dans ScolIA.
**Manque.** **M-2810 (B)** → **bloquant E** (doublon allégé).
**Autres manques.**
- **M-2811 (I)** : bandeau du régime (R-MSE8) : ajouter, dans les classes hors périmètre de doublon, « Cette classe n'est pas en essai ; l'appel se fait dans Charlemagne » au lieu d'un écran vide.
- **M-2812 (C)** : quand il est dans le périmètre, un bouton « signaler un écart » directement sur l'appel.

### P05 — Mme Garnier, mère séparée
**Situation.** Elle reçoit un courriel d'activation ; elle pense à un hameçonnage ; l'ancienne adresse du couple reçoit aussi un courriel pour l'autre enfant.
**Manques.**
- **M-2813 (I)** : **lettre d'activation imprimée nominative** (code et QR) remise par l'établissement (cartable, guichet, réunion de rentrée) comme alternative au courriel ; le code seul ne suffit pas : il faut aussi un code envoyé à une coordonnée **vérifiée** du dossier (MSE-286, R-MSE44).
- **M-2814 (I)** : une adresse électronique **partagée** par deux titulaires ne reçoit jamais deux invitations ; elle passe en « à décider » (T0 §6.6) et le secrétariat demande à chaque parent son adresse.
- **M-2815 (C)** : l'invitation nomme **les enfants** concernés et l'établissement, rien d'autre.

### P06 — Mme Lopez, grand-mère tutrice, sans appareil compatible
**Situation.** Téléphone ancien, pas d'ordinateur.
**Constat.** D-009 vise une passkey pour tous **à terme** ; le parcours de secours (D-018) suppose une adresse électronique.
**Manques.**
- **M-2816 (I)** : **clé de sécurité physique** (FIDO2) prêtée ou fournie par l'établissement comme passkey ; enrôlement au guichet (MSE-287).
- **M-2817 (I)** : tant qu'aucun appareil n'est possible, **compte limité** à la lecture (pas d'action sensible) avec code par SMS, décision tracée du secrétariat ; l'information due (absences, R131-5) part de toute façon par SMS ou courrier (socle 8) [R, Q-MSE11].

### P07 — Thomas, support ScolIA
**Situation.** « La page d'accueil de Mme Garnier affiche une erreur » ; elle seule.
**Manque.** **M-2818 (B)** → **bloquant D** (« vue telle que la voit X »).
**Autres manques.**
- **M-2819 (I)** : journaux techniques **corrélés** à la demande de support (identifiant de requête affiché à l'utilisateur dans le message d'erreur, « référence : 7F3A ») pour diagnostiquer **sans** accès aux données (MSE-288).
- **M-2820 (C)** : réponses types par catégorie (MSE-263 existe ; à alimenter par la base d'aide).

### P08 — Les attaquants
**« Max », opérateur malveillant.** (1) Il ouvre un tunnel vers la base de production et lance `ent-reprise --source postgres://localhost:5432` → **bloquant A** (M-2821, B). (2) Il garde une copie de l'annexe nominative d'un rapport → l'annexe est chiffrée, accessible par lien court tracé, détruite avec les copies (S-572) ; un opérateur seul ne peut pas l'ouvrir sans un responsable de l'établissement (M-2822, I, R-MSE45). (3) Il active un « bris de glace » pour lire des données → le bris de glace ne donne accès qu'aux journaux techniques et aux files, jamais aux tables métier (R-MSE17) ; il est notifié à la direction : vérifié.
**Faux support au téléphone.** « Bonjour, ici le support ScolIA, j'ai besoin que vous m'autorisiez l'accès, je vous envoie un lien. » → l'autorisation ne se donne **que** dans l'ENT, sur une demande **visible** dans la console de l'établissement, jamais par un lien reçu ; l'aide et chaque autorisation le rappellent ; le support ne demande jamais de code (M-2823, I).
**Cas limite.** CL28-58 : une demande d'autorisation arrive d'un compte de support **inconnu** de la liste publiée (MSE-264) → refus automatique, alerte à l'éditeur.

### P09 — Mme Aubert, DPO (auditrice RGPD)
**Situation.** Elle audite le 28 avant de donner son avis sur la porte `donnees_reelles`.
**Ce qu'elle vérifie.**
1. **Répétition réelle** en pré-production fermée : elle exige que l'AIPD couvre la **reprise** comme traitement (finalité : migration ; durée : jusqu'à la bascule + S-572) (M-2824, I) ; que la pré-production soit **en France**, chez l'hébergeur de production, avec les mêmes mesures : prévu (R-MSE3).
2. **Rapports** : le résumé est sans données, l'annexe chiffrée ; elle demande que l'**annexe** soit détruite avec les copies de la source et que seul le **résumé** soit gardé comme preuve (catégorie `mes.reprise`) (M-2825, I).
3. **Support** : liste des personnes publiée ; elle demande la **localisation** du support : support **depuis l'Union européenne** seulement, écrit au contrat (M-2826, I) ; et que le rapport mensuel du support lui soit accessible (lecture, sans données).
4. **Démonstration** : jamais de données réelles : vérifié (R-MSE6) ; l'**établissement de formation** (P03) ne reprend que des noms de personnels **consentants** : à écrire (R-MSE43).
5. **Comptes non activés** : désactivés après 60 jours (S-569) ; la personne créée sans compte actif reste au dossier (c'est une donnée de l'établissement, pas un compte).

### P10 — M. Perrin, responsable informatique de l'OGEC
**Situation.** Il exporte de Charlemagne par le requêteur : des CSV dont les colonnes dépendent de la requête qu'il a construite.
**Manque.** **M-2827 (I)** : un adaptateur Charlemagne à **format fixe** ne suffira pas : **adaptateur configurable** avec **correspondance de colonnes** enregistrée par établissement (colonne source → champ attendu), aperçu des 20 premières lignes, validation, versionnée ; les formats officiels (SIECLE) restent fixes (MSE-289).
**Cas limite.** CL28-59 : une colonne change de nom entre deux exports → inspection : arrêt et proposition de remise en correspondance.

### P11 — M. Arnaud, collège public venant de Pronote (2028)
**Situation.** Il veut basculer à la Toussaint, pas en été, et se demande ce qu'il faut reprendre de l'historique.
**Manques.**
- **M-2828 (I)** : **périmètre historique** choisi : année en cours complète (notes du premier trimestre, absences), année précédente pour les bulletins ; années antérieures en **documents** (bulletins en PDF, au coffre du socle 7) plutôt qu'en données (MSE-290).
- **M-2829 (I)** : bascule **en cours d'année** : la période en cours est arrêtée dans l'ancien outil à une date, reprise, puis ouverte dans ScolIA ; les moyennes de la période se calculent sur les deux sources (notes reprises marquées « saisies dans l'ancien outil ») — à porter au socle 21.

### P12 — Mme Roche, référente numérique
**Situation.** Pendant le doublon, une version par jour ; les notes de version l'inondent.
**Manques.**
- **M-2830 (C)** : **résumé hebdomadaire** des nouveautés par rôle (tuile et courriel au référent), le détail restant consultable (S-575).
- **M-2831 (C)** : kit de formation imprimable par rôle (MSE-246 existe) **mis à jour automatiquement** à chaque version qui change un écran.

### P13 — Inès, 12 ans
**Situation.** Pas d'adresse électronique ; un téléphone partagé à la maison.
**Ce qui manque.** Les invitations supposent une adresse.
**Ajout.** **Séance d'activation en classe** : le professeur principal ouvre une séance (code de séance affiché, 20 minutes), chaque élève active son compte avec son code personnel imprimé et enrôle une passkey sur l'appareil qu'il utilisera, ou reçoit un compte limité en attendant (MSE-291) ; rattaché au M-2816.

### P14 — Mme Lambert, comptable de l'OGEC
**Situation.** Reprise des payeurs et soldes de Charlemagne ; finances en « mode suivi » pendant le doublon (D-034).
**Manques.**
- **M-2832 (B)** → **bloquant C** (zone chiffrée).
- Totaux au centime par foyer et par payeur : prévus ; elle demande que le rapport donne **le total général** des soldes ouverts des deux côtés, à signer par elle (MSE-292).
**Cas limite.** CL28-60 : un mandat SEPA repris de Charlemagne : **jamais réactivé** pendant le doublon (mode suivi, pas de prélèvement) ; la RUM et la date de signature sont gardées pour le jour de la bascule des finances.

### P15 — M. Benali, CPE, lundi 6 septembre 2027 à 8 h 10
**Situation.** Les avis d'absence ne partent pas (fournisseur de SMS en panne).
**Ce qui marche.** Incident immédiat (R-MSE7) ; plan de secours.
**Ajouts.**
- **Mode dégradé** : la liste des absents à prévenir est affichée, imprimable, avec les numéros (accès CPE, tracé), et un bouton « prévenu par téléphone » par famille ; à la reprise du service, les avis déjà faits par téléphone ne repartent pas (MSE-293, R-MSE9 précisée).
- Pendant le doublon, si l'avis officiel vient de Charlemagne, la panne de ScolIA **n'est pas** une panne d'avis : le bandeau du régime le dit.

### P16 — Une prospecte, directrice d'un lycée voisin
**Situation.** Elle teste la démo un soir ; elle veut savoir si « ses » données seraient en sécurité et combien de temps prend la reprise.
**Ajouts.**
- La démo montre **un rapport de reprise fictif** complet (comptages, totaux, échantillons) : c'est l'argument de confiance (MSE-294).
- Le **dossier de conformité de la plateforme** (27 §8.8) est accessible depuis la démo.
- Sa démo personnelle (MSE-227) expire et se détruit (S-570) ; aucune donnée qu'elle saisit n'est gardée.

### Manques transverses : mobile, IA, tests
- Mobile : la séance d'activation en classe (P13) et l'enrôlement de passkey sont des écrans mobiles clés ; la révocation d'une autorisation de support aussi.
- IA : les synonymes proposés (classes, motifs) sont **mesurés** sur les répétitions (taux d'acceptation) ; jamais de proposition de rapprochement de personnes.
- Tests : ajouter les bloquants A à E au §17 du 28 (aucune option d'URL ; pré-approbation liée à l'empreinte ; colonnes sensibles chiffrées ; vue telle que la voit X sans action ; périmètre de doublon).

---

## 4. Vérification réglementaire complémentaire
Voir `28_MISE_EN_SERVICE.md` §4 et `27_RGPD_ARCHIVES.md` §4.10 : CNIL (données réelles en test interdites, fiche 11 du guide du développeur, [V]) fonde l'établissement de formation **fictif** (P03) et la pré-production fermée ; SDET 2024 (journalisation des personnels techniques et de la prise en main à distance, [V]) fonde la « vue telle que la voit X » tracée et l'information de X ; RGPD 28.3 b (confidentialité) fonde l'engagement des personnes du support. Non vérifié : délai de contestation d'un prélèvement SEPA (P14, renvoyé à l'expert-comptable), obligations d'accessibilité d'un éditeur privé (P06, P13).

---

## 5. Manques qui relèvent d'autres socles
| Manque | Socle | Action |
|---|---|---|
| Clé FIDO2 et compte limité (M-2816, M-2817) | 1 | parcours d'enrôlement au guichet ; compte limité sans action sensible |
| Bascule en cours de période (M-2829) | 21, 23 | notes « saisies dans l'ancien outil » |
| Mode dégradé de l'avis d'absence | 12, 8 | liste imprimable, « prévenu par téléphone » |
| Identifiant de requête affiché (M-2819) | transverse (`ent-api`) | erreurs avec référence |
| Lettre d'activation imprimée (M-2813) | 7 | modèle de document |

---

## 6. Synthèse

### 6.1 Manques par gravité
- **Bloquants (5)** : M-2800, M-2810, M-2818, M-2821, M-2832.
- **Importants (20)** : M-2801, M-2802, M-2804, M-2805, M-2807, M-2808, M-2811, M-2813, M-2814, M-2816, M-2817, M-2819, M-2822, M-2823, M-2824, M-2825, M-2826, M-2827, M-2828, M-2829.
- **Confort (8)** : M-2803, M-2806, M-2809, M-2812, M-2815, M-2820, M-2830, M-2831.

### 6.2 Ajouts dans la conception (`28_MISE_EN_SERVICE.md` §21)
Actions MSE-280 à MSE-294 ; règles R-MSE37 à R-MSE45 ; cas limites CL28-57 à CL28-60 ; tables `rep_preapprobation`, `rep_bascule_dependance`, `rep_doublon_perimetre`, `rep_correspondance_colonne`, `sup_vue_sujet_avis`, `mes_seance_activation` ; migration 2577 attribuée ; S-574, S-575.

### 6.3 Les dix ajouts les plus importants
1. `ent-reprise` ne reçoit qu'un fichier, jamais une adresse (A).
2. Pré-approbation d'une simulation liée à l'empreinte de l'export (B).
3. Colonnes sensibles chiffrées en zone de reprise (C).
4. « Vue telle que la voit X », sans usurpation, avec information de X (D).
5. Doublon allégé par périmètre (E).
6. Établissement de formation : structure réelle, personnes fictives.
7. Adaptateur Charlemagne configurable (correspondance de colonnes).
8. Lettre d'activation imprimée et séance d'activation en classe.
9. Dépendances et runbook de bascule.
10. Support depuis l'Union européenne, annexe nominative détruite avec les copies.

### 6.4 Ce que la passe a montré sur la conception
La conception était juste sur les principes (simulation, preuve, rejouabilité) mais trop confiante dans les **gardes déclaratives** (une adresse « locale », un second accord « au moment d'appliquer ») et dans la **disponibilité des gens** (une direction joignable à 23 h, des familles qui ont toutes une adresse électronique, des professeurs prêts à tout saisir deux fois). Les ajouts rendent les gardes **structurelles** et adaptent le parcours à la vie réelle de l'établissement.

---

## 7. Décisions

### 7.1 Appliquées par défaut (D-031)
Les cinq bloquants A–E ; M-2804 (décisions par lot, jamais pour des élèves) ; M-2807 (établissement de formation fictif) ; M-2813 (lettre d'activation) ; M-2816 (clé FIDO2) ; M-2825 (annexe détruite avec les copies) ; M-2826 (support depuis l'UE) ; M-2827 (adaptateur configurable) ; M-2828 (périmètre historique : année en cours + précédente, antérieures en documents).

### 7.2 Soumis à Florian
| N° | Question | Recommandation |
|---|---|---|
| Q-MSE11 | Compte limité par SMS pour les familles sans appareil compatible (M-2817) : exception à D-009 ? | oui, lecture seule, décision tracée, revue annuelle |
| Q-MSE12 | Clés FIDO2 fournies par l'établissement (coût : quelques dizaines d'euros par clé) | stock de 10 clés pour l'établissement pilote |
| Q-MSE13 | Périmètre du doublon allégé pour 2027-2028 (classes pilotes, domaines) | à choisir avec la direction au printemps 2027 ; défaut : 4 classes pour l'appel, 3 pour les notes, toute la vie scolaire |

### 7.3 Réglages signalés
S-574 : marge de comptages entre simulation pré-approuvée et application (défaut : 0 sur les personnes, 1 % sur les autres objets). S-575 : rythme du résumé des nouveautés (défaut : hebdomadaire).

---

## 8. Faits, hypothèses, contradictions
**Faits [F]** : la garde existante des scripts PRA (`pra-lib.sh`) repose sur l'adresse ; `ent_import_ligne.donnees` est en JSONB ; `/labo/bootstrap` est exclu du binaire de production par la CI. **Hypothèses [H]** : exports Charlemagne par requêteur (à confirmer par Florian sur un export anonymisé, F-02) ; disponibilité de la direction ; part des familles sans appareil compatible. **Contradictions** : R-MSE10 (garde par adresse) remplacée par R-MSE37 ; R-MSE11 précisée par R-MSE38 ; §13 (zone en texte brut) corrigé par R-MSE39 ; le « doublon » de D-034 lu comme double saisie générale, corrigé par R-MSE41 (à confirmer par Florian, Q-MSE13).
