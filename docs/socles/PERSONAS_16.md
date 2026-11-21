# Passe « personas » sur le socle 16 — Messagerie et conversations (et l'esquisse du socle 17)

**Date :** 2026-11-10 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur ou contredit une décision (§10.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production. Le code Next.js a été **lu seulement**, sur une copie de `origin/main` extraite par `git archive` dans le dossier temporaire de la session (rien exécuté, rien construit, rien modifié).
**Branche :** `ent-socle-messagerie-personas`, partie de `origin/ent-mc0hoe` (`c9351f1`, qui contient `ENT` avec le lot M0a et la migration 480).
**Éprouve :** `docs/socles/16_MESSAGERIE.md` (lu en entier), avec `CLAUDE.md`, `docs/DECISIONS.md` D-031, **D-034**, **D-035**, **D-036** ; le code réellement livré par le lot **M0a** (`ent-rust/migrations/480_msg_m0a_fuites.sql`, `crates/ent-db/src/msg_repo.rs`, `service.rs`, `tests_messagerie_m0a.rs`) ; `docs/TRANSITION_T0_INVENTAIRE.md` (§3.0 maturité, P06, P07) ; `34_SANTE.md` §10.15 (messagerie de l'infirmerie) ; `08_NOTIFICATIONS.md` §14 (urgence) ; forme reprise de `PERSONAS_10.md`, `PERSONAS_21.md`, `PERSONAS_42.md`.
**Ajouts :** faits **directement** dans `16_MESSAGERIE.md`, marqués **« ajout personas »** (nouveau §33, plus des corrections ponctuelles en tête, aux §5, §25, §27, §30, §31). **Rien n'est supprimé.**
**Numérotation :** manques **M-2301 à M-2412** (plage libre : le plus haut numéro pris sur les branches est M-2299) ; existant Next.js **NX-87 à NX-104** ; défauts Next.js **NXD-1 à NXD-17** ; défauts résiduels du Rust après M0a **D21 à D28** (suite du §2.2) ; actions **MSG-320 à MSG-359**, **ANN-117 à ANN-121** ; règles **R-M48 à R-M66** ; cas limites **CM-81 à CM-100** ; questions **Q-M23 à Q-M30** ; réglages **S-450 à S-456**.

---

## 0. À lire en premier : cinq bloquants et leur recommandation (appliquée par défaut, D-031)

La conception est solide sur le fond (lire = participer, un fil par foyer, boîtes de service, plage calme, gel, IA en brouillons). La confronter à dix-huit personas **et au code réellement livré par M0a** montre que **M0a a fermé les quatre fuites du §2.2, mais pas toutes les fuites** : la règle provisoire (« lecture déduite de l'élève, filtrée par rôle ») laisse passer trois cas graves que les personas rencontrent dès la première semaine. Ils doivent être fermés **avant la démo du début décembre** (D-034), qui montre la messagerie avec les familles séparées du labo.

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut, D-031) | Lots |
|---|---|---|---|---|
| **A** | **Le fil qu'un élève ouvre est lu par tous ses professeurs.** Fait [F] : `msg_repo.rs:168` rend lisible du personnel tout fil `audience = 'eleve'`, sans condition de rôle ; il suffit d'avoir `messagerie.eleve` sur l'élève (tout professeur de ses cours, `actions.rs`, portée D-022). M0a l'a assumé (§27.3, choix 2). | Chloé (P11) écrit à la CPE « je suis harcelée » à 22 h : ses neuf professeurs peuvent lire le fil. Si l'adulte mis en cause est un professeur, il lit le signalement qui le vise. C'est exactement la fuite D3, déplacée des parents vers les adultes. | **D21** : un fil ouvert par un élève suit la règle du fil privé d'un parent (lisible de direction, CPE, administratif, **et du personnel qui y a écrit**), jamais des professeurs par le seul lien d'enseignement. Le professeur que l'élève veut joindre est **ajouté** au fil par la CPE, ou l'élève lui écrit depuis un fil ouvert par ce professeur. | **M0b** (migration 481) |
| **B** | **Le fil qu'un personnel ouvre « aux responsables » réunit les deux foyers.** Fait [F] : un fil ouvert par l'établissement reçoit `PorteeFil::responsables()` (`service.rs`, `ouvrir_fil_for_actor`) sans `foyer_id` ; `fils_lisibles_famille` le rend lisible de **tout lien communicant** de l'élève, quel que soit le foyer, et `notify_parents_push_fil` pousse l'avis à tous. Le §27.3 ne le mentionne pas : seul le fil ouvert **par** un parent est séparé (RP1-11). Il est aussi lu de **tout professeur** de l'élève (la règle D2 ne couvre que les fils privés et de relance). | La CPE écrit « aux parents de Léo » ; Mme Lopez (P07, adresse protégée par ordonnance) répond « nous avons déménagé près de … » ; M. Lopez reçoit le push et lit la réponse. Le professeur d'allemand la lit aussi. Le cas CM-02 n'est donc pas couvert en décembre. | **D22** : provisoire avant les participants (M1) : un fil ouvert par un personnel vers les responsables est **créé une fois par foyer** de liens communicants (nouvelle colonne `foyer_lecteur_id`, distincte de `foyer_id` qui signifie « payeurs ») ; il n'est lisible du personnel que par son auteur, ceux qui y ont écrit, et direction, CPE, administratif. **D23** : pour un élève **majeur**, le fil ouvert par le personnel lui est adressé (audience `eleve`), les parents n'y entrent que selon les droits conservés du socle 4. | **M0b** |
| **C** | **Rien ne respecte la plage calme, et la messagerie sonne pendant un confinement.** Fait [F] : la messagerie n'a aucun différé ; le push part dans la requête (`notify_parents_push_fil`, aucune heure calme). Le texte national (circulaire « numérique raisonné » de juillet 2025, cadre de référence de 2026, §7 R-1) demande un réglage **par défaut** qui retient la remise de 20 h à 7 h et le week-end. Pendant une **alerte intrusion** (PPMS), une notification sonore sur le téléphone d'un adulte confiné ou d'un élève peut révéler une présence (P04). | Mme Kessler (P01) écrit le dimanche à 21 h, les familles sont réveillées ; le jour de l'alerte, 600 parents écrivent « mon enfant va bien ? » et les téléphones des personnels confinés vibrent. | **R-M20 minimal en M0b** : colonne `remise_prevue_le` sur `ent_msg`, remise et avis différés à 7 h (fuseau de l'établissement), l'auteur le voit avant d'envoyer, dérogation de la direction tracée. **R-M52** : pendant une alerte U3 active du socle 8 (`surete.alerte`), la messagerie **n'émet aucun avis** (ni push, ni e-mail, ni SMS) ; les familles qui écrivent voient le bandeau de l'alerte et une réponse automatique ; leurs fils vont à une boîte « cellule de crise ». | M0b (différé) ; M4 (complet) ; M3 + socle 8 N3 (crise) |
| **D** | **La conception viole D-036 à six endroits.** `ent_msg_boite.horaires JSONB`, `astreinte JSONB`, `ent_msg_parametre (cle, valeur)` (clé-valeur = JSON aplati, selon le vocabulaire de T0), `mission_codes TEXT[]`, `etiquettes TEXT[]`, `criteres TEXT[]`, et l'« audience décrite JSON » du socle 17. Côté Next.js, `famille_messaging_settings.roles_can_initiate` est un JSONB, les « channels » vivent dans deux documents JSON (`channels/channels.json`, `chat/messages.json`), les pièces famille ont un repli `content_base64`. | Un réglage ou une audience en JSON n'a ni contrainte, ni clé étrangère, ni historique ; c'est ce que D-036 interdit. Aucune migration n'en a encore été écrite : c'est le moment de corriger. | **Normalisation** du §25 (§33.6 de la conception) : `ent_msg_boite_horaire`, `ent_msg_astreinte`, `ent_msg_boite_mission`, `ent_msg_etiquette` et `ent_msg_traitement_etiquette`, `ent_msg_vigilance_critere`, `ent_msg_reglage` (une version = une ligne à colonnes typées), `ent_pub_audience_critere`. Exceptions permises listées à `MIGRATIONS.md` : charge utile de l'outbox, instantanés d'audit, export JSON (format d'échange). | M1 et suivants (avant écriture) |
| **E** | **Les « channels » de Next.js sont ouverts à tous.** Fait [F, NXD-1] : `app/api/channels/**` ne demande qu'une session ; tout utilisateur connecté lit tous les messages de tous les salons, modifie ou supprime n'importe quel salon ou message ; les pièces partent sur une URL **publique** non signée ; un mode « anonyme » efface l'auteur. La messagerie famille a une IDOR sur les pièces (NXD-3) et un périmètre professeur contournable (NXD-4). | Next.js est intouchable (CLAUDE.md) : le pilote ne peut ni corriger ni couper. Mais tant qu'il tourne, ces défauts sont en production. | **Signalés à Florian** (§10.2) avec le détail du §4 : la décision de couper `channels` ou de restreindre son accès appartient à Florian. Côté ScolIA : **rien n'est repris des channels** (ni modèle, ni données : perdable, T0 P05), seulement le besoin (salons de personnels, « Général »), servi par les fils d'équipe et les publications internes. | — (Florian) |

---

## 1. Méthode

**Démarche.** Celle de `PERSONAS_21.md` §1 et `PERSONAS_42.md` §1 : dix-huit personas réalistes, prises dans la consigne et dans le tableau « acteurs » du §3 de la conception, couvrant le **lycée pilote** (privé sous contrat, OGEC) et un **collège public** fictif (EPLE : archives publiques, C. pén. 432-9, LIEN pour l'infirmier), les familles ordinaires et les cas extrêmes, plus un persona de **démo** (P18 : Florian, début décembre 2026). Pour chacune : sa semaine ou son année, ses **irritants**, ses **peurs**, ce qui la **ravirait** ; les **manques** et **contradictions** ; les cas limites.

**Grille transversale** posée à chaque persona : **(1) état réel** — que lui permet le code après M0a, aujourd'hui, et que lit-elle qu'elle ne devrait pas lire ? **(2) parité** — que fait-elle dans Next.js, et le retrouve-t-elle ? **(3) temps** — à quelle heure écrit-elle, reçoit-elle, qui est réveillé ? **(4) D-036** — ce qu'elle règle est-il une colonne ou un JSON caché ? Puis les questions habituelles : qui voit quoi, ce qui change à une date, ce que l'IA prépare sans décider.

**Lecture du code M0a [F].** Migration 480 lue en entier ; `msg_repo.rs` (`fils_lisibles_famille`, `fils_lisibles_personnel`, `destinataires_famille_fil`) et `service.rs` (`actor_can_access_fil`, `role_lit_fils_reserves`, `ouvrir_fil_for_actor`, `notify_parents_push_fil`) lus aux lignes citées ; liste des tests `tests_messagerie_m0a.rs` ; droits par défaut (`ent-auth/src/actions.rs`). Résultat au §3.

**Audit de parité [F].** Un sous-agent a relu en lecture seule les 23 routes de `app/api/messaging/**`, les 4 de `app/api/channels/**`, `app/api/famille/messages/**`, `app/api/vie-scolaire/messages-foyer/**`, les bibliothèques `app/lib/messaging/**`, `famille-messaging-*.ts`, les schémas et les scripts de rétention. Résultat au §4.

**Vérification réglementaire.** Un sous-agent de recherche a contrôlé le §5 de la conception le 2026-11-10 (kit juridique du SDET 2025 et annexe opérationnelle 2024 lus en entier, cnil.fr, Légifrance, ai-act-service-desk ; education.gouv.fr et eduscol renvoient 403). Résultat au §7, versé au §5 de la conception.

**Gravité** (identique aux passes précédentes) : **bloquant** · **important** · **confort** · **différenciant**.

**Conventions.** [V] lu sur une source officielle ; [NV] non lu sur source officielle ; [H] hypothèse ; [R] recommandation ; [F] fait vérifié dans le code (Rust : chemins depuis `ent-rust/` ; Next.js : chemins depuis la racine de `origin/main`).

**Limites.** Les personas sont construites, pas interviewées ; les pratiques de terrain (cahier de liaison AESH, groupes WhatsApp, appels relayés par post-it) sont [H]. Aucune donnée réelle n'a été lue. L'usage réel de la messagerie Next.js par les personnels et les familles (combien de conversations, de fils famille actifs) **n'est connu que de Florian** (Q-M23).

---

## 2. Les personas en un tableau

| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **Mme Kessler**, professeure de français, 4ᵉ C « difficile », 40 messages par semaine, écrit le soir et le dimanche | Collège public (fictif) | Plage calme, débit, contestations, protection, transfert à la vie scolaire |
| P02 | **M. Royer**, professeur principal de 2ᵈᵉ 4 (35 élèves) | Lycée pilote | Lot au-delà de 30, fil d'équipe, bilan de mi-trimestre, compte rendu du conseil |
| P03 | **Mme Benali**, CPE, et **Yanis**, AED | Lycée pilote | Boîte vie scolaire, AED aveugle sous la règle provisoire, absences signalées, astreinte |
| P04 | **Mme Duval**, cheffe d'établissement, et **M. Benyahia**, principal du collège | Lycée pilote, collège public | Crise et PPMS, supervision prévue par la charte, gel, pilotage des délais |
| P05 | **Mme Morel**, secrétaire, et **M. Gomes**, agent d'accueil | Lycée pilote | Relais des appels, courrier papier, mêmes questions cent fois, appelant non identifiable |
| P06 | **Mme Lefèvre**, infirmière (salariée de l'OGEC), et **M. Caron**, infirmier (LIEN) | Lycée pilote, collège public | Point ouvert « messagerie de l'infirmerie », HDS, secret, texte libre |
| P07 | **Mme et M. Lopez**, séparés ; ordonnance de protection : M. Lopez ne doit pas entrer en relation avec Mme Lopez, dont le domicile est caché ; il garde l'autorité parentale | Lycée pilote, Léo en 2ᵈᵉ | Fil commun impossible, réponse de la mère lue par le père, coordonnées dans une réponse |
| P08 | **Mme Kovalenko**, arrivée d'Ukraine en 2025, lit peu le français | Collège public, fils en 5ᵉ (UPE2A) | Langue, traduction, cartes d'action, enfant pris comme interprète |
| P09 | **M. Vidal**, parent aveugle (VoiceOver, NVDA) | Lycée pilote | Accessibilité, circulaires scannées, signature sans geste graphique |
| P10 | **M. Fabre**, parent qui harcèle Mme Kessler (vingt messages par jour) | Collège public | Débit, restriction, preuve, information due maintenue |
| P11 | **Chloé**, 14 ans, harcelée par trois camarades (couloirs et réseau social) | Collège public | Parler discrètement, signaler, ne pas être lue de ses professeurs ni de ses parents, 22 h |
| P12 | **Yasmine**, 18 ans en octobre, terminale ; a choisi que ses parents ne reçoivent plus ses absences | Lycée pilote | Fils qui lui sont adressés, père qui écrit au PP, bascule de majorité |
| P13 | **Mme Diallo**, AESH de Léo (ULIS) et de deux autres élèves, PIAL sur deux établissements | Lycée pilote, collège | Savoir l'absence de l'élève, liaison avec la famille sans être seule, fin de contrat |
| P14 | **M. Okafor**, intervenant théâtre (dix séances), et **Mme Roux**, policière de prévention (une séance) | Lycée pilote | Pas de compte, joindre le référent, ne jamais joindre un élève |
| P15 | **M. Moreau**, parent délégué de la 2ᵈᵉ 4 (liste APEL) | Lycée pilote | Écrire aux parents de la classe sans voir leurs adresses, recevoir leurs questions |
| P16 | **M. Blanc**, délégué à la protection des données (OGEC), et le DPO académique | Lycée pilote, collège | Registre, AIPD, durées, journal de supervision, sous-traitants |
| P17 | **Les attaquants** : élève qui usurpe le compte d'un parent, père interdit qui recrée un compte, professeur curieux, pièce jointe piégée, faux message « de la direction », injection | — | Usurpation, lecture des fils d'autrui, pièce piégée |
| P18 | **Florian**, le lundi de la démo de début décembre 2026 | Labo | Ce qui doit marcher, ce qui ne doit pas se voir, transition depuis Next.js |

---

## 3. État réel du code après M0a (fait de code)

### 3.1 Ce que M0a a réellement changé [F]
- **Migration 480** : vues `ent_eleve_lien_communicant` (lien actif, personne non fusionnée, sans interdiction `communication` en cours) et `ent_eleve_lien_sollicitable` (idem, hors « ne pas solliciter ») ; colonnes `ent_msg_fil.audience` (`responsables` par défaut, `famille`, `eleve`) et `ent_msg_fil.vie_scolaire` ; reprise : fils ouverts par un compte élève passés en `audience = 'eleve'` avec `prive_user_id` ; fils liés au carnet ou à une alerte d'absences marqués `vie_scolaire` ; RLS **permissive** sur 17 tables (sans contexte d'établissement et hors mode strict, tout reste visible : protection contre l'oubli d'un filtre seulement quand le contexte est posé).
- **Lecture famille** (`msg_repo.rs:104-145`) : un parent lit un fil non privé (ou le sien), d'audience `famille` ou `responsables`, s'il a un lien communicant ; un fil `vie_scolaire` exige `peut_voir_vs` **et** l'autorité ; un fil de relance exige d'être payeur du foyer. Un élève lit les fils `famille` ou `eleve` sans `foyer_id`.
- **Lecture personnel** (`msg_repo.rs:152-185`, `service.rs:17460-17467`) : après `messagerie.eleve` et D-022 ; fils privés d'un parent lus par admin, direction, administratif, CPE ; relances par admin, direction, administratif, compta ; dans les deux cas aussi par qui y a écrit ; **tout le reste est lu par tout personnel qui a le droit sur l'élève**.
- **Envois** : push au titre fixe « Nouveau message » ; carnet sans contenu dans e-mail, SMS, push ; sujet retiré des événements ; « ne pas solliciter » respecté par le push, les campagnes, les rencontres.
- **Tests** : 11 tests d'attaque, chacun précédé d'un témoin (`tests_messagerie_m0a.rs`).

### 3.2 Fuites et manques résiduels (suite du §2.2 de la conception)

| # | Gravité | Défaut | Preuve [F] | Scénario | Correctif |
|---|---|---|---|---|---|
| **D21** | **critique** | Fil ouvert par un élève lisible de **tout** personnel ayant `messagerie.eleve` sur lui, professeurs compris | `msg_repo.rs:168` (`OR f.audience = 'eleve'`) ; `actions.rs` (le professeur a `MessagerieEleve` sur ses cours) ; choix 2 du §27.3 | P11 : signalement de harcèlement lu des neuf professeurs | M0b : règle des fils privés (rôles réservés + qui a écrit) |
| **D22** | **critique** | Fil ouvert par un personnel « aux responsables » lisible des **deux foyers** et de **tout** professeur de l'élève ; push aux deux foyers à chaque réponse | `service.rs` `ouvrir_fil_for_actor` (`PorteeFil::responsables()`, pas de foyer) ; `msg_repo.rs:119-131` ; `destinataires_famille_fil` renvoie « tous » pour un fil non privé | P07 : la réponse de la mère (adresse protégée) lue par le père | M0b : un fil par foyer (`foyer_lecteur_id`), lecture du personnel restreinte |
| **D23** | élevée | Élève **majeur** : un fil ouvert par le personnel lui est invisible (audience `responsables`) et va à ses parents, même s'il s'y est opposé | `msg_repo.rs:133` (l'élève ne lit que `famille` / `eleve`) ; aucune lecture de la majorité dans la messagerie | P12 : la CPE écrit au sujet des absences de Yasmine ; ses parents lisent, elle non | M0b : audience `eleve` pour un majeur, parents selon les droits conservés (socle 4) |
| **D24** | élevée | Aucune plage calme, aucune disponibilité des personnels dans la messagerie ; push synchrone à toute heure | `notify_parents_push_fil` (`service.rs:1568-1612`) sans heure calme | P01 : message du dimanche 21 h remis et sonné à 21 h | M0b (différé minimal), M4 |
| **D25** | élevée | Interdiction saisie **par libellé seul** (personne absente de la fiche) : sans aucun effet, même si la personne est liée plus tard | `480:23-38` (`i.personne_id IS NOT NULL`) ; §27.3 choix 4 | P17 b : le père interdit se fait lier au guichet sous une autre adresse | M0b + socle 4 : tout lien créé après une interdiction par libellé sur l'élève est **à vérifier** (aucune participation) jusqu'à décision de la direction |
| **D26** | moyenne | La règle provisoire D2 est **par rôle de base** : un AED (profil `personnel` + rôle D-017) ne lit aucun fil privé, donc ne peut pas traiter les messages des familles | `service.rs:17460-17467` | P03 : Yanis ne voit pas les messages du lundi matin | M0b : action D-017 `messagerie.fils_reserves.lire` (portée établissement) au lieu des rôles, accordée par défaut à CPE, direction, administratif, assignable à un rôle AED |
| **D27** | moyenne | Annonces et campagnes : le **corps** part encore par e-mail ; l'e-mail du carnet part à tout responsable communicant, sans `peut_voir_vs` | §27.3 (D4, « le texte reste dans l'ENT et l'e-mail ») ; `PROJECT_STATUS.md` (« non fait ») | Une annonce sur un deuil arrive en clair dans une boîte e-mail partagée | M0b : enveloppe neutre ; destinataires par nature (socle 8 N0) |
| **D28** | moyenne | Si un rôle `infirmerie` ou `psy` recevait `messagerie.eleve` par un rôle D-017, ses fils (audience `responsables`) seraient lus de tous les professeurs de l'élève | `actions.rs` (aujourd'hui `MessagerieEleve` n'est pas donné à l'infirmerie : rien ne fuit **aujourd'hui**) ; aucune garde | Un administrateur coche « messagerie » pour l'infirmière | M0b : garde serveur, `messagerie.eleve` non attribuable aux profils santé et psychologue avant les boîtes C3 (M3) |

**Ce qui tient [F]** : l'interdiction de communication par personne (D1), le « ne pas solliciter » des envois (D6), la séparation des fils ouverts par un parent (RP1-11), le carnet borné au périmètre (D7), aucun sujet dans les événements ni les titres de push (D4), le cloisonnement entre établissements.

---

## 4. Audit de parité avec Next.js (fait de code, lecture seule)

### 4.1 Maturité [F + avis]
`TRANSITION_T0_INVENTAIRE.md` classe la messagerie interne « Partiel [H] » (39 fichiers, 8 200 lignes, 0 test) ; l'audit corrige : **2 fichiers de test** (`app/lib/messaging/assist.test.ts`, `app/lib/famille-messaging-matrix.test.ts`, 6 cas), **0 TODO**. Avis par sous-module :
- **Messagerie interne des personnels : fiable avec réserves.** Modèle relationnel propre, participation vérifiée sur chaque route (`app/lib/messaging/service.ts:367-388`), pas d'IDOR. Réserves : aucun test du service ni des droits, 2 370 lignes dans un fichier, aucune transaction, édition sans historique, aucune analyse des pièces, salles de visio en mémoire du processus.
- **« Channels » : ébauche dangereuse** (NXD-1, NXD-6, NXD-7). À ne pas reprendre.
- **Messagerie famille : partielle.** Schéma sain, fils par foyer, diffusion ; mais périmètre professeur contournable, pièces non sûres, aucune notification vers l'établissement quand un parent répond, « lu » par côté et non par personne, un parent ne peut pas ouvrir de fil.
- `app/lib/channel-access.ts` **n'est pas de la messagerie** (canal web personnel contre appli famille).

### 4.2 Fonctions sans équivalent dans la conception (à ne pas perdre)

| NX | Fonction Next.js [F] | Reprise [R] |
|---|---|---|
| NX-87 | Conversation directe **unique par paire** de personnels (`normalizePair`, `service.ts:87`) | R-M63 : un seul fil direct par paire de personnels, rouvert plutôt que dupliqué |
| NX-88 | Épingler un fil, le marquer « non lu » **par participant** | MSG-109 étendu (épingle, « non lu ») ; colonnes de `ent_msg_participant` |
| NX-89 | « Supprimer pour moi » : l'historique antérieur est masqué pour soi seul (`hiddenAt`) | **Repris comme archivage personnel** (MSG-109) ; jamais de masquage de l'historique d'un fil famille (preuve) |
| NX-90 | Messages système automatiques (ajout, retrait, départ, renommage, `postSystemMessage`, `service.ts:1168`) | déjà prévus (« a rejoint ») ; renommage d'un fil interne : MSG-322 |
| NX-91 | **Mentions** `@` validées contre les participants, avis même en fil muet | MSG-323 entre personnels seulement ; jamais dans un fil famille ou élève |
| NX-92 | **Épingler un message** visible de tous | MSG-324, fils internes et d'équipe |
| NX-93 | Citation (réponse à un message) et **transfert** avec contrôle des pièces d'origine (`service.ts:1633-1656`) | réponse citée (`reponse_a`) prévue ; transfert : MSG-325, **interdit** d'un fil famille ou élève vers un autre fil (seule la boîte transfère le fil entier, MSG-164) |
| NX-94 | « Vu par » détaillé réservé à l'auteur | **Refusé** sous cette forme (§6.5) : « lu » sans heure, agrégé dans les fils d'équipe |
| NX-95 | Présence (en ligne, occupé, ne pas déranger, absent 1 / 4 / 24 h), « en train d'écrire » | **Refusés** côté familles et élèves (S-137) ; entre personnels : statut « ne pas déranger » repris comme pause (MSG-184) ; présence en ligne et frappe **non reprises** (§6.5) |
| NX-96 | **Recherche** globale dans ses conversations, jokers échappés | MSG-111 (déjà) |
| NX-97 | **Visio** WebRTC en conversation directe et de groupe, un seul appel par conversation | socle 33 (lien vers la salle, §6.1) ; **parité à garantir** si les personnels l'utilisent (Q-M23) |
| NX-98 | **Assistant de rédaction** (réponse, reformulation en 4 tons, résumé « décisions / questions / qui fait quoi »), Mistral, pseudonymisation stricte « échec fermé », rien n'est envoyé par l'assistant | **Repris** dans M13 (MSG-300 à MSG-303) ; garder la pseudonymisation échec fermé et le résumé en trois rubriques ; ajouter le journal IA qui manque |
| NX-99 | Messages **vocaux**, vidéos, visionneuse d'images | vocaux et vidéos : **non repris** pour les familles et les élèves (minimisation, accessibilité, modération) ; entre personnels : Q-M28 |
| NX-100 | Avis Web Push respectant le muet, regroupés par conversation, lien profond | M6 (déjà, sans contenu : NXD-8) |
| NX-101 | Dock flottant hors page ; messagerie personnelle **bloquée pendant une supervision** (`access.ts:43-61`) | écran mobile et web (M14) ; R-M64 : un compte en mode supervision (accès d'un administrateur « en tant que ») ne lit jamais la messagerie de la personne |
| NX-102 | Famille : **diffusion** à plusieurs foyers créant un fil par foyer (`is_broadcast`) | déjà : lot individuel ≤ 30 (MSG-133) ; au-delà, publication (17) |
| NX-103 | Famille : réglage `allowParentAttachments` (couper les pièces des parents), page de réglages direction | déjà (MSG-261) |
| NX-104 | Rétention interne de 365 jours avec suppression des objets S3, par établissement (`app/lib/messaging/retention.ts`) | §6.1 (durées) ; **sans suppression physique hors purge visée**, avec gel et journal de purge |

### 4.3 Défauts de Next.js (signalés à Florian ; Next.js reste intouchable)

| # | Gravité | Défaut [F] |
|---|---|---|
| NXD-1 | **critique** | `/api/channels/**` : session seule ; lecture de tous les messages, modification et suppression de tout salon et de tout message par tout utilisateur connecté ; salon « privé » non appliqué (`app/api/channels/route.ts:15-79`, `messages/route.ts:64-125`) ; exposition réelle aux comptes famille à vérifier (le proxy des comptes « appli » ne liste pas `/api/channels`, `channel-access.ts:11-31`) |
| NXD-2 | **critique** | Pièce famille : la clé S3 est **fournie par le client** sans contrôle de dépôt ni de préfixe, puis servie en URL signée (`app/api/famille/messages/route.ts:53-77`, `famille-messaging-db.ts:253-277`) ; exploitabilité à confirmer |
| NXD-3 | élevée | IDOR : `famille/messages/attachment/[id]` ne vérifie pas que le fil appartient au foyer du parent (`route.ts:13-17`, le commentaire l.16 le reconnaît) |
| NXD-4 | élevée | « Professeur limité à ses classes » contournable par un `foyerId` explicite ou un `threadId` quelconque ; la liste du personnel renvoie les 100 derniers fils de l'établissement (`vie-scolaire/messages-foyer/route.ts:42-148`, `famille-messaging-db.ts:146-168`) |
| NXD-5 | élevée | Pièces : type déclaré par le client, aucune signature, aucun antivirus, zip et tout `image/*`, `audio/*`, `video/*` admis ; `channels/upload` sans aucune limite et en URL publique |
| NXD-6 | élevée | Channels « anonymes » : auteur mis à `null`, message inattribuable |
| NXD-7 | élevée | Channels stockés dans un tableau JSON unique relu et réécrit à chaque message (pertes en concurrence ; contraire à D-036) |
| NXD-8 | moyenne | 120 caractères du message dans le Web Push ; 280 dans `famille_notif.preview` |
| NXD-9 | moyenne | Présence de tout membre de l'établissement lisible de tous ; « vu par » avec e-mail et image ; messages d'erreur internes renvoyés au client |
| NXD-10 | moyenne | Édition et suppression **écrasent** le contenu (aucun historique) |
| NXD-11 | moyenne | Purge **physique** à 365 jours, lancée à la main (aucune planification trouvée) ; conversation détruite quand le dernier membre part ; aucun gel |
| NXD-12 | moyenne | Contrôle de clé de pièce par `includes` au lieu de `startsWith` ; aucune limite de longueur du corps interne |
| NXD-13 | moyenne | Salles de visio et limiteur de l'IA en mémoire du processus ; STUN de Google imposé (adresses IP des participants chez un tiers) |
| NXD-14 | moyenne | Envoi non transactionnel (message, pièces, notification) |
| NXD-15 | faible | Rôles testés par sous-chaîne ; classes comparées par préfixe (« 4 » correspond à « 4B ») ; 100 fils sans pagination |
| NXD-16 | faible | Cache de la première page possiblement obsolète après édition |
| NXD-17 | faible | Aucune limitation de débit (sauf l'assistant) |

**Reprise des données (D-035, T0 P06, P07)** [R] : messagerie interne → **archive en lecture seule** pendant sa durée de rétention (pas de reprise dans le socle 16) ; fils famille **ouverts** → repris en lecture seule, chacun rattaché à ses participants réels (même règle que MIG-3 : un foyer, un fil) ; pièces `content_base64` → exportées en fichiers (un `pg_dump` ne suffit pas pour S3) ; channels, présence, `famille_notif`, `messaging_signal_relay`, `messaging_call_ring` → **perdables**.

---

## 5. Parcours et manques, persona par persona

### P01 — Mme Kessler, professeure de français, la 4ᵉ C « difficile »
**Sa semaine.** Six classes, dont la 4ᵉ C : 28 élèves, quatre familles qui écrivent beaucoup. Elle corrige le dimanche soir et répond dans la foulée. Lundi, neuf devoirs non rendus. Mercredi, un incident en classe ; jeudi, une mère conteste une note par six messages. Elle a reçu un message d'élève à 23 h 40.
**Irritants** : répondre vingt fois la même chose ; les familles qui lui écrivent pour une absence ; les fils qui ne finissent jamais. **Peurs** : un parent agressif, une capture d'écran sortie de son contexte, écrire au mauvais parent. **La ravirait** : écrire le dimanche sans réveiller personne, ne rien recevoir le week-end, passer la main à la vie scolaire d'un geste.

- **M-2301 (bloquant → D24)** Aujourd'hui son message du dimanche 21 h part et sonne à 21 h. Il faut le différé (§0 C) et l'écran qui l'annonce : « partira lundi à 7 h ».
- **M-2302 (important)** Un message différé reste **modifiable et annulable jusqu'à sa remise** (le délai de 15 minutes de R-M27 ne court qu'à partir de la remise) ; vue « mes envois différés » (R-M48, MSG-326).
- **M-2303 (important)** La plage calme protège aussi **les personnels entre eux** : un message d'un collègue à 22 h est remis, mais son avis attend 7 h, sauf urgence de la direction (R-M49). La conception ne le dit pas.
- **M-2304 (important)** Pendant les **vacances scolaires**, les avis vers les personnels sont suspendus par défaut ; la réponse d'absence de la boîte ou du professeur l'indique aux familles (R-M49).
- **M-2305 (important)** Aucun délai de réponse n'est promis **au nom d'un professeur** : la famille voit « les professeurs répondent pendant leur temps de travail ; pour une question urgente, écrivez à la vie scolaire ». L'obligation de réponse de D. 111-4 pèse sur l'établissement, tenue par les boîtes (R-M45) ; le professeur peut **passer la main** (MSG-320).
- **M-2306 (important)** **Passer la main** : « transmettre à la vie scolaire / au PP / à la direction » avec une note interne ; la famille voit « votre demande est suivie par la vie scolaire » ; le professeur peut sortir du fil (MSG-320).
- **M-2307 (important)** **Débit** : une famille ouvre au plus 3 nouveaux fils par jour vers une même personne ; au-delà, l'écran l'oriente vers la boîte vie scolaire (règle fixe, sans jugement, R-M50).
- **M-2308 (important)** **Aiguillage à la saisie** côté famille : un message qui ressemble à une absence (« sera absent », « malade », « rendez-vous médical ») propose « l'adresser à la vie scolaire ? » (mots repérés de façon déterministe, sans IA, MSG-327).
- **M-2309 (important)** Contestation de note : « clore et proposer un rendez-vous » (carte du socle 19) au lieu d'une discussion sans fin (MSG-147 enrichi).
- **M-2310 (important)** Protection : en cas de menace, elle **demande** à la direction le gel et l'export horodaté (MSG-321) ; elle ne s'exporte pas elle-même ses fils famille.
- **M-2311 (confort)** Résumé du matin déterministe : « 12 nouveaux : 3 ressemblent à des absences (→ vie scolaire), 2 questions de devoir… » (MSG-185, MSG-303).
- **M-2312 (confort)** Mobile : mode « hors service » d'un geste qui coupe tout avis jusqu'à l'heure choisie (MSG-184).

### P02 — M. Royer, professeur principal de 2ᵈᵉ 4
**Son trimestre.** 35 élèves, une équipe de 12 enseignants dont une suppléante en novembre, une AESH. Bilan de mi-trimestre personnalisé à chaque famille ; conseil de classe en décembre ; compte rendu.

- **M-2313 (important)** Le lot individuel est plafonné à **30** (S-139) : il ne peut pas écrire aux 35 familles de **sa** classe. Recommandation : pour le PP, le plafond est **l'effectif de sa classe** (borne : sa classe, 40 au plus) ; ailleurs, 30 (S-450, R-M13 précisé).
- **M-2314 (important)** **Lot à textes individuels** : 35 bilans différents, préparés un par un (avec l'IA si elle est disponible), relus dans un aperçu qui déroule chaque texte, **une** confirmation N3 pour le lot avec empreinte de l'aperçu. CM-76 (« chaque brouillon confirmé ») est précisé : chaque brouillon est **relu**, le lot est confirmé une fois (MSG-328, CM-81).
- **M-2315 (important)** Tant que M9 n'existe pas, le fil d'équipe est manuel : prévoir que M1 pose déjà le **fil interne à plusieurs** (groupe nommé, ajout et retrait de membres par son auteur), repris de NX-90 ; M9 le rend automatique.
- **M-2316 (important)** Compte rendu du conseil aux familles : publication du PP à sa classe (socle 17, ANN-100) ; celui des délégués de parents : P15.
- **M-2317 (confort)** Il voudrait voir « qui, dans l'équipe, a écrit récemment à la famille de Léo » : **refusé** (R-M1, lire = participer) ; un collègue le dit dans le fil d'équipe ou ajoute le PP au fil (MSG-140). Refus motivé, noté pour la recette.

### P03 — Mme Benali, CPE, et Yanis, AED
**Le lundi matin.** 60 messages depuis vendredi : absences, retards, une rumeur de bagarre, un père qui demande l'adresse de la mère, une élève qui « veut parler à quelqu'un ».

- **M-2318 (important → D26)** Sous la règle provisoire, Yanis (AED) ne lit aucun fil privé d'un parent : il ne peut pas traiter la boîte. Correctif par action D-017 (§3.2).
- **M-2319 (important)** Avant les boîtes (M3), une **file « messages des familles non traités »** pour la vie scolaire (fils privés des parents de son périmètre, sans réponse d'un personnel), triée par ancienneté : c'est ce qu'il faut pour la démo et pour D. 111-4 (MSG-329, M0b).
- **M-2320 (important)** Absence signalée par message : bouton « déclarer l'absence depuis ce message » (carte du socle 12) dès que l'appel le permet ; la pièce jointe devient le justificatif (§12 point 6, M7 avancé partiellement).
- **M-2321 (important)** Un parent demande l'adresse ou le téléphone de l'autre : **alerte de rédaction** quand une réponse du personnel contient un motif de coordonnée (adresse, téléphone, e-mail) dans un fil dont l'élève a une protection (adresse protégée, ordonnance) (R-M57).
- **M-2322 (important)** **Réponses types de la boîte traduites une fois par un humain** (absence reçue, rendez-vous, document à fournir), dans les langues de l'établissement : utile sans IA (MSG-330, P08).
- **M-2323 (confort)** Astreinte du vendredi soir (CM-44) : ne déclencher l'astreinte que sur mots d'alerte **et** expéditeur élève ou famille ; jamais sur un message de personnel.

### P04 — Mme Duval, cheffe d'établissement ; M. Benyahia, principal du collège
**La crise.** Mardi 10 h 12, alerte intrusion au collège : confinement. Le socle 8 lance l'alerte U3 (modèles validés à froid). Dans les dix minutes, 600 parents écrivent ; les téléphones des adultes confinés vibrent. Le soir, fin d'alerte ; le lendemain, information aux familles et cellule d'écoute.
**La supervision.** En janvier, une plainte vise les échanges d'un professeur avec un élève de 3ᵉ ; elle doit lire le fil.

- **M-2324 (bloquant, §0 C)** **Silence de crise** : pendant une alerte U3 active de type intrusion ou confinement, la messagerie n'émet **aucun** avis, vers personne ; elle reprend à la fin d'alerte (R-M52). À coordonner avec le socle 8 (qui ne fait passer que les messages de crise, silencieux par défaut pour l'intrusion [H, consignes PPMS à vérifier]) et le socle 14.
- **M-2325 (important)** **Mode crise** côté familles : bandeau de l'alerte en tête de l'écran « Écrire », réponse automatique (« l'établissement applique ses consignes de sécurité ; ne venez pas ; suivez les messages officiels »), nouveaux fils dirigés vers une boîte « cellule de crise » (MSG-331, CM-82).
- **M-2326 (important)** **Fil de crise interne** silencieux (personnels présents), ouvert d'un geste par la direction, tant que le socle 14 (liste des présents) n'existe pas ; jamais un canal d'alerte (MSG-332).
- **M-2327 (important)** **Charte de messagerie** : gabarit fourni et versionné (MSG-129) qui prévoit la supervision, les personnes désignées, la plage calme, la conservation ; le serveur **refuse** la supervision motivée tant que la version en vigueur de la charte ne contient pas la clause (R-M53). Avis juridique toujours requis, surtout dans le public (C. pén. 432-9).
- **M-2328 (important)** Pilotage D. 111-4 : « demandes de familles sans réponse depuis plus de 7 jours ouvrés », par boîte, sans classement des agents (MSG-265 précisé).
- **M-2329 (important)** **Revue annuelle des gels** : un gel sans procédure active depuis 12 mois remonte à la direction pour levée ou maintien motivé (sinon conservation infinie, §6.1) (R-M60, MSG-333).
- **M-2330 (confort)** Exercice PPMS : la messagerie applique le mode « exercice » du socle 8 (bandeau préfixé, aucune boîte de crise).

### P05 — Mme Morel, secrétaire, et M. Gomes, agent d'accueil
**La journée.** 40 appels ; « rappeler Mme X », « Léo sera absent », « je suis le père de Léo, il est bien là ? » ; le courrier papier du matin (mairie, rectorat, une lettre de parent) ; cent demandes de certificat de scolarité par an.

- **M-2331 (important)** Message téléphonique vers un professeur : une carte **« rappeler »** dont le numéro est lu au moment de la lecture dans le socle 4 si le lecteur y a droit ; un numéro tapé par l'accueil est marqué « donné par l'appelant ». Jamais de recopie d'un numéro protégé dans un fil (R-M54).
- **M-2332 (important)** **Appelant non identifiable** ou sans droit (interdiction, lien suspendu) : l'accueil ne confirme ni la présence ni l'absence de l'élève ; l'écran du message téléphonique affiche « transmettre à la direction » sans motif (R-M51, CM-83).
- **M-2333 (important)** **Courrier reçu** : scan déposé dans le coffre (socle 7) et transmis par carte document à la boîte ou à la personne destinataire, avec un registre (date, expéditeur, objet, destinataire) ; la durée « chrono de courrier » de l'instruction de 2005 s'applique au registre (§8 : socles 7 et 14).
- **M-2334 (important)** Réponses de la boîte secrétariat par **carte document** (certificat généré par le socle 7) en un geste, et réponses types (MSG-330).
- **M-2335 (important)** Convocation disciplinaire : la messagerie ne vaut **jamais** preuve de convocation (D. 511-31 : recommandé ou remise contre signature) ; la carte « convocation » affiche « envoyée par recommandé le … » quand le socle 13 le sait (R-M65).

### P06 — Mme Lefèvre (lycée pilote) et M. Caron (collège public), infirmiers
**Ce qui existe [F].** Aucune messagerie pour l'infirmerie aujourd'hui : le rôle `infirmerie` n'a pas `messagerie.eleve` (`actions.rs`) ; le module santé est désactivé (migration 216). Rien ne fuit, rien n'existe.
**Ce qu'ils veulent.** Fixer un rendez-vous, prévenir un retour à la maison, rappeler un PAI à renouveler, être joints discrètement par un élève. **Ce qu'ils redoutent** : que la vie scolaire ou un professeur lise un échange de santé ; héberger des données de soins hors HDS ; un message d'élève sur une contraception d'urgence (secret vis-à-vis des parents, CSP L. 5134-1, `34_SANTE.md` §0 point 3 e).

- **M-2336 (bloquant pour l'activation)** La boîte infirmerie n'est **pas ouverte avant M3** (boîtes C3 testées) ; garde D28 entre-temps.
- **M-2337 (important)** **Pas de texte libre en N-0 et N-1** : la boîte infirmerie propose des **demandes structurées** — élève : « je voudrais voir l'infirmière » (créneau souhaité, « c'est urgent » oui / non, aucun champ libre) ; famille : motif codé non médical (retour à la maison, PAI à renouveler, traitement à déposer, rendez-vous, autre) et rappel demandé ; personnel infirmier : réponses types (rendez-vous, « passe me voir ») et cartes. Le **texte libre** n'existe qu'en **N-2** (cabinet sous HDS). Raison : un élève écrira son motif quoi qu'en dise un avertissement ; ne pas offrir le champ est la seule minimisation qui tienne (R-M58, Q-M21 précisé, **soumis au DPO**).
- **M-2338 (important)** Public : l'infirmier trace ses actes dans LIEN ; ScolIA ne sert que la logistique (comme `34_SANTE.md` §6) ; aucune « synthèse » vers LIEN.
- **M-2339 (important)** Absence de l'infirmière : réponse automatique « l'infirmerie est fermée jusqu'au … ; en cas d'urgence, adressez-vous à la vie scolaire ou appelez le 15 » ; la remplaçante désignée (MSG-175) reçoit la boîte.
- **M-2340 (confort)** Information de la vie scolaire « Léo rentre chez lui » : carte « mouvement » du socle 34 (sans motif), jamais un message.

### P07 — Mme et M. Lopez, séparés sous ordonnance de protection
**La situation.** L'ordonnance (C. civ. 515-11) interdit à M. Lopez d'entrer en relation avec Mme Lopez et autorise la dissimulation de son domicile ; **il garde l'autorité parentale** et doit recevoir les informations dues sur Léo. Ce n'est donc **pas** une interdiction de communication avec l'établissement (CM-05) : c'est une protection **entre les deux parents**.

- **M-2341 (bloquant → D22)** Aujourd'hui, le fil de la CPE « aux parents de Léo » réunit les deux foyers ; la réponse de la mère est lue par le père.
- **M-2342 (important)** **Protection entre responsables** : dès que le socle 4 connaît une protection entre deux responsables (ordonnance, adresse protégée vis-à-vis de l'autre, interdiction de contact entre eux), aucun fil, aucune carte d'action ni aucun formulaire ne les réunit, même s'ils déclaraient un même foyer ; l'écran du personnel affiche « parents : 2 conversations séparées (obligatoire) » (R-M55, CM-84).
- **M-2343 (important)** Les **cartes d'action** partagées entre titulaires (autorisation, signature) n'affichent jamais l'action de l'autre parent ni sa date : « accord recueilli » seulement (R-M56).
- **M-2344 (important)** Alerte de rédaction sur les coordonnées (R-M57, P03).
- **M-2345 (important → D25)** Si l'interdiction n'a été saisie que par libellé, un nouveau lien créé ensuite n'est pas actif dans la messagerie tant que la direction n'a pas confirmé (raccord socle 4).
- **M-2346 (important)** La phrase « les deux responsables ont été informés » (§9.1 point 9) est affichée **au personnel seulement**, jamais à un parent (elle confirme à un parent violent que l'autre est encore joint).

### P08 — Mme Kovalenko, parent non francophone
**Son besoin.** Elle lit l'ukrainien et le russe, peu le français ; son fils sert d'interprète, ce qu'elle n'aime pas.

- **M-2347 (important)** Tout ce qui est **système** (écrans, cartes d'action, gabarits, réponses types de boîte) est traduit par des humains, une fois, dans les langues déclarées par l'établissement : c'est la base, sans IA (R-M59).
- **M-2348 (important)** **Traduction à la lecture, côté destinataire** (N0), à la demande, marquée « traduction automatique, peut contenir des erreurs », original à côté : c'est le défaut. La traduction **sortante** par le personnel (N3, texte français joint) reste possible mais n'est pas le chemin ordinaire (§6.7, MSG-305, MSG-306).
- **M-2349 (important)** Jamais de traduction automatique d'un message de **discipline, de santé, d'orientation ou d'information préoccupante** sans la mention « faites vérifier par un interprète » ; jamais dans une boîte C3 (CM-77).
- **M-2350 (important)** **L'élève n'est jamais mis dans le fil de ses parents pour traduire** (R-M9) ; un interprète tiers sans compte peut être ajouté au fil par lien nominatif à usage limité (socle 7 R-D8), avec engagement de confidentialité (MSG-334).
- **M-2351 (important)** Le personnel voit, en écrivant, « langue de lecture : ukrainien » (préférence du socle 4) ; l'IA peut proposer une version simplifiée (N3).
- **M-2352 (confort)** Écriture de droite à gauche (arabe, hébreu) correcte à l'écran et dans l'export PDF.

### P09 — M. Vidal, parent aveugle
- **M-2353 (important)** Une publication ou un message de lot dont l'information n'est que dans une **image** (affiche scannée) est refusé sans texte équivalent ; pour un message individuel, l'écran le demande (R-M61).
- **M-2354 (important)** Prise de connaissance et signature **sans geste graphique** : bouton « j'ai pris connaissance », passkey (D-009) pour ce qui l'exige (R-M35 précisé).
- **M-2355 (important)** Chaque lot de la messagerie a dans ses critères de fin un passage **clavier + lecteur d'écran** (NVDA, VoiceOver) et axe-core, pas seulement M14 (RGAA 4.1.2, §7 R-10).
- **M-2356 (important)** Fil navigable par titres (un message = un titre de niveau 3 avec auteur et date), nouveaux messages annoncés une fois (région ARIA polie), pas de défilement infini sans repère.
- **M-2357 (confort)** Préférence « texte brut » des avis e-mail (plages braille).
- **M-2358 (confort)** Dictée vocale aussi pour les familles (MSG-152 étendu).

### P10 — M. Fabre, parent qui harcèle Mme Kessler
**Les faits.** Vingt messages par jour, insultes, menaces voilées ; captures publiées sur un réseau social.

- **M-2359 (important)** Débit R-M50 (au-delà de trois fils par jour vers elle, orientation vers la boîte) et **plafond de messages** par fil et par jour (20) pour un compte famille, avec avis à la direction au dépassement (R-M50).
- **M-2360 (important)** **Lots** : M10 (signalement, restriction) passe **avant tout usage réel avec des familles**, pas seulement avant le pilote avec des élèves (§27.1 de la conception, phrase à corriger).
- **M-2361 (important)** Restriction `boite_seulement` vers la boîte direction ; le professeur **sort du fil** sans le clore ; elle voit « les échanges avec cette famille passent par la direction » ; l'information due continue (R-M40).
- **M-2362 (important)** La personne restreinte peut **demander un réexamen** à la direction (une fois par période de 30 jours) ; décision motivée et tracée (MSG-335).
- **M-2363 (important)** Avertissement doux avant envoi des termes injurieux évidents **aussi pour les familles** (S-142 l'avait réservé aux élèves) ; jamais de blocage (S-451).
- **M-2364 (confort)** Export horodaté à empreintes pour la plainte (MSG-231) avec la mention de la protection fonctionnelle de l'agent [NV] (MSG-321).

### P11 — Chloé, 14 ans, harcelée
**La nuit.** 22 h, après une soirée de messages humiliants sur un réseau social, elle ouvre ScolIA sur la tablette familiale et écrit à la CPE.

- **M-2365 (bloquant → D21)** Aujourd'hui, ses professeurs peuvent lire ce fil.
- **M-2366 (important)** Parcours **« signaler un harcèlement »** distinct de « parler à quelqu'un » : victime ou témoin, faits dans ou hors de l'établissement, captures d'écran jointes (gelées dès le dépôt, lisibles du référent et de la CPE seulement), aucun avis automatique aux parents (décision humaine), rappel 3018 et 119 (MSG-336, CM-85).
- **M-2367 (important)** **Accusé de réception automatique immédiat** de la boîte, même la nuit : « ton message est arrivé ; il sera lu demain à partir de 7 h 30 ; si tu es en danger maintenant : 119 (jour et nuit), 3018 (jusqu'à 23 h), 15, 17, 112 ». Ce n'est pas une information nouvelle diffusée par un adulte, c'est la réponse à son propre geste : la plage calme ne s'y oppose pas (R-M62).
- **M-2368 (important)** **Tablette familiale** : le push de l'élève ne montre rien (« Nouveau message ») ; son fil n'apparaît dans aucun écran famille ; « masquer de ma liste » (archivage personnel) sans suppression ; déconnexion facile (MSG-128 précisé).
- **M-2369 (important)** Quand la CPE écrit aux familles des élèves mis en cause, rien ne nomme Chloé hors ce que la CPE écrit ; aucun fil ne réunit les familles (CM-72) ; l'enveloppe ne cite aucun élève tiers (§21.3 point 8).
- **M-2370 (important)** Le gel d'un signalement est **automatique** et revu chaque année (R-M60).
- **M-2371 (confort)** Horaires du 3018 et du 119 paramétrables, mis à jour par l'éditeur, jamais en dur (§7 R-5).

### P12 — Yasmine, 18 ans, terminale
- **M-2372 (important → D23)** Aujourd'hui, un fil de la CPE à propos de ses absences lui est invisible et va à ses parents.
- **M-2373 (important)** À J-30 de la majorité, ELV-39 (socle 4) lui demande ce que ses parents continuent de recevoir ; la messagerie lit ce choix, ne décide rien (§10.3).
- **M-2374 (important)** Son père écrit au PP « pourquoi a-t-elle manqué ? » : alerte de rédaction R-M12 au PP, avec la phrase type « nous ne pouvons pas vous communiquer d'information sans l'accord de Yasmine » (réponse type de boîte).
- **M-2375 (confort)** Elle peut écrire directement à la direction (matrice, P ✓).

### P13 — Mme Diallo, AESH
**Sa semaine.** Trois élèves, deux établissements (PIAL). Elle apprend souvent l'absence de Léo en arrivant devant la salle. Avec la famille, elle tient un **cahier de liaison** papier [H].

- **M-2376 (important)** Être **avisée de l'absence** de l'élève accompagné et des changements de son emploi du temps (nature du socle 8, données du socle 12 et 10) : c'est son premier irritant, hors messagerie (§8).
- **M-2377 (important)** **Fil de liaison famille–AESH** : ouvert par le PP ou le coordonnateur ULIS, qui y reste participant ; l'AESH y écrit, n'est jamais seule avec la famille ; remplace le cahier de liaison (Q-M11 précisé, MSG-337, R-M66).
- **M-2378 (important)** « Mes établissements » (MSG-186) pour le PIAL ; deux messageries distinctes (D-013).
- **M-2379 (confort)** Fin de contrat en cours d'année : sortie des fils à la date (R-M18), successeur ajouté.

### P14 — M. Okafor et Mme Roux, intervenants extérieurs
- **M-2380 (important)** Un intervenant extérieur n'est **jamais** participant d'un fil famille ou élève ; seulement d'un fil interne avec son référent (et l'équipe du projet), pour la durée de son engagement (socle 6), par lien nominatif à usage limité s'il n'a pas de compte (R-M66, CM-86).
- **M-2381 (important)** Annulation d'une séance : avis par le référent dans ce fil (ou SMS du socle 8 au tiers sans compte).
- **M-2382 (important)** Photos d'élèves prises pendant l'atelier : jamais en pièce jointe d'un fil ; dépôt dans un album soumis aux consentements (socles 4 et 7) (§8).
- **M-2383 (confort)** Compte invité limité (socle 6 Q-P10).

### P15 — M. Moreau, parent délégué de la 2ᵈᵉ 4
**Le cadre [V, §7 R-12].** Les associations de parents peuvent obtenir la liste des parents (noms, adresses postale et électronique) **avec leur accord exprès** ; elles diffusent leurs documents sans contrôle préalable, dans des limites (vie privée, injure, propagande), les désaccords allant à l'autorité académique (C. éduc. D. 111-6 à D. 111-9).

- **M-2384 (important)** **Contradiction** avec la borne du §8.2 (« jamais un fil famille ↔ famille ») : recommandation : **boîte des représentants de parents** de la classe (membres = parents élus de l'année, socle 46 ; à défaut, désignés par la direction), à laquelle les parents peuvent écrire ; les délégués répondent au nom de la boîte ; aucun parent ne voit l'adresse d'un autre ; la borne reste vraie entre personnes (R-M3 inchangée) (CM-87, MSG-338).
- **M-2385 (important)** **Publication des représentants** (compte rendu du conseil) diffusée par ScolIA à l'audience « parents de la 2ᵈᵉ 4 qui ont accepté » : **consentement exprès** enregistré par parent, révocable (table, pas un booléen caché) ; auteur affiché « Représentants des parents » ; **pas de validation de fond** par l'établissement (D. 111-9) ; la direction peut seulement suspendre pour un motif des limites légales, tracé, avec renvoi à l'autorité académique (ANN-117, ANN-118).
- **M-2386 (important)** Fin de mandat : sortie de la boîte ; les fils restent à la boîte et aux successeurs.
- **M-2387 (confort)** Espace des associations (APEL, FCPE) : socle 46 / 17.

### P16 — M. Blanc, DPO de l'OGEC, et le DPO académique
- **M-2388 (important)** **Fiche de traitement** de la messagerie générée (finalités, données, destinataires, durées, sous-traitants, transferts) et **AIPD pré-remplie** (mineurs, supervision, signalements, santé possible, IA) (MSG-339).
- **M-2389 (important)** **Durées** proposées au §6.1, à valider (soumis, §10.2).
- **M-2390 (important)** **Sous-traitants et transferts** : Apple et Google (push : enveloppe neutre seulement, métadonnées hors UE), OVH (SMS), Scaleway TEM (e-mail), Mistral (IA, sous garanties) ; liste tenue dans la fiche.
- **M-2391 (important)** Journal des supervisions et des accès sensibles lisible du DPO sans contenu (MSG-232) ; journaux 12 mois au plus (kit SDET [V]).
- **M-2392 (important)** **Accusés de lecture minimisés** (§6.5) : `lu_le` stocké en **date**, pas en horodatage.
- **M-2393 (important)** Droit d'accès d'un mineur : exercé avec ses responsables pour ses propres fils sans révéler aux parents leur contenu ? **Question au DPO** (Q-M5, Q-M24) ; recommandation : l'élève de 15 ans et plus exerce seul le droit d'accès à **ses** fils.
- **M-2394 (important)** Rapport « exceptions D-036 » de la brique (MIGRATIONS.md) et preuves de purge (lot, nombre, catégorie, visa).
- **M-2395 (confort)** Carte « qui voit quoi » de la messagerie (§21.2) générée pour l'information des personnes (charte).

### P17 — Les attaquants
| Attaque | Ce qui la bloque [R] | Manque |
|---|---|---|
| (a) Élève qui usurpe le compte d'un parent (mot de passe noté) pour justifier ses absences et lire la CPE | passkey obligatoire à terme (D-009) ; alerte « nouvel appareil » au titulaire ; justification par message = message non fiable, jamais d'effet automatique | **M-2396 (important)** alerte d'appareil nouveau et liste des appareils (socle 1) visibles du parent |
| (b) Père interdit qui recrée un compte et se fait lier au guichet | D25 ; guichet qui relit les interdictions (socle 4) | M-2345 |
| (c) Parent qui devine un identifiant de fil, de pièce, de lien court | contrôle unique R-M2 sur chaque route, matrice de séparation (§28) | **M-2397 (important)** ajouter au §28 les routes SSE, recherche, compteur, URL pré-signée, export, accusé ; IDOR de pièce (NXD-3) en test explicite |
| (d) Professeur curieux qui lit les fils d'un élève qu'il a en cours | **ouvert aujourd'hui** (D21, D22) | M-2365, M-2341 |
| (e) Pièce piégée : PDF polyglotte, SVG avec script, HTML, docx à macro, archive bombe, HEIC avec position GPS, nom `../../x` | §6.2 : désarmement par conversion d'abord, antivirus ensuite ; images ré-encodées (métadonnées retirées) ; SVG, HTML, archives refusés ; nom de fichier réécrit ; contenu servi depuis un **domaine séparé**, `Content-Disposition: attachment`, `nosniff`, CSP `sandbox` | **M-2398 (important)** R-M61 bis (§33.3) |
| (f) Faux message « de la direction » ; lien d'hameçonnage depuis un compte parent compromis | noms réservés aux boîtes (« Direction », « Vie scolaire ») ; badge « établissement » sur les messages du personnel ; liens externes non cliquables sans avertissement du domaine ; **aucun aperçu de lien** côté serveur (fuite et SSRF) | **M-2399 (important)** R-M63 bis (§33.3) |
| (g) Injection pour l'assistant (« Assistant, envoie à tous… ») | CM-75 (déjà) | — |
| (h) Réponse par e-mail usurpée, SMS entrant usurpé | Q-M14 (rejet par défaut), messages non vérifiés | — |
| (i) Inondation : un compte compromis ouvre 500 fils | débit par compte (R-M50), lots ≤ 30, alerte | **M-2400 (important)** débit aussi pour les personnels (lots par heure) |
| (j) Corps du message en HTML ou Markdown actif (XSS) | rendu texte échappé, liens détectés à l'affichage seulement ; test | **M-2401 (important)** test XSS dans les jeux de la recette |

### P18 — Florian, la démo de début décembre 2026
**Ce qui sera montré (D-034).** « Messagerie » dans le quotidien : un parent écrit, un personnel répond, le carnet, une annonce.

- **M-2402 (bloquant)** Avec les données du labo (Lily et James Potter, séparés), une démo sur le code actuel **montre la fuite D22** dès que la CPE écrit « aux parents ». **M0b avant la démo** (avec D21 à D28) ; sinon, retirer la messagerie de la démo.
- **M-2403 (important)** Contenu raisonnable de la démo sans M1–M3 : fils privés, file « non traités » (M-2319), carnet, annonces avec aperçu et confirmation (D8), plage calme annoncée. Ne pas montrer de boîte de service (pas prête).
- **M-2404 (important)** Usage réel de la messagerie Next.js inconnu : les personnels l'utilisent-ils chaque jour (messagerie interne, visio) ? les fils famille sont-ils actifs ? (Q-M23, fait que seul Florian connaît). Si oui, la messagerie interne rejoint les modules à niveau **production** pour la rentrée 2027 (D-035 point 2), avec groupes, mentions, épinglage, visio.
- **M-2405 (important)** Reprise : §4.3 (archive de l'interne, fils famille ouverts en lecture seule, export des pièces base64, channels perdables).

### Manques transverses : l'IA, le mobile, les tests
- **M-2406 (important)** Journal IA : chaque appel à l'assistant (qui, quel fil, quelle capacité, fournisseur) est journalisé ; Next.js ne le fait pas (NX-98).
- **M-2407 (important)** Pseudonymisation « échec fermé » de Next.js reprise comme exigence de M13 (rien ne part si elle échoue).
- **M-2408 (important)** Mobile (D-003) : avis sans contenu, plage calme respectée par le badge (mise à jour silencieuse), mode crise (R-M52) appliqué aux applis.
- **M-2409 (important)** Tests : chaque défaut D21 à D28 a un test d'attaque précédé d'un témoin, comme M0a.
- **M-2410 (confort)** Statistiques de bruit par famille (socle 8) visibles de la direction pour repérer une famille submergée.
- **M-2411 (différenciant)** Aiguillage à la saisie (M-2308) et réponse automatique de nuit (M-2367) : deux gestes sans IA qui allègent la vie scolaire et rassurent les élèves.
- **M-2412 (différenciant)** Boîte des représentants de parents avec consentement exprès (M-2384) : aucun logiciel connu [H] ne la propose proprement.

---

## 6. Points à trancher

### 6.1 Durées de conservation (point ouvert de la conception → **DPO et service d'archives**)
Constat [V, §7 R-2] : aucune durée propre aux messages dans le kit juridique ni dans l'annexe opérationnelle ; base active « année scolaire ou cycle » ; données supprimées dans les 3 mois après la perte du droit au compte ; l'ENT n'est pas un système d'archivage ; l'élimination demande le visa du service d'archives (archives publiques, annexe 2024 §3.16). Proposition [R] (hypothèses de travail, appliquées dans le code comme paramètres par catégorie, **à valider** avant toute donnée réelle) :

| Catégorie | Durée proposée | Fondement |
|---|---|---|
| Fil famille ou élève | jusqu'à la **fin de l'année scolaire qui suit son dernier message**, puis purge par lot visé ; au plus tard 3 mois après la perte du droit au compte ; ce qui fonde une décision est **versé au dossier** avant (MSG-149) | kit (année ou cycle) ; arrêté de 2006 (3 mois) ; instruction de 2005 (mots et rappels aux familles : 1 an) |
| Mots du carnet | fin de l'année scolaire suivante | instruction de 2005 (mots d'excuse) |
| Fils internes et d'équipe | fin de l'année scolaire suivante | proportionnalité [H] |
| Boîte infirmerie N-0 / N-1 (demandes structurées) | fin de l'année scolaire | minimisation |
| Boîte infirmerie N-2 | règle du cabinet (`34_SANTE.md`) | CSP |
| Signalement et ses pièces | clôture du traitement + 1 an, sauf gel ou procédure | instruction de 2005 (signalements : 1 an) |
| Fil gelé | jusqu'à la levée ; **revue annuelle obligatoire** (R-M60) | éviter la conservation sans fin |
| Journal des supervisions et accès sensibles | 12 mois | kit SDET (journaux 12 mois au plus) |
| « Lu » (accusé) | suit le message ; stocké en date seulement | minimisation (§6.5) |
| Message téléphonique, message entrant non rattaché | 3 mois après traitement | [H] |
| Fichier d'export produit | 30 jours (le fait de l'export reste au journal) | [H] |
| Brouillon, envoi différé annulé | 90 jours | [H] |
| Pièce refusée par l'analyse | 30 jours en quarantaine (preuve), puis effacement | [H] |
| Publications (socle 17) | année + 1, archive (ANN-116) | [H] |

**Soumis à Florian** (§10.2) : faire valider ce tableau par le DPO de l'OGEC et, pour un EPLE, par le service d'archives départemental (visa d'élimination).

### 6.2 Antivirus des pièces jointes (point ouvert → **soumis à Florian : dépendance nouvelle**)
- **Recommandation [R]** : chaîne « **désarmer d'abord, analyser ensuite** ». (1) Formats admis bornés (PDF, images, documents bureautiques ouverts) ; SVG, HTML, archives, exécutables, PDF protégés refusés. (2) **Conversion** par le moteur maison (D-005) : images ré-encodées (métadonnées EXIF, dont la position GPS, retirées), PDF et bureautique reconstruits ; **c'est la version convertie qui est servie**, l'original reste en quarantaine non servie. (3) **ClamAV** en processus séparé (démon `clamd`, appelé par socket depuis un conteneur isolé, signatures mises à jour plusieurs fois par jour) : licence **GPL-2.0** [V, dépôt Cisco-Talos] ; aucune édition de liens avec `libclamav` dans nos binaires (l'appel par socket à un programme séparé n'étend pas la GPL à notre code : raisonnement général [NV], à confirmer). (4) Servi depuis un domaine séparé, en téléchargement, `nosniff`, CSP `sandbox`.
- **En attendant la décision** : M5 part avec (1), (2), (4) ; l'antivirus est un adaptateur (`AnalyseurAntivirus`, aujourd'hui `AucuneAnalyse`, `ent-object/src/upload.rs:339`) branché ensuite ; les pièces des familles restent désactivables (MSG-261).
- **Soumis** : adopter ClamAV (identité, activité, licence, sécurité vérifiées ici ; décision de Florian, CLAUDE.md « toute autre brique »).

### 6.3 Messagerie de l'infirmerie (point ouvert → **soumis au DPO**)
Aligné sur `34_SANTE.md` §10.15, avec une précision : en **N-0 et N-1**, pas de texte libre (demandes structurées, M-2337) ; en **N-2** (cabinet sous HDS), fils stockés dans le cabinet, texte libre, pièces en plis ; **public** : logistique seulement, LIEN pour les actes ; boîte ouverte seulement après M3 (isolement C3 testé) ; garde D28 d'ici là. Aucune règle trouvée sur la messagerie d'un infirmier scolaire avec une famille ; L. 1111-8 (HDS) vise les données recueillies lors d'activités de prévention et de soins [V partiel] ; MSSanté existe pour les professionnels de santé scolaires [NV] : à étudier pour les échanges avec les médecins traitants, hors ScolIA.

### 6.4 Plage calme
**Recommandation (appliquée par défaut, D-031)** : Q-M17 confirmée et précisée :
1. Messages d'un **personnel vers une famille ou un élève** écrits de 20 h à 7 h et du vendredi 20 h au lundi 7 h : **remise différée** (visibilité et avis), annoncée avant l'envoi, modifiable et annulable jusqu'à la remise (R-M48) ; dérogation de la direction tracée.
2. Messages **entre personnels** : remis, **avis différés** (droit à la déconnexion) ; chacun peut choisir de recevoir quand même (préférence personnelle, R-M49).
3. Messages **des familles et des élèves** : enregistrés à toute heure ; avis aux personnels selon leur disponibilité (socle 8 §16) ; **accusé automatique** de la boîte permis (R-M62).
4. **Vacances scolaires** : avis vers les personnels suspendus par défaut ; réponse d'absence des boîtes.
5. **Urgence** : la crise passe par le socle 8 (U3), jamais par la messagerie ; pendant une alerte intrusion, silence de la messagerie (R-M52).
6. **Privé sous contrat** : le texte vise surtout le public [NV, §7 R-1] ; ScolIA applique le même défaut partout (un défaut protecteur coûte peu) ; l'établissement peut **élargir** librement, **réduire** avec motif tracé (MSG-262).
7. Fuseau de l'établissement, jamais `Europe/Paris` en dur.

### 6.5 Accusés de lecture
**Recommandation (appliquée par défaut)** :
- **Famille, élève** : voient « lu par l'établissement » (boîte) ou « lu par Mme B. » (fil individuel), **sans heure**.
- **Personnel** : voit, pour chaque participant famille ou élève, « lu le 12 novembre » (date sans heure) ou « non lu depuis 3 jours ».
- **Entre personnels** : « lu » sans heure dans un fil à deux ; dans un fil d'équipe, compte agrégé (« lu par 5 sur 8 »), sans noms. Le « vu par » nominatif horodaté de Next.js (NX-94) **n'est pas repris** : un accusé horodaté vers un agent est un outil de contrôle individuel à justifier [NV, CNIL : minimisation, surveillance excessive].
- **Prise de connaissance signée** (carnet, information importante) : horodatée, car c'est une preuve.
- La supervision, l'auteur, un membre d'une autre boîte ne posent jamais d'accusé (R-M19) ; aucun pixel dans les e-mails.
- Stockage : `lu_jusqu_a_message_id` + `lu_le DATE`.

### 6.6 Modération
**Recommandation (appliquée par défaut)** : aucun filtrage bloquant ; avertissement doux avant envoi (élèves **et** familles, S-451) ; signalement en un geste ; référent harcèlement ; restriction motivée, datée, réexaminée à 30 jours, avec **demande de réexamen** par la personne (MSG-335) ; débit (R-M50) ; fils de classe et de délégués modérés **a posteriori** par l'adulte animateur (retrait tracé) ; publications à quatre yeux au-delà du seuil ; publications des représentants de parents sans contrôle de fond (D. 111-9). Aucune modération par l'IA.

### 6.7 Traduction par l'IA
**Recommandation (appliquée par défaut)** : (1) base sans IA : tout le système traduit par des humains (R-M59) ; (2) traduction **à la lecture**, à la demande, côté destinataire (N0), marquée, original visible : c'est le chemin ordinaire ; (3) traduction **sortante** par le personnel : proposée, relue, confirmée (N3), texte français joint, marque machine `traduit` (règlement IA art. 50 [V]) ; **jamais envoyée sans geste humain** ; (4) jamais pour C3 ; avertissement pour discipline, santé, orientation, information préoccupante ; (5) traductions conservées dans une table `ent_msg_traduction (message_id, langue, texte, moteur, cree_le)` pour ne pas renvoyer deux fois le même texte au fournisseur ; (6) fournisseur : Mistral sous garanties, après l'étude juridique (Q-EV8) ; (7) une famille peut choisir « ne jamais traduire automatiquement ».

### 6.8 D-036 : aucune donnée en JSON
**Recommandation (appliquée par défaut)** : §0 D ; détail des tables au §33.6 de la conception. Rien du JSON de Next.js n'est repris tel quel (`roles_can_initiate` → les sept réglages typés ; channels → perdables ; `content_base64` → fichiers). Exceptions permises (déclarées à `MIGRATIONS.md`) : charge utile de l'outbox (`ent_metier_event.payload`, transitoire), instantanés d'audit, export JSON du droit d'accès (format d'échange). À signaler au socle 4 : `ent_eleve_interdiction.objet TEXT[]` et, au socle 8, `ent_campagne.canaux TEXT[]`, `cible_ids UUID[]` (`191_campagnes.sql:11-13`), `ent_formulaire.cible_ids UUID[]` (`193`) : tableaux sans clé étrangère, à classer.

---

## 7. Vérification réglementaire (2026-11-10)

| # | Règle | Référence | Source | Statut | Effet |
|---|---|---|---|---|---|
| R-1 | ENT et vie scolaire paramétrés **par défaut** sans nouveaux échanges ni notifications de 20 h à 7 h et du vendredi 20 h au lundi 7 h ; saisie visible à la fin de la plage, rappel ; adaptation locale par le chef d'établissement | circulaire « numérique raisonné » (BO n° 28 du 10 juillet 2025, MENE2519904C) ; arrêté « cadre de référence du numérique pour l'éducation » (JO du 23 septembre 2026) | https://www.banquedesterritoires.fr/pour-un-numerique-raisonne-lecole-la-circulaire-du-ministere-de-leducation-detaille-les-mesures ; https://www.banquedesterritoires.fr/numerique-pour-leducation-le-ministere-detaille-le-cadre-de-reference (BO et education.gouv.fr : 403) | **[NV]** cette passe (la conception l'avait lu sur le PDF du référentiel, [V]) ; numérotation « exigences 8 et 9 » non reconfirmée ; portée au privé sous contrat non établie ; rien sur le sens famille → personnel | §6.4 |
| R-2 | Messagerie ENT = correspondance privée ; base active année ou cycle ; journaux 12 mois au plus ; suppression 3 mois après la perte du droit ; **aucune durée pour les messages ni les accusés** ; l'ENT n'est pas un système d'archivage, élimination sous visa | kit juridique SDET 2025 (maj. février 2026) §4.4.1, §4.8, §4.11 ; annexe opérationnelle 2024 §3.16 | https://www.education.gouv.fr/sites/default/files/2025-07/kit-juridique-441540.pdf ; https://eduscol.education.gouv.fr/sites/default/files/document/sdetannexe-operationnelleversion2024pdf-106008.pdf | [V] | §6.1 |
| R-3 | Aucune position CNIL propre aux accusés de lecture ; minimisation, sanction de surveillance excessive d'un employeur | RGPD art. 5 ; cnil.fr | cnil.fr (pages de sanctions et « messagerie professionnelle ») | [NV] | §6.5 |
| R-4 | Mineurs : consentement conjoint sous 15 ans pour les seuls traitements fondés sur le consentement ; 8 recommandations CNIL (2021) | loi 78-17 art. 45 ; recommandation 4 CNIL | https://www.cnil.fr/fr/recommandation-4-rechercher-le-consentement-dun-parent-pour-les-mineurs-de-moins-de-15-ans | [V] | ENT sur mission d'intérêt public ; fonctions facultatives à part ; Q-M5 au DPO |
| R-5 | **Article 40 du CPP** : l'ordonnance n° 2025-1091 du 19 novembre 2025 **réécrit à droit constant** la partie législative du CPP au 1ᵉʳ janvier 2029, date reportable par la loi de ratification (projet n° 604, Sénat, 6 mai 2026) ; elle **n'abroge pas** l'obligation | ord. 2025-1091 | https://www.senat.fr/leg/pjl25-604.html ; https://www.actu-juridique.fr/… | [NV] | **Corrige** le §5 R-17 et le §14 point 4 de la conception (« abrogé ») : références légales paramétrables, jamais en dur |
| R-6 | 3018 : 7 j / 7, 9 h – 23 h en métropole, horaires décalés outre-mer | fiche e-Enfance 2026 | https://e-enfance.org/wp-content/uploads/2026/08/Fiche-horaires-3018-4.pdf | [NV] (association) | horaires paramétrables (M-2371) |
| R-7 | Contrôle de la messagerie : règles et personnes désignées dans la charte, « vie privée résiduelle », autorisation du juge possible ; diffusion massive = communication publique | kit SDET §4.4.1, §4.9 ; annexe 2024 §3.8.3 | (même URL que R-2) | [V] | R-M53 (charte exigée avant supervision) |
| R-8 | HDS pour les données recueillies lors d'activités de prévention, diagnostic, soins ; secret (C. pén. 226-13) ; kit MSSanté pour les professionnels de santé scolaires (ANS, 2023) | CSP L. 1111-8 ; C. pén. 226-13 | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000049577902 ; https://esante.gouv.fr/actualites/mssante-se-deploie-aupres-des-medecines-scolaires | [V] partiel ; MSSanté [NV] | §6.3 |
| R-9 | Règlement IA : art. 50 applicable depuis le 2 août 2026 ; annexe III au 2 décembre 2027 (art. 113 consolidé après l'omnibus) | règlement 2024/1689 | https://ai-act-service-desk.ec.europa.eu/en/ai-act/article-50 ; …/article-113 | [V] | marquage dès maintenant (R-M47) |
| R-10 | RGAA 4.1.2 ; obligations de déclaration rappelées par le cadre de référence de 2026 | RGAA | https://accessibilite.numerique.gouv.fr/doc/RGAA-v4.1.2.odt | [NV] pour l'applicabilité au privé | M-2355 |
| R-11 | Archives publiques : organismes de droit public et personnes privées gérant un service public ; données de l'établissement = archives publiques selon le SDET | C. patr. L. 211-4 ; annexe 2024 §3.16.4 | https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000032860057 | [V] partiel | aucune purge sans visa (public) ; statut du privé à confirmer |
| R-12 | Associations de parents : liste des parents (noms, adresses) **avec leur accord exprès** ; diffusion de leurs documents **sans contrôle préalable**, dans des limites (vie privée, injure, propagande) ; désaccord porté à l'autorité académique | C. éduc. D. 111-6 à D. 111-9 | https://www.legifrance.gouv.fr/codes/id/LEGISCTA000006182458 ; https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006525723 | [V] (attribution exacte D. 111-8 / D. 111-9 à revérifier) | P15 |
| R-13 | Droit à la déconnexion : négociation annuelle (C. trav. L. 2242-17, 7°) ; enseignants : pas de texte propre, accord fonction publique du 13 juillet 2021 et chartes | — | guide académique (Bourgogne-Franche-Comté, 2025) | [NV] | R-M49 |
| R-14 | ClamAV sous GPL-2.0, base de signatures mise à jour plusieurs fois par jour | — | https://github.com/Cisco-Talos/clamav ; https://www.clamav.net/about | [V] | §6.2 |

**Non lus dans cette passe** : C. pén. 432-9, RGPD art. 8, C. civ. 371-1, L. 111-6, 222-33-2-3, CASF L. 226-2-1, CSP R. 4312-5, loi 2005-102 art. 47, directive accessibilité (EAA), tableau de l'instruction de 2005 (la conception les cite, certains [V] le 2026-11-09).

---

## 8. Manques qui relèvent d'autres socles

| Socle | Manque | Personas |
|---|---|---|
| 1 | alerte d'appareil nouveau, liste des appareils | P17 a |
| 4 | interdiction par libellé : nouveau lien « à vérifier » (D25) ; **protection entre responsables** comme fait distinct de l'interdiction de communication (R-M55) ; choix de l'élève majeur à J-30 ; consentement exprès à la liste des représentants de parents ; tableaux `objet TEXT[]` (D-036) | P07, P12, P15 |
| 6 | fin de contrat AESH, engagement des intervenants, missions des boîtes | P13, P14 |
| 7 | registre du courrier reçu et carte document ; album photo soumis aux consentements ; désarmement par conversion (D-005) pour les pièces | P05, P14, P17 |
| 8 | **silence de crise** (aucun avis de messagerie pendant l'alerte intrusion) ; avis AESH de l'absence de l'élève accompagné ; alignement des heures calmes sur 20 h – 7 h ; enveloppes neutres des annonces (D27) ; tableaux `canaux`, `cible_ids` | P04, P13 |
| 12 | carte « déclarer l'absence depuis ce message » | P03 |
| 13 | carte convocation « envoyée par recommandé » | P05 |
| 14 | liste des présents en mise en sûreté (remplace le fil de crise interne) | P04 |
| 17 | publications des représentants de parents ; texte équivalent obligatoire des images ; audience normalisée | P09, P15 |
| 33 | visio depuis un fil (parité NX-97) | P18 |
| 34 | boîte infirmerie en demandes structurées (N-0, N-1) | P06 |
| 46 | mandats des représentants de parents (membres de la boîte) | P15 |

---

## 9. Synthèse

### 9.1 Manques par gravité
112 manques (M-2301 à M-2412) : **6 bloquants** (M-2301, M-2324, M-2336, M-2341, M-2365, M-2402, qui portent D21, D22, D24 et le silence de crise : réunis en cinq points au §0 avec D-036 et les défauts de Next.js), **87 importants**, **17 confort**, **2 différenciants**.

### 9.2 Ajouts dans la conception (`16_MESSAGERIE.md`)
Nouveau **§33** : défauts résiduels D21 à D28 ; lot **M0b** élargi (M0b-1 sécurité avant la démo, M0b-2 fiabilité) ; ordre des lots corrigé (M10 avant tout usage réel avec des familles) ; actions MSG-320 à MSG-339 et ANN-117 à ANN-121 ; règles R-M48 à R-M66 ; cas CM-81 à CM-100 ; normalisation D-036 du §25 ; points tranchés (§6 ci-dessus) ; questions Q-M23 à Q-M30 ; réglages S-450 à S-456 ; contradictions C8 à C12. Corrections ponctuelles : en tête (statut), §5 (R-17, R-18), §25 (renvois), §27.1 (M0b), §30 (Q-M10, Q-M15, Q-M21), §31.

### 9.3 Les dix ajouts les plus importants
1. **D21** : le fil d'un élève n'est plus lu de ses professeurs (M0b).
2. **D22** : un fil du personnel « aux responsables » est créé par foyer et n'est plus lu de tous les professeurs (M0b).
3. **D23** : l'élève majeur reçoit les fils qui le concernent (M0b).
4. **Plage calme minimale** en M0b et **silence de crise** (R-M52).
5. **Normalisation D-036** du modèle cible avant toute migration.
6. **Infirmerie sans texte libre** hors HDS (demandes structurées).
7. **Désarmer d'abord** : conversion, métadonnées retirées, domaine séparé ; ClamAV soumis.
8. **Protection entre responsables** (R-M55) distincte de l'interdiction de communication.
9. **Boîte des représentants de parents** avec consentement exprès.
10. **Lot du PP à l'effectif de sa classe** et lot à textes individuels confirmé une fois.

### 9.4 Ce que la passe a montré sur la conception
- Une règle **provisoire** (M0a) doit être éprouvée par personas **comme une règle définitive** : elle vivra jusqu'à M1, donc pendant la démo.
- Les cas « parents séparés » de la conception partaient de l'**interdiction de communication** ; le cas le plus fréquent et le plus dangereux est la **protection entre les parents**, où l'établissement doit continuer d'écrire aux deux.
- Le temps est le grand oublié du code : ni plage calme, ni disponibilité, ni crise.
- Le JSON s'était glissé dans le modèle cible par les réglages et les horaires, les endroits qu'on croit « techniques ».

---

## 10. Décisions : appliquées par défaut (D-031) et soumises à Florian

### 10.1 Appliquées par défaut (D-031)
1. M0b avant la démo, avec D21 à D28 (§0 A, B, C).
2. Fil d'élève : règle des fils privés ; fil du personnel aux responsables : un par foyer ; élève majeur destinataire.
3. Plage calme (§6.4), y compris entre personnels (avis différés) et vacances ; accusé automatique des boîtes ; silence de crise.
4. Accusés (§6.5) : sans heure, agrégés entre personnels, `lu_le` en date ; « vu par » nominatif non repris.
5. Modération (§6.6) : avertissement doux pour familles et élèves, débit, réexamen d'une restriction.
6. Traduction (§6.7) : à la lecture par défaut, sortante en N3, gabarits humains d'abord.
7. Normalisation D-036 (§6.8, §33.6 de la conception).
8. Infirmerie : demandes structurées sans texte libre en N-0 / N-1 (aussi soumis au DPO, 10.2).
9. Lot du PP à l'effectif de sa classe ; lot à textes individuels confirmé une fois.
10. M10 avant tout usage réel avec des familles.
11. Boîte des représentants de parents (nuance la borne « famille ↔ famille » du §8.2, qui était une recommandation non décidée).
12. Intervenants extérieurs : jamais dans un fil famille ou élève.
13. Rien n'est repris des « channels » de Next.js ; messagerie interne en archive ; fils famille ouverts en lecture seule.

### 10.2 Soumis à Florian (engagent l'extérieur, une dépendance, ou sont des faits qu'il est seul à connaître)
1. **Durées de conservation** (§6.1) : validation par le DPO de l'OGEC et, dans le public, visa du service d'archives.
2. **Antivirus** (§6.2) : adopter ClamAV (GPL-2.0, processus séparé) — dépendance nouvelle.
3. **Infirmerie** (§6.3) : avis écrit du DPO sur la boîte N-0 / N-1 sans HDS.
4. **Avis juridique** sur la supervision par la direction (C. pén. 432-9) et sur la charte, avant tout pilote avec des élèves (déjà Q-M7).
5. **Défauts de Next.js** (§4.3) : NXD-1 (channels ouverts à tous) et NXD-2 / NXD-3 (pièces famille) sont en production ; couper ou restreindre `channels` est une décision et une action de Florian (Next.js intouchable pour le pilote).
6. **Usage réel** de la messagerie Next.js (Q-M23) : messagerie interne et visio utilisées chaque jour ? fils famille actifs ? — décide si la messagerie interne vise le niveau production à la rentrée 2027.
7. **Démo** : si M0b n'est pas prêt, retirer la messagerie de la démo de décembre.

---

## 11. Réglages signalés (D-023)

| # | Réglage | Recommandation |
|---|---|---|
| S-450 | Plafond du lot individuel par rôle | fixe : 30 ; PP : effectif de sa classe (≤ 40) |
| S-451 | Avertissement des termes injurieux | élèves et familles, jamais bloquant, non désactivable par personne |
| S-452 | Débit des familles (fils par jour, messages par fil) | fixe : 3 fils par jour vers une même personne, 20 messages par fil et par jour |
| S-453 | Réponse automatique de nuit des boîtes | fixe dans sa structure (numéros d'urgence), texte d'accueil libre |
| S-454 | Mentions, épinglage de messages, réactions | entre personnels seulement ; aucun réglage |
| S-455 | Plage calme des personnels (réception) | défaut identique à la plage d'envoi ; « recevoir quand même » personnel |
| S-456 | Langues des gabarits | celles que l'établissement déclare ; ajout par traduction humaine relue |

---

## 12. Faits, hypothèses, contradictions
**Faits [F]** : §3 (code M0a lu, lignes citées) ; §4 (Next.js lu sur `origin/main`, copie extraite, rien exécuté).
**Vérifié [V]** : §7 (sources citées) ; beaucoup de points sont [NV] dans cette passe faute d'accès au BO et à eduscol (403).
**Hypothèses [H]** : pratiques de terrain (cahier de liaison AESH, appels relayés, messages du dimanche, comptes partagés sur la tablette familiale), volumes, consignes de silence en PPMS (à vérifier sur le guide PPMS en vigueur).

**Contradictions relevées** :
- **C8** `16` §27.3 (« un fil ouvert par l'élève est lisible de tout personnel ayant `messagerie.eleve` », choix 2) contre R-M9 / §10.1 (« discrètement », MSG-128) : tranché par D21.
- **C9** `16` §27.1 (« avant tout pilote avec des élèves : M8 et M10 ») contre P10 (parent harcelant dès le premier usage réel) : M10 aussi avant tout usage réel avec des familles.
- **C10** `16` §5 R-17 et §14 point 4 (« art. 40 abrogé au 1ᵉʳ janvier 2029 ») contre l'ordonnance 2025-1091 (réécriture à droit constant, date reportable) : corrigé.
- **C11** `16` §8.2 (« jamais un fil famille ↔ famille ») contre C. éduc. D. 111-8 / D. 111-9 (associations de parents) : nuancé par la boîte des représentants.
- **C12** `16` §25 (JSONB, clé-valeur, tableaux) contre D-036 : tranché par §33.6.
- **C13** `TRANSITION_T0_INVENTAIRE.md` (« messagerie interne : 0 test ») : 2 fichiers de test (assistant, matrice famille), aucun sur les droits ; `channel-access.ts` n'est pas de la messagerie.
- **C14** `16` CM-76 (« chaque brouillon confirmé ») contre le bilan de 35 familles du PP : chaque brouillon relu, lot confirmé une fois.
