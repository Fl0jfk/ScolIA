# Palier des faibles regroupés n° 1 — livraison

Palier de correctifs (règle 39), branche unique `claude/project-thread-98w8lf`, partie d'`origin/ENT` (`5022055a`) le
2026-11-27. N° 5 de la file du point quotidien du 26-11 (« lots de faibles regroupés »). Quatre lots, une contre-revue par
lot, une seule PR en brouillon et une seule CI à la fin. Deux migrations : **658** (lot 2) et **495** (lot 3, après la contre-revue). Aucune dépendance nouvelle.

Rapports d'origine (sur `ent-mc0hoe`, `docs/pilotage/revues/`) : `11_MB1_SERVEUR_CONTRE_REVUE.md`,
`42_VOYAGES_V3B_SUITES_2_CONTRE_REVUE.md`, `16_E1_SUITES_5_CONTRE_REVUE.md`, `T3_FIABILITE_CONTRE_REVUE.md`,
`20_CAHIER_SUITES_2_CONTRE_REVUE.md`, `03_I5_SUITES_7_CONTRE_REVUE.md`.

Hors palier, laissés aux paliers en cours : RGPD (palier G3b + G4) et facturation (solde du régime mois par mois, versé).

## Lot 1 — mobile, partie serveur (MB1)

| Faible | Fait | Preuve |
|---|---|---|
| F1 messages 400/409/422/503 relayés | L'enveloppe remplace par le message neutre du code tout texte qui a l'air technique (fragment SQL, contrainte, SQLSTATE, panique, chemin de code : `enveloppe::texte_technique`) ; tout 5xx, 503 compris, a son message neutre. | `enveloppe::tests_f1` ; `tests_mobile_mb1::mb1_f1_texte_technique_jamais_relaye_et_5xx_non_conserve` (400, 409, 503 avec un texte de base, clé non conservée sur 5xx). |
| F2 rejeu à l'extérieur des gardes santé et élève | Documenté (`idempotence.rs`, en tête) : seule la même session rejoue sa propre réponse de succès, 24 h, jamais une réponse sensible. Pas d'élévation. | — |
| F3 attente en boucle | Documenté (200 lectures au plus, 10 s) ; plafond par session en file. | — |
| F4 retrait de `x-scolia-code` | Test sur l'application complète : une couche intérieure (idempotence) pose l'en-tête, le web ne le reçoit jamais et reçoit du texte. | `mb1_f4_en_tete_interne_jamais_servi_au_web` |
| F5 `/mobile/labo` | Les routes de labo répondent 404 hors `ENT_ENV` non productif, même dans un binaire de labo (`garde_env`). Le motif `/mobile/labo` est ajouté aux contrôles de séparation du Dockerfile et de la CI. | contrôle L4 de la CI |
| F6 corps lu en mémoire | Documenté : borné par `max_body_bytes()`, comme `RequestBodyLimitLayer`. | — |
| F7 purge en `Utc::now()` | `purge::une_passe_a(ent, maintenant)` ; `une_passe` l'appelle à l'heure réelle (tâche périodique). Le test de purge passe par la passe complète à l'instant injecté. | `mb1_idempotence_purge_a_24_heures_horloge_injectee` |
| Tests manquants | Web avec clé (rejoué), requête dans l'empreinte (`?x=1` : conflit), autre établissement (rien de rejoué), 5xx non conservé. | `mb1_web_avec_cle_requete_dans_l_empreinte_et_autre_etablissement` |

Restent en file : HMAC de l'empreinte, clé écrite dans la transaction métier, plafond de l'attente par session, tests de
reprise après abandon et d'échec de conservation.

## Lot 2 — voyages, V3b suites 2

| Point | Fait | Preuve |
|---|---|---|
| M-2 course révocation / usage en SQL direct | **658** : la garde des faits de contournement prend le verrou consultatif de la sortie (même clé que `verrou_sortie`) avant de lire les faits. Une révocation et un usage concurrents ne passent plus tous les deux. | `faibles_l2_m2_revocation_et_usage_concurrents_en_sql_direct_un_seul_passe` : rouge avec la fonction de 657, vert avec 658 (vérifié en local). |
| F-3, F-5 messages SQL non traduits | `db_pouvoir` : un refus de cycle de vie venu de la base (déjà utilisé, révoqué, expiré, non autorisé, décision trop tardive) devient un **conflit** (409, `MSG_CONTOURNEMENT_ETAT`) au lieu d'un refus de la règle 3 ; un second avoir collectif aussi (`MSG_AVOIR_COLLECTIF_BORNE`) ; la révocation par une autre personne reste une interdiction. Un refus de portée reste celui de la règle 3. | `faibles_l2_f3_f5_refus_de_la_base_traduits_en_refus_metier` |
| F-4 révocation d'une demande expirée jamais autorisée | Conflit (409), plus un refus de personne (403). | `faibles_l2_f4_f6_revocation_d_une_demande_expiree_et_refus_trace` |
| F-6 refus de révocation non tracé | La révocation passe par `tracer_refus_regle3` ; le refus « seule l'autrice révoque » est dans `est_refus_a_tracer`. | même test |
| Précédent de M-4 (test de 656) | Le test du rattrapage de 656 ne rejoue plus 656 puis 657 sur la base commune : il ne remet que les deux contraintes écartées. Sans cela, depuis 658, il remettait durablement la garde sans verrou. | `v3b_le_rattrapage_de_656_atteint_les_lignes_sous_la_rls_stricte` |

Restent en file : F-1 (contournement et décision par un parent : déjà couvert par `avant_verrou`, test dédié à écrire),
F-2 (portée affichée), F-7 (couverture HTTP de la révocation), F-8 (idempotence du rattrapage), révocation d'un collectif
utilisé pour les écritures à venir (M-3).

Règle prudente (D-069) : un refus de cycle de vie venu de la base est une course ; il n'est pas tracé comme un refus de la
règle 3, puisque la personne n'a rien tenté d'interdit.

## Lot 3 — messagerie (E1 passe 5) et fiabilité (T3)

| Point | Fait | Preuve |
|---|---|---|
| E1 faible 2 : 55P03 non mémorisé | Un message dont le contrôle d'existence a échoué dans le lot n'est plus réattendu : les avis suivants du même message sont reportés aussitôt (`ControleMessage::en_echec`). | `faibles_l3_f2_un_message_verrouille_n_est_attendu_qu_une_fois_par_lot` (trois canaux, une seule attente). |
| E1 faible 3 : report sans fin | D-069 : un avis dont le contrôle échoue encore 24 h après son **premier** échec de contrôle passe en `echec`, motif `controle_existence_impossible`, jamais renvoyé (`REPORT_CONTROLE_MAX`). **495** : colonne `controle_echec_depuis`, posée au premier échec, jamais avancée. Ni `created_at` (un avis planifié loin devant serait perdu au premier échec), ni `planifie_pour` (avance à chaque report). | `faibles_l3_f3_un_avis_en_echec_de_controle_depuis_24_heures_passe_en_echec` (vieux échec : `echec` ; avis créé il y a 36 h, premier échec maintenant : reste en attente) |
| E1 faible 5 : limiteur | Le limiteur compte les lignes écartées par cause et rend le compte avec la ligne suivante (`autoriser_global_compte`) ; `signaler_echec_etablissement` s'en sert. Le contrôle d'existence a une cause par classe d'erreur (base, autre). | `journal_limite::les_lignes_ecartees_sont_comptees_par_cause_et_rendues_avec_la_suivante` |
| T3 : exemption par extrait de ligne | Les trois exceptions de `controle_demarrage` exigent un nombre exact d'occurrences (`OCCURRENCES_EXACTES`). | `garde_rls_stricte::aucun_acces_direct_au_pool_hors_exception` |
| T3 : commentaire inexact | Corrigé (`tests_internat_i4_suites3.rs`) : chaque banc a son propre établissement. | — |
| T3 : constante « report < 5 min » | Vérifiée dans le test E1 (le `planifie_pour` reporté est lu). | `p5_e1_une_annulation_validee_apres_le_report_ecarte_l_avis_avec_message_annule` |

Restent en file : report calculé depuis le début du passage (E1 faible 3, seconde partie), équivalence enveloppe / brut
déjà testée par T3, assertion `Accorde` du point d'entrée (suppose un labo connecté en propriétaire).

## Lot 4 — cahier de textes (suites 2) et inscriptions (I5 passe 7)

| Point | Fait | Preuve |
|---|---|---|
| Cahier M2 : 2420 recrée une fonction déjà recréée par 2419 | Les tests qui rejouent tout (`rejouer_liste`) rejouent 2419 et 2420 **d'un bloc**, dans une transaction (`REJEUX_GROUPES`, points de reprise pour les échecs tolérés). Les autres sessions ne voient plus la version de 2419 servir les tables de 2420. Production et base neuve : inchangées. | suites qui rejouent tout (`tests_d036`, `tests_inscriptions_i2`…) |
| Verrou consultatif 7770042 non libéré sur panique | Le test du REVOKE prend le verrou sur une connexion détachée : une panique ferme la connexion et le serveur libère le verrou. | `c8_suites2_reprise_retiree_a_tout_role_sauf_au_proprietaire` |
| `verifier_gel` non discriminant | Le refus doit venir du gel (message R-C28). Sur une table en ajout seul, UPDATE et DELETE sont refusés d'abord par l'ajout seul : la preuve passe alors par le catalogue (déclencheur de gel actif sur UPDATE et DELETE). | `c8_gel_*` |
| I5 : commentaire du test de santé | 15 s, pas 3 s. | — |

Restent en file : séparation propriétaire / rôle applicatif à écrire comme condition d'exploitation (déploiement), vraie
fusion testée avec une année close, extrait RGPD d'une ligne restée sur l'élève absorbé (à vérifier avec le palier G3b).
I5 : offre « d'avant » lue dans l'état courant, restauration non couverte en cas de panique.

## Compteurs (règle 28)

Aucune action ajoutée (`Action::TOUTES` inchangé), aucune table (compte RLS inchangé), aucune route, aucune source.
`migrate.rs` : +2 (658 puis 495, en fin de tableau). Aucune table : 495 ajoute une colonne à `ent_notification_envoi`. `MIGRATION_TAG` inchangé (3808).

## Contre-revue (une par lot, lecture seule)

| Lot | Verdict | Points relevés et suite donnée |
|---|---|---|
| 1 | Après correctifs | `texte_technique` trop large (« contrainte » et `ent_` partout attrapaient des messages métier) : marqueurs ancrés (`constraint "`, `contrainte "`, `contrainte « `), préfixes `ent_` et `pg_` seulement en début de mot ; cas négatifs avec de vrais messages métier dans `tests_f1`. Un 503 garde son texte s'il passe le filtre (« Opération concurrente, veuillez réessayer. ») ; le texte technique y est toujours remplacé. Assertion fragile `bilan.echecs == 0` retirée du test de purge. |
| 2 | Versable | Le message « un contournement non autorisé ne se révoque pas » devient un conflit ; « … ne s'utilise pas » reste un refus de la règle 3, tracé. Avoir collectif séparé : « payeur déjà couvert » = conflit (`MSG_AVOIR_COLLECTIF_BORNE`), « montant supérieur à l'argent versé » = invalide (`MSG_AVOIR_COLLECTIF_MONTANT`). |
| 3 | Après correctifs | 24 h comptées depuis `created_at` : remplacé par `controle_echec_depuis` (495). |
| 4 | Versable | `verifier_gel` vérifié vert sur les tables en ajout seul (`c8_gel_*`). |

Faibles de la contre-revue laissés en file : le test M-2 attend 300 ms pour constater le blocage (pas d'horloge, mais un délai) ;
branche de préfixe ancienne de `db_pouvoir` presque morte ; le groupe `REJEUX_GROUPES` n'est rejoué que si 2419 est dans la
liste (aujourd'hui toujours vrai) ; un avis du même message en échec ouvre encore sa propre transaction de report ; les tests
MB1 du labo dépendent d'`ENT_ENV` non productif (vrai en CI).

## Vérifications locales

Voir la section « Tests locaux » ci-dessous, remplie en fin de palier.
