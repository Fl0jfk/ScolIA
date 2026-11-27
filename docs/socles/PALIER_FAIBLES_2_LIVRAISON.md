# Palier des faibles regroupés n° 2 — livraison

Palier de correctifs (règle 39), branche unique `claude/project-thread-gco1gz`, partie d'`origin/ENT` (`016636d9`) le
2026-11-27. Dernier point de la file du point quotidien du 27-11 qui se code sans conception (« palier des faibles
regroupés n° 2 »). **Aucune migration, aucune dépendance, aucune route de production** (une route de labo de plus,
`/mobile/labo/emplacement`, sous la feature `labo` et la garde `ENT_ENV`).

Hors palier, laissés aux paliers en cours pour ne pas chevaucher leurs fichiers : RGPD (PR 165 G3b + G4, PR 166 G5),
mobile (PR 164, saisie des notes). Les faibles des rapports V5 (extrait par élève) relèvent du RGPD et restent en file.

Rapports d'origine (sur `ent-mc0hoe`, `docs/pilotage/revues/`) : `42_VOYAGES_V3B_SUITES_2_CONTRE_REVUE.md`,
`11_MB1_SERVEUR_CONTRE_REVUE.md`, `16_E1_SUITES_5_CONTRE_REVUE.md`, et les « restent en file » de
`PALIER_FAIBLES_1_LIVRAISON.md`.

## Lot 1 — voyages (V3b suites 2)

| Point | Fait | Preuve |
|---|---|---|
| Faible de la contre-revue du palier 1 : le test M-2 attendait 300 ms | Plus de délai fixe : la seconde transaction tourne sur une connexion dont on connaît le pid ; le test attend (au plus 10 s) que `pg_stat_activity` la montre **en attente d'un verrou**, puis vérifie qu'elle n'a pas fini. | `faibles_l2_m2_revocation_et_usage_concurrents_en_sql_direct_un_seul_passe` |
| F-8 idempotence du rattrapage (656) sur `annulee_le` et les autres lignes | Deux sorties de plus dans l'établissement du test : une annulée sans `annulee_le` (rattrapée sur son `modifie_le` d'avant le passage), une annulée à l'instant déjà posé (jamais modifiée). Le second passage laisse **toutes** les sorties de l'établissement identiques (rentrée, annulation, modification). La contrainte `ent_sortie_annulee_le_chk` écartée pour le montage est rétablie. | `v3b2_le_rattrapage_restaure_le_reglage_de_l_etablissement_et_se_rejoue_sans_effet` |
| F-7 couverture HTTP de la révocation | Contournement réel par HTTP : la comptabilité, rendue parent d'un participant, demande ; la direction (libre) autorise ; la demandeuse reçoit 403, un autre établissement 404, l'autrice 200 (`revoque`), une seconde révocation 409. | `routes_d_argent_de_v3_repondent_201_202_403_404_422` (fin du test) |
| F-1 contournement et décision par un parent | Une famille qui tente de décider reçoit 404 (elle ne voit pas l'argent de la sortie). | même test |

Reste en file : F-2 (portée affichée), M-3 (révocation d'un collectif déjà utilisé pour les écritures à venir), frontière
exacte des 30 jours.

## Lot 2 — idempotence mobile, partie serveur (restes de MB1)

| Point | Fait | Preuve |
|---|---|---|
| F3 plafond d'attente par session | Au plus `ATTENTES_MAX_PAR_SESSION` (4) requêtes d'une même session attendent à la fois une clé en cours (compteur du processus, place rendue à la destruction : fin d'attente, réponse ou client qui coupe). Au-delà : `409 idempotence.en_cours` tout de suite, avec `Retry-After`. La place est rendue avant l'exécution quand l'attente aboutit à une reprise. | `tests_pilote::places_d_attente_plafonnees_par_session_et_rendues`, `faibles2_mb1_f3_attente_plafonnee_par_session` (6 requêtes derrière une exécution de 3 s : 4 rejouent, 2 refusées, une seule exécution) |
| Reste de M2 : délai de reprise supérieur au délai effectif de requête | L'exécution tourne dans une tâche détachée que `TimeoutLayer` n'arrête pas. Elle est désormais bornée à `IDEMPOTENCE_EXECUTION_MAX_S` (120 s, contrôlé à la compilation : strictement moins que `IDEMPOTENCE_ABANDON_S`, 150 s). Au-delà : exécution interrompue (sa transaction est annulée), clé close **non rejouable**, réponse `408 delai.depasse`. D-069 : jamais libérée, donc jamais une seconde exécution, même si l'écriture avait abouti juste avant la borne. La table n'admettant que 200 à 399 (2227), le statut stocké est 202 ; il n'est jamais servi (un rejeu reçoit 409). | `faibles2_mb1_execution_bornee_cle_close_non_rejouable` (borne raccourcie à 200 ms pour le seul établissement du test) |
| Test de reprise après abandon | Le client coupe pendant l'exécution ; la tâche détachée conserve la réponse ; le nouvel essai de même clé la rejoue, sans seconde exécution. | `faibles2_mb1_reprise_apres_abandon_du_client_rejoue_sans_reexecuter` |
| Test d'échec de conservation (valeurs hors bornes) | Route de labo dont la réponse porte un `Location` de plus de 500 caractères : réponse servie intacte, clé close non rejouable, rejeu en 409, une seule exécution. | `faibles2_mb1_reponse_hors_bornes_non_rejouable_sans_seconde_execution` |

Écart trouvé en écrivant le test de borne : un premier essai stockait le statut 408, refusé par la contrainte
`statut_http BETWEEN 200 AND 399` ; la clé restait « en cours » (rejeu en `409 en_cours` pendant 10 s). Corrigé (202).

Reste en file : HMAC de l'empreinte, écriture de la clé dans la transaction métier pour les écritures critiques.

## Lot 3 — messagerie (E1 passe 5)

| Point | Fait | Preuve |
|---|---|---|
| Faible 3, seconde partie : report calculé depuis le début du passage | Le nouveau `planifie_pour` se compte depuis l'instant réel du report : instant du passage + temps écoulé depuis le début du lot (horloge monotone, une durée, jamais une date). Les horodatages (`controle_echec_depuis`, `traite_le`) restent l'instant du passage (horloge injectée des tests). | `faibles_l3_*`, `tests_messagerie_p5` verts |
| Faible de la contre-revue du palier 1 : un avis du même message en échec ouvrait sa propre transaction de report | Les avis suivants d'un message déjà en échec dans la boucle sont mis de côté et reportés **ensemble**, en une transaction (`id = ANY`), à la fin de la boucle des individuels et de chaque groupe ; le premier échec reste reporté tout de suite. Un échec du report lui-même est journalisé, les avis restent `envoi` (repris comme envois interrompus), comme avant. | `faibles_l3_f2_un_message_verrouille_n_est_attendu_qu_une_fois_par_lot` (trois canaux, une attente, comptes du bilan inchangés) |

Reste en file : équivalence enveloppe / brut, exposition du code d'erreur du dépôt (E1 suites 4, F2).

## Écarté du palier : finances

Les faibles de la contre-revue du solde du régime (test F-5 par le vrai chemin de régime, test du décrément du plafond
libre avec `extra > 0` et payeur suivant déjà posé, arithmétique saturée) demandent des cas chiffrés à concevoir sur le
moteur de facturation. Laissés en file pour un lot financier, avec F-7 (conception).

## Contre-revue (lecture seule, `docs/pilotage/revues/PALIER_FAIBLES_2_CONTRE_REVUE.md` dans le dossier du Projet)

Verdict : versable après correctifs, aucun élevé.
- **M1** (messagerie) : les avis mis de côté n'étaient reportés qu'en fin de boucle ; une erreur d'envoi d'un avis suivant
  les laissait en `envoi`. Corrigé : report dès qu'on passe à un autre message, et en fin de boucle.
- **M2** (idempotence) : clôture de la clé après la borne en trois essais. Limites écrites dans `idempotence.rs` : un
  handler coupé après son propre `commit` l'est à mi-chemin ; trois échecs de clôture laissent la clé reprenable après
  150 s. Un jeton d'annulation vérifié avant le commit reste en file.
- **M3** (test F-8) : les contraintes écartées sont rétablies dès le premier passage, avant toute assertion.
- Faibles en file : test du plafond dépendant d'une exécution de 3 s ; tests d'abandon et de hors-bornes de non-régression
  seulement ; 404 de la famille non distinctif.

## Compteurs (règle 28)

Aucune action (`Action::TOUTES` inchangé), aucune table, aucune migration (`migrate.rs` et `MIGRATION_TAG` inchangés),
aucune route de production, aucune source. Une route de labo (`/mobile/labo/emplacement`), couverte par le motif
`/mobile/labo` déjà interdit au Dockerfile et en CI.
