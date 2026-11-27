# Contre-revue — socle 27, palier G3b + G4, lots 1 et 2 (extrait d'accès, marquage, relecture) — 2026-11-26

Branche `ent-socle-27-palier-g3b-g4`, aucune PR (règle 39 : une CI pour le palier). Code relu à **`e2c517e43`** (tête du
lot 2). Commits relus : `4e49258ae` (2544 versée), `258ae1ef8`, `67392f4dd`, `41c094bad` (lot 1), `e2c517e43` (lot 2),
soit `git diff e7e33d498...e2c517e43` (26 fichiers, +6737 / −50).

Relecteur indépendant, lecture seule : rien n'a été compilé ni exécuté, aucune CI déclenchée, aucun fichier de code
touché (un autre fil code le lot 3 sur la même branche). Les chemins `fichier:ligne` sont ceux de `e2c517e43`, sous
`ent-rust/crates/ent-db/src/` sauf mention. Le lot 1 n'avait eu qu'une auto-revue : il est relu ici en entier.

## Verdict : **non versable en l'état** — trois élevés, tous sur l'isolation entre titulaires

La migration 2545 est propre et le cadre (gardes, RLS, refus de lecture d'un extrait périmé, trace avant service, DPO
limité à l'extrait soumis) est solide. Mais l'exemple de référence M-2711 n'est pas tenu dans deux cas réalistes (E1,
E2), et l'ajout d'un sujet de fratrie contourne la vérification de qualité (E3). Le marquage repose sur une liste
« autres titulaires » qui ignore les responsables sans compte, alors que Pf3 a retiré le garde-fou qui bloquait le
titulaire quand l'autre parent est protégé : les deux changements ensemble ouvrent une fuite qui n'existait pas en G3a.
Les élevés sont à corriger avant le lot 4 (remise), avec un jeu d'essai M-2711 dédié.

## Contrôles de cadre

- **Règle 25** : les 5 commits ne touchent ni `docs/POINT_QUOTIDIEN.md` ni `docs/PILOTAGE.md`. `migrations_versees.txt`
  ne reçoit que `2544_rgpd_g3_demandes` (commit à part `4e49258ae`, ligne 445) ; 2545 n'y est pas.
- **Migration 2545** : `m!("2545_rgpd_g3b_extrait")` en **fin** de tableau après 2544 (`migrate.rs:614`, règle 38) ;
  `MIGRATION_TAG` inchangé. Les 3 fonctions (`rgp_reglage_journal_ajout_seul`, `rgp_extrait_section_garde`,
  `rgp_extrait_garde`) portent `SET search_path = pg_catalog, public, pg_temp` et ne lisent aucune relation. Rejouable :
  `IF NOT EXISTS`, `DROP TRIGGER IF EXISTS` puis `CREATE`, FK posée sous `DO $$ … IF NOT EXISTS`, contrainte du journal
  supprimée puis recréée (le nom implicite `rgp_demande_journal_evenement_check` de 2544 correspond bien, l'ensemble
  est élargi sans valeur retirée). Pas de JSON, tableau ni BYTEA en colonne (D-036). La modification de 2545 au lot 2
  (+36 lignes, table `rgp_extrait_masquage`) est admissible puisque 2545 n'est pas versée.
- **RLS** : `ent_rls_poser` sur les 6 tables ; compteur `tests_rls.rs` +6 ; FK composites `(etablissement_id, …)`
  partout. Toutes les requêtes du moteur portent `etablissement_id = p.etab` en plus de la RLS (`lire_table`, sauf
  `ent_user`, `sans_etab`, filtré par `x.id = p.compte`).
- **SQL dynamique** : tables et colonnes viennent du registre ou de littéraux, passent par `identifiant_sur` ; les
  valeurs sont liées (`$1…$9`). Aucune entrée libre interpolée. `droits_masquage_ajouter` valide ses identifiants
  avant stockage, jamais interpolés.
- **Contenu jamais stocké** : vérifié, seules structure, empreintes et décisions sont écrites ; la lecture reconstruit
  et compare l'empreinte (`droits_extrait.rs:1957`), refus 409 et périmé sinon.
- **Trace avant service** (R-RG8) : `tracer_consultation_a` avant la reconstruction (`droits_extrait.rs:1929-1940`).
- Aucun `#[ignore]`, aucune heure réelle dans les tests relus.

## Élevés

### E1 — Les responsables sans compte ne sont jamais « autres » : leurs données et leur protection sortent dans l'extrait du titulaire
`droits_extrait_masquage.rs:356-365` (`faits_eleve`). La liste des autres titulaires part de
`SELECT l.user_id, p.id FROM ent_eleve_lien l LEFT JOIN ent_personne p ON … p.user_id = l.user_id`, décodée en
`(Uuid, Option<Uuid>)`. Or depuis la migration 120 `ent_eleve_lien.user_id` est **facultatif** et `personne_id` est le
rattachement obligatoire (`120_personne.sql:47, 145`). Trois conséquences :

1. Un lien sans compte (parent qui n'a jamais activé l'ENT, cas courant) rend `user_id` NULL : le décodage en `Uuid`
   échoue (`UnexpectedNull`) et **l'assemblage entier tombe en erreur** pour tout élève qui a un tel responsable
   (sauf si ce lien est celui du demandeur sans compte, exclu par `IS DISTINCT FROM NULL` = faux).
2. Si l'on corrige naïvement le décodage (`Option<Uuid>`), l'autre parent sans compte n'entre **pas** dans
   `autres` (la jointure ne suit pas `l.personne_id`) : aucune ligne qui le désigne n'est marquée, et sa protection
   n'est jamais vue (`proteges` est calculé sur `autres`).
3. Les lignes qui décrivent l'autre parent **sans colonne d'identifiant** ne sont jamais marquées :
   `ent_inscription_responsable.personne_id` est facultatif (`141_inscription_dossier.sql:72`, `ON DELETE SET NULL`) ;
   un responsable saisi au dossier d'inscription sans fiche rapprochée garde nom, téléphone et adresse en clair dans
   l'extrait. Même chose pour les colonnes qui désignent un parent sous un autre nom que ceux de
   `COLONNES_PERSONNE` (`:75`) : `cree_par`, `saisi_par`, `depose_par`, `demandeur_user_id`, `signataire_*`…

Avec **Pf3** (`droits_demandes.rs:834-844`), la protection d'un autre parent ne bloque plus le titulaire : elle est
censée être traitée par le marquage. Pour un autre parent protégé et sans compte, le père reçoit donc l'adresse
protégée de la mère dans `ent_inscription_responsable` et les indices de localisation de l'élève, ce que G3a
empêchait. Le test `tests_rgpd_g3b2.rs` ne couvre que l'autre parent **avec** compte et `personne_id` renseigné.
**Correctif** : construire `autres` depuis `l.personne_id` (et le compte de cette personne s'il existe), y ajouter les
`personne_id` des responsables de dossier et des participants de fil famille ; marquer par défaut toute ligne de
`ent_inscription_responsable` (et tables de responsables) qui n'est pas celle du demandeur, identifiant présent ou non ;
tests avec un autre parent sans compte, protégé et non protégé.

### E2 — M-2711 : un titulaire sorti d'un fil reçoit tout le fil, y compris les échanges ultérieurs de l'autre parent avec le CPE
`droits_extrait.rs:428-443` (`fils_de_messagerie`). Le filtre retient un fil dès qu'il existe **une** ligne
`ent_msg_participant` du demandeur, sans regarder `du`, `au` ni `suspendu_le`, puis rend **tous** les messages du fil
(`regles_messagerie`, `x.fil_id IN {f}`). Or la messagerie fait sortir un parent d'un fil par `au` avec un
`motif_sortie` comme `separation`, `interdiction` ou `lien_clos` (`482_msg_m1_participants.sql:160-166`), et tout le code
de lecture de la messagerie exige `p.au IS NULL` (`msg_repo.rs:652, 703, 731`, `msg_m2.rs:462`, `msg_m3.rs`). Cas
concret : le père est retiré du fil « vie scolaire de Léa » à la séparation ; la mère continue d'y écrire au CPE ; la
demande d'accès du père lui rend ces messages, qu'il n'a jamais pu lire. Même défaut pour un participant ajouté tard
(`du`) qui reçoit l'historique antérieur, et pour le sujet « personne » (`:440`). Le marquage ne rattrape rien : dans
un fil de messagerie, seul l'auteur est masqué (`droits_extrait_masquage.rs:225-227`), le corps reste.
**Correctif** : borner chaque message à la période de participation du demandeur
(`m.created_at >= pa.du AND (pa.au IS NULL OR m.created_at < pa.au)`, `suspendu_le` compris), appliquer la même
borne à `ent_msg_version`, `ent_msg_piece`, `ent_msg_participant` ; jeu d'essai M-2711 dédié (fil mère–CPE sans le
père ; fil commun dont le père sort ; fil où il entre tard), que la livraison signale elle-même comme non couvert.

### E3 — Ajouter un sujet à une demande « pour soi » contourne la vérification de qualité et ouvre l'extrait complet d'un autre élève
`droits_extrait.rs:1557-1566` (`droits_sujet_ajouter`) appelle `qualite_a_la_date` avec la qualité **de la demande** ;
pour une qualité « pour soi » (`soi_meme`, `eleve_mineur`, `personnel`, `ancien_eleve`), celle-ci rend `SansObjet`
**sans regarder le sujet** (`droits_demandes.rs:777-779`). Le sujet ajouté est enregistré `sans_objet`, que
l'assemblage accepte sans arbitrage (`droits_extrait.rs:1672`). Pire, `dem_est_sujet` vaut vrai pour **tout** sujet
élève d'une demande « pour soi » (`:1424`) : la messagerie de l'élève ajouté sort en entier
(`p.dem_est_sujet AND pa.eleve_id = p.sujet`, `:434`) et le marquage applique R-RG48 comme si l'élève ajouté était le
demandeur. Il suffit d'une erreur de saisie au guichet (le frère au lieu de la sœur, ou une « fratrie » ajoutée à la
demande d'un élève majeur) pour assembler l'extrait complet d'un autre élève sans qu'aucune qualité ait été constatée.
`verifier_sujet` (`droits_demandes.rs:3083`) ne contrôle que l'existence dans l'établissement. **Correctif** : pour une
qualité « pour soi », refuser tout sujet ajouté (une fratrie n'a de sens que pour un titulaire), ou constater la
qualité du demandeur sur ce sujet avec la grille « responsable » ; calculer `dem_est_sujet` en comparant le sujet au
dossier d'élève du compte demandeur, pas à la seule qualité. Test à ajouter.

## Moyens

- **M1. Les masquages manuels disparaissent à chaque nouvel assemblage** (`droits_extrait_masquage.rs:418-460`).
  `persister_marquages` n'insère que les marquages calculés et ne reporte que les décisions `propose_par = 'sys'`
  (`:444`). Un masquage ajouté à la main (RGP-151, décidé `masque` d'emblée) n'est donc **pas** recopié sur la
  version suivante ; comme il n'était pas « à décider », rien ne bloque la remise de la nouvelle version, qui contient
  l'élément que l'agent avait retiré. Une simple modification d'une donnée de l'élève (qui périme l'extrait) suffit.
  Correctif : reporter les masquages `agent` sur la nouvelle version (ou les recréer `a_decider`), test.
- **M2. La décision reportée suit l'emplacement, pas le contenu** (`:430-446`). Le report compare table, `ligne_id`,
  `rang`, colonne et raison, jamais l'empreinte de l'élément. Une levée motivée sur l'ancien téléphone de l'autre
  parent est reportée sur le nouveau. Pour une ligne **sans identifiant**, seul le `rang` désigne l'élément : une ligne
  insérée avant décale les rangs et la levée passe à une autre ligne. La livraison annonce un report « quand l'élément
  n'a pas changé », ce que le code ne vérifie pas. Correctif : ne reporter que si l'empreinte de la ligne (ou de la
  cellule) est identique, ne jamais reporter une levée sur une ligne sans identifiant.
- **M3. Une protection posée après l'assemblage ne périme pas l'extrait.** L'empreinte (`empreinte_section`) porte sur
  le contenu, pas sur les faits du marquage (`faits_eleve`). Si une adresse protégée est posée pour la mère entre
  l'assemblage et la remise (cas typique d'un conflit en cours), l'extrait reste valide avec ses anciens marquages : les
  indices de localisation ne sont pas marqués. À traiter avant le lot 4 : recalculer les faits (ou leur empreinte) à la
  lecture et à la remise, et périmer si les marquages calculés diffèrent.
- **M4. R-RG46 : le secrétariat apprend qu'une protection existe et qui elle vise** (`droits_extrait_masquage.rs:331`,
  `droits_extrait.rs:1997`). `a_arbitrer` n'est l'alias que d'`adresse_protegee` : il est donc équivalent au code. La
  vue des marquages transmet aussi `table`, `ligne_id`, `colonne` (`ent-api/src/rgpd_extrait.rs:94-109`), et le contenu
  affiche `[réservé à la direction]` à l'endroit exact ; un marquage de ligne entière sur
  `ent_inscription_responsable` désigne la personne protégée. Même remarque, plus faible, pour la vue de la demande
  (`epurer_pour_traiteur`, `droits_demandes.rs:2989` : `a_arbitrer` ne remplace que `restreinte` et
  `confirmee_sans_autorite`). Correctif : donner au porteur de `droits.traiter` seul une vue où les éléments protégés
  sont **absents** (ni marquage listé ni emplacement), avec un compteur global « éléments réservés à la direction »
  posé quel que soit le dossier, ou faire porter le même alias à d'autres raisons réservées.
- **M5. L'ajout manuel ne masque pas ce qu'il annonce** (`:695-760`, `remplacer` `:276-297`). Il est enregistré avec
  `rang = 0` ; sans `ligne_id`, `remplacer` ne masque que la **première ligne** de la table (`_ => m.rang == rang`) alors
  que l'écran et la doc parlent d'« une table, une ligne ou une colonne ». Avec un `ligne_id` sur une table dont les
  lignes n'ont pas d'identifiant, c'est aussi le rang 0 qui est masqué. Aucun contrôle que table, ligne ou colonne
  existent dans l'extrait : une faute de frappe crée un masquage sans effet, affiché comme « masqué ». Correctif :
  `rang` nullable (NULL = toutes les lignes), validation de l'élément contre le contenu reconstruit.
- **M6. Aucun marquage pour un sujet « personne » ou « agent »** (`droits_extrait.rs:1684-1696`). R-RG45 (pièce du
  coffre qui concerne un tiers, jamais remise) n'est appliqué qu'aux élèves, alors que la section `documents` de la
  personne rend `ent_doc_piece` (`:676-689`). Une pièce déposée pour le foyer (livret de famille, jugement) part dans
  l'extrait d'un parent. Correctif : appliquer au moins `piece_tiers` et `autre_eleve` aux sujets personne.

## Faibles

- **F1.** `rgp_extrait_masquage` n'a ni garde ni unicité (migration 2545, dernière table) : suppression possible au
  niveau base, colonnes de structure modifiables, une décision peut être réécrite sans que l'ancienne soit gardée
  ailleurs que dans le journal neutre. Ajouter une garde sur le modèle de `rgp_extrait_section_garde` et un index
  unique sur l'élément.
- **F2.** `rgp_extrait_garde` ne fige pas `periode_du`, `periode_au`, `assemble_par_compte_id`, et n'empêche pas de
  remettre `perime_le` à NULL (« dépérimer » un extrait).
- **F3.** `ent_msg_telephone`, `ent_msg_lien_decision`, `ent_msg_lecture`, `ent_msg_fil_eleve`, `ent_msg_brouillon`
  sont écartées (`TABLES_MESSAGERIE`, `:395-407`) sans règle ni ligne « non couverte » : l'extrait se dit complet sans
  les notes d'appel téléphonique du demandeur. Les exclure prudemment est juste, mais les déclarer.
- **F4.** `GET …/contenu` écrit (trace, passage à périmé, journal) : un GET à effet de bord ; acceptable pour la trace,
  à garder en tête pour le cache et la protection CSRF.
- **F5.** Une décision de marquage prise après l'avis du DPO n'efface pas cet avis, alors qu'une nouvelle soumission le
  fait : l'avis « favorable » peut porter sur une relecture qui a changé.
- **F6.** DPO sans `droits.traiter` : 403 pour un extrait existant non soumis, 404 pour un identifiant inconnu
  (`droits_extrait.rs:1922-1925`). Oracle d'existence étroit (identifiants aléatoires), à aligner sur 404.
- **F7.** `autre_eleve` ne masque que la colonne d'identifiant (`:217-219`) : nom, classe ou texte libre d'un autre élève
  dans un fait collectif restent. Signalé pour le lot de relecture, la levée étant de toute façon humaine.
- **F8.** Pf6 : la reprise sous `FOR UPDATE SKIP LOCKED` (`droits_demandes.rs`, passe de destruction) arrive après la
  suppression des objets ; deux passes peuvent donc supprimer deux fois (idempotent) mais n'écrivent qu'un
  `piece_detruite`. Conforme à l'intention.

## Correctifs G3a du lot 1

| Réf. | État | Preuve |
|---|---|---|
| Pf1 plafond réglable, verrou | corrigé | `pg_advisory_xact_lock` puis lecture de `rgp_reglage`, test labo |
| Pf3 qualité restreinte au seul demandeur protégé | corrigé, **mais** dépend du marquage, voir E1 | `droits_demandes.rs:834-844` |
| Pf4 vue épurée pour `droits.traiter` | corrigé pour les codes, alias trop parlant (M4) | `droits_demandes.rs:2989-3030`, toutes les entrées passent par `epurer_vue` |
| Pf5 numéro à la vérification réussie | acté et testé | livraison, test labo |
| Pf6 verrou de destruction, avis unique, refus sans identité | corrigé (F8) | `SKIP LOCKED`, `cle_dedup`, `droits_refuser` |
| Pf8 horloge injectée | corrigé | `accuser_reception(.., maintenant)`, `droits_alerter_echeances_a` |
| M-a pièce déplacée sur la nouvelle vérification | corrigé | `valider_piece(.., demande)` |
| Motif de prolongation hors journal | corrigé | texte neutre dans le journal |

## Isolation, refus neutres, ce qui tient

- Cloisonnement entre établissements : rien trouvé ; sujet d'un autre établissement → 404 (`verifier_sujet`).
- Protections jamais lues ni nommées : `TABLES_JAMAIS`, `COLONNES_JAMAIS`, et `faits_eleve` ne lit que des
  identifiants. Champs scellés jamais déchiffrés, IBAN masqué.
- Le DPO n'ouvre que l'extrait soumis ; l'avis exige la soumission ; l'arbitrage d'incomplétude exige
  `droits.arbitrer` et un motif par section ; un extrait incomplet n'est pas « remissible » sans ces motifs.
- Levée d'une raison réservée par `droits.traiter` seul → 404, identique à un marquage inexistant.
- M-2711 dans le cas simple (fil mère–CPE où le père n'a jamais été participant) : tenu par la requête, mais non testé.

## Limites de cette revue

Lecture seule à `e2c517e43`, sans compilation ni exécution : E1 point 1 (échec de décodage) est déduit du type Rust et
du schéma, pas observé. L'écran Leptos a été lu aux points sensibles seulement (libellé des raisons, affichage des
marquages). Le lot 3 (CSV) n'est pas couvert.
