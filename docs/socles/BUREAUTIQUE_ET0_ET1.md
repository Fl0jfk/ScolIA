# Socle 32 — rapport des lots ET0 et ET1 (code) : espace documents des personnels

**Date :** 2026-11-17 — **Branche :** `ent-socle-32-code-et01` (partie de `origin/ENT` c622d696 ; rien poussé sur `ENT`).
**Conception :** `docs/socles/32_BUREAUTIQUE.md` (sur `origin/ent-mc0hoe`) : §17.1, §17.2, §17.4, §17.7, §18, §19, §24, §27.1 et §30.
**Niveau atteint :** *implémenté et testé* (local ; CI : voir §9). **Non validé** par Florian, **pas prêt pilote**, rien déployé, **aucune donnée de production touchée**, aucun accès au S3 réel. ET0 et ET1 posent la **base** qui remplacera la fonction M19 de Next.js ; la **reprise des fichiers** (ET14) n'est pas faite : tant qu'elle ne l'est pas, M19 reste en production.

## 1. Ce qui est livré

| Lot | Contenu | Migrations | Tests |
|---|---|---|---|
| **ET0** (préalables du noyau) | usage de dépôt `Espace` dans `ent-object` (liste blanche §12.1, plafond 64 Mio) ; **URL d'envoi à taille signée** (SigV4 `content-length` signé en S3, jeton local lié à la taille) ; **réservation atomique de quota** (compteur verrouillé, 15 minutes) ; **réconciliation** de la taille réelle (relue chez le fournisseur) et de l'empreinte (calculée par le serveur) ; **analyse fermée par défaut** (adaptateur `AnalyseurAntivirus`) ; tâches de fond (expiration, purge, élagage, réconciliation) | 2640 | `tests_espaces_et0` (9 tests), `ent-object` (9 tests d'envoi et d'usage), `espaces::tests` (9 tests purs) |
| **ET1** (espace personnel et espaces libres, parité M19) | espaces `personnel` et `libre_personnels` ; membres (ajout, retrait, départ, transfert) ; dossiers ; fichiers à dépôt direct ; **versions immuables**, retour arrière, élagage ; téléchargement et aperçu minimal par lien court de 60 s ; corbeille de 30 jours et purge ; partages nominatifs datés ; « partagés avec moi » ; avis d'invitation et badge « non vu » ; quota ; journal en ajout seul ; routes `ent-api` ; tâche de fond | 2641, 2642, 2643 | `tests_espaces_et1` (15 tests), `garde_espaces` (6 gardes), matrice de séparation (domaine `espaces`) |

Architecture : `ent-db/src/espaces.rs` (types, bornes, fonctions pures), `espaces_quota.rs` (ET0), `espaces_service.rs` (espaces, membres, invitations, quota, journal), `espaces_contenu.rs` (dossiers, dépôt, versions, corbeille), `espaces_partages.rs`, `espaces_taches.rs` ; `ent-api/src/espaces.rs` (routes, traduction HTTP seulement). Aucune dépendance nouvelle (D-062).

## 2. Parité M19 : NX32-01 à NX32-20, action par action

| NX32 | Action | ETR | Test |
|---|---|---|---|
| 01 | Parcourir son espace personnel par dossiers | 110 | `et1_mon_espace_idempotent_et_lectures_sans_ecriture`, `et1_arborescence_un_depot_par_fichier_bilan_nx32_05` |
| 02 | « Fichiers partagés » (reçus) | 131 | `et1_partages_nominatifs_nx32_14_a_16`, `et1_partage_destinataires_resolus_et_dossier_recursif` |
| 03 | Créer un dossier | 112 | `et1_dossiers_noms_profondeur_deplacement` |
| 04 | Téléverser par URL d'envoi directe (taille signée) | 114 | `et0_envoi_de_70_mio_signe_1_mio_est_refuse`, `et0_vingt_envois_paralleles_ne_depassent_pas_le_quota` |
| 05 | Déposer une arborescence (un dépôt par fichier, bilan des refus) | 115 | `et1_arborescence_un_depot_par_fichier_bilan_nx32_05` |
| 06 | Télécharger (nom forcé) | 118 | `et1_telechargement_lien_court_lie_au_compte_nx32_06` |
| 07 | Déplacer fichier ou dossier ; refus si le nom existe, ou dans soi-même | 120 | `et1_dossiers_noms_profondeur_deplacement`, `et1_deplacer_renommer_sans_toucher_l_objet_et_partage_qui_suit` |
| 08, 09 | Supprimer (corbeille) un fichier, un dossier avec confirmation | 122, 123 | `et1_corbeille_apercu_restauration_purge_figee` |
| 10 | Créer un dossier partagé nommé avec des collègues (espace libre) | 106, 150 | `et1_membres_resolus_par_le_coeur_nd32_03`, `et1_mon_espace_idempotent_et_lectures_sans_ecriture` |
| 11 | Modifier les membres | 153 | `et1_retirer_quitter_transferer_dernier_animateur` |
| 12 | Lister ses partages (par type, activité) | 111 | `et1_mon_espace_idempotent_et_lectures_sans_ecriture` |
| 13 | Quitter un espace partagé | 155 | `et1_retirer_quitter_transferer_dernier_animateur` |
| 14, 15 | Partager un fichier, modifier les destinataires | 130 | `et1_partages_nominatifs_nx32_14_a_16`, `et1_partage_destinataires_resolus_et_dossier_recursif` |
| 16 | Quitter un partage de fichier | 132 | `et1_partages_nominatifs_nx32_14_a_16` |
| 17 | Choisir des collègues dans l'annuaire | 151 | `et1_membres_resolus_par_le_coeur_nd32_03` |
| 18 | Voir son quota | 260 | `et1_quota_vue_et_depassement_nx32_18` |
| 19, 20 | Badge « invitation non vue » ; avis aux nouveaux membres | 152 | `et1_invitation_avis_neutre_et_badge_nx32_19` |

Autres ETR du lot : ETR-107 (renommer, décrire, couleur et icône : `et1_droits_verifies_dans_chaque_commande_nd32_04`), ETR-133 et ETR-134 (retirer, prolonger, lister les partages : `et1_partages_nominatifs_nx32_14_a_16`), ETR-154 (transfert : `et1_retirer_quitter_transferer_dernier_animateur`), ETR-160 à 164 (versions : `et1_jamais_d_ecrasement_versions_nd32_02`), ETR-124 et 125 (restaurer, vider : `et1_corbeille_apercu_restauration_purge_figee`). NX32-21 et NX32-22 (documents vierges, récents) sont rattachés à ET5 et ET4 par la conception : **hors lot**.

## 3. Défauts Next.js ND32-01 à ND32-18 : rendus impossibles (règle 14, D-051)

| ND32 | Défaut | Fermé par | Test de non-régression |
|---|---|---|---|
| 01 critique | taille et quota déclarés par le navigateur | taille signée dans l'URL, quota réservé en base, taille relue chez le fournisseur, empreinte du serveur | `et0_envoi_de_70_mio_signe_1_mio_est_refuse`, `et0_taille_invalide_refusee_et_cle_opaque`, `ent-object` `tests_envoi` (faux fournisseur S3 qui recalcule la signature) |
| 02 critique | un membre écrase le fichier du propriétaire | versions immuables, « nom déjà pris » avec choix explicite, déclencheur d'immuabilité | `et1_jamais_d_ecrasement_versions_nd32_02` |
| 03 critique | membres de partage non vérifiés | agent actif de l'annuaire, même message pour tout refus, clés composites | `et1_membres_resolus_par_le_coeur_nd32_03`, `et1_partage_destinataires_resolus_et_dossier_recursif` |
| 04 | contrôle par la session seule | `espaces.utiliser` / `creer` / `partager` puis rôle d'espace dans chaque commande ; garde statique | `et1_droits_verifies_dans_chaque_commande_nd32_04`, `garde_espaces` |
| 05 | aucune liste blanche, aucun antivirus | type réel par signature, macros et actifs refusés, analyse fermée par défaut | `et0_contenu_actif_ou_incoherent_refuse_a_la_finalisation`, `et0_objet_non_analyse_invisible_ferme_par_defaut`, `et0_fichier_infecte_en_quarantaine_quota_rendu` |
| 06 | liens d'une heure, transmissibles | jeton de 60 s lié au compte, émis après contrôle des droits | `et1_telechargement_lien_court_lie_au_compte_nx32_06` |
| 07 | métadonnées en JSON-mémoire | 16 tables normalisées, sans JSON, tableau ni BYTEA | `et1_schema_sans_json_ni_tableau_ni_bytea_et_rls_posee`, `garde_espaces` |
| 08 | partage par pointeur de chemin | partage sur l'identifiant (clé composite) | `et1_deplacer_renommer_sans_toucher_l_objet_et_partage_qui_suit` |
| 09 | déplacement non atomique | arborescence en base, l'objet ne bouge jamais | idem |
| 10 | contrôle de chemin naïf | clés opaques `{établissement}/espaces/{uuid}.bin`, noms sans séparateur ni invisible | `et0_taille_invalide_refusee_et_cle_opaque`, `espaces::tests::noms_propres_et_interdits` |
| 11 | écriture lors d'une lecture | badge levé par une commande explicite | `et1_mon_espace_idempotent_et_lectures_sans_ecriture` |
| 12 | avis en tâche détachée | notification du socle 8 écrite dans la transaction, enveloppe neutre | `et1_invitation_avis_neutre_et_badge_nx32_19` |
| 13 | erreur technique renvoyée | erreurs typées (`Refus`, 404 uniforme) | tous les tests de refus |
| 14 | envoi chargé en mémoire | envoi direct (en production, aucun octet ne passe par l'API) | `garde_espaces::aucun_octet_de_fichier_ne_transite_par_l_api` |
| 15 | tous les partages chargés, course sur le quota | compteurs verrouillés, requêtes ciblées | `et0_vingt_envois_paralleles_ne_depassent_pas_le_quota`, `et0_reconciliation_recalcule_l_occupation` |
| 16 | aucune borne | nom ≤ 120, profondeur ≤ 8, 2 000 éléments par dossier, 200 membres, 20 espaces par créateur | `et1_dossiers_noms_profondeur_deplacement`, `et1_droits_verifies_dans_chaque_commande_nd32_04` |
| 17 | pas de rôle lecture seule | rôles fermés animateur, contributeur, lecteur ; partage en lecture | `et1_jamais_d_ecrasement_versions_nd32_02`, `et1_partages_nominatifs_nx32_14_a_16` |
| 18 | clés sans identifiant d'établissement | clé préfixée par l'établissement (CHECK en base) ; cloisonnement par identifiant | `et1_cloisonnement_entre_etablissements_et_entre_espaces` |

ND32-19 et ND32-20 (bureautique vide, récents non atomiques) relèvent d'ET4 et d'ET5.

## 4. Règles R-ET couvertes

R-ET1 (membres lus à la date, jamais recopiés : ajoutés datés ; le calcul d'ancrage est ET2) ; R-ET2 (lire = membre à la date ou destinataire d'un partage actif : `acces_element`, `acces_dossier`) ; R-ET3 (version immuable, retour arrière = version) ; R-ET4 (l'objet ne bouge jamais) ; R-ET5 (rôles fermés) ; R-ET6 (version figée jamais élaguée ni effacée) ; R-ET9 (aucun lien public, 60 s) ; R-ET11 (vue des membres effectifs) ; R-ET12 (agent actif de l'établissement seulement) ; R-ET13 (partage sur l'identifiant) ; R-ET20 (volume compté à l'espace) ; R-ET21 (taille signée, réservation, réconciliation) ; R-ET22 (la liste des formats avec élèves exclut le ZIP : constante `ESPACE_AVEC_ELEVES`, sans effet en ET1) ; R-ET25 (bornes).

## 5. Sécurité par conception (exigences du lancement)

- **Jamais d'URL publique** : seuls des jetons courts liés au compte et à l'établissement (`/fichiers/{jeton}` pour la lecture, `/espaces/envoi/{jeton}` pour l'envoi local ; en S3, URL SigV4 de 15 minutes pour l'envoi, 60 s pour la lecture). Jetons d'envoi et de lecture non substituables (tests).
- **Aucune fuite** vers un autre établissement ni un non-membre : tout refus de lecture répond « introuvable » ; clés composites `(établissement, …)` partout, y compris espace-dossier-élément ; RLS posée sur les 16 tables ; `TenantTx` pour tout accès.
- **Quotas côté serveur**, y compris pour l'envoi direct : réservation avant l'émission de l'URL, taille signée, taille relue après l'envoi.
- **Jamais d'écrasement silencieux** : « nom déjà pris » (409) puis choix explicite ; version immuable.
- **Historique, corbeille, purge** : journal en ajout seul ; corbeille de 30 jours ; effacement à l'échéance, quota rendu, effacement physique après validation ; élagage S-3210.

## 6. Écarts à la conception, hypothèses et points à confirmer (D-031 : recommandation appliquée par défaut)

1. **Conteneur isolé de `ent-convert` (§30.5 : « ET0 inclut le conteneur isolé »)** : **non fait**. Il dépend des moteurs ArtCraft et de LibreOffice (ET5a), exclus de ce lot par la consigne. Sans lui, l'aperçu est **minimal** : lien en ligne pour PDF, images, texte ; tout le reste en téléchargement. **À confirmer** : déplacer R-ET41 en ET5a.
2. **Anciens formats OLE (DOC, XLS, PPT)** : admis par §12.1 mais **refusés** par le noyau (macros non vérifiables sans le moteur isolé) ; `detecter` répond « type inconnu ». À traiter avec ET5a ; **à inventorier dans l'export M19** (ils seraient listés à leur propriétaire, jamais perdus en silence, §26.3).
3. **Audio, vidéo, ZIP** : reconnus par signature (MP3, OGG, M4A, MP4, WebM ; ZIP sans décompression, chemins et programmes contrôlés). Plafond **64 Mio** pour tout fichier : la **vidéo à 500 Mio** (S-3214) est « soumise à Florian » (§28.2) et n'est pas appliquée.
4. **Quota d'un espace libre** : 2 Gio, **hypothèse** (la conception ne le chiffre pas ; M19 comptait 2 Gio au propriétaire) ; réglable par établissement dans `ent_espace_quota_profil` (lecture en ET0 ; écriture en ET9). **20 espaces libres** par créateur : hypothèse de borne d'usage (sans elle, chaque espace ayant son quota, le stockage ne serait plus borné).
5. **Nom d'un fichier : 200 caractères** (limite du noyau) au lieu de 120 : R-ET25 vise les noms d'espaces et de dossiers ; des noms repris de M19 peuvent être plus longs.
6. **Personnels seulement** : en ET1, ni l'espace personnel ni les espaces libres ne sont ouverts aux élèves et aux familles (`espaces.utiliser` n'est donné qu'aux personnels) ; ETR-110 dit « tous » : l'espace personnel d'élève (quota 1 Gio, protection du collectif) arrive avec ET2. **À confirmer.**
7. **Un compte sans fiche de personne** dans l'établissement n'a pas d'espace (refus explicite) ; en production, la fiche et l'agent viennent du socle 6 (reprise de comptes).
8. **« Export » (ligne ET1 du tableau des lots)** : **non fait**. L'export ZIP de l'espace personnel est ETR-266, rattaché à **ET9** (coffre du socle 7) par le tableau des actions. **À confirmer** que ET1 ne l'exige pas.
9. **Inventaire de reprise M19 « sur export anonymisé »** (ligne ET0) : **non fait** : aucun export n'a été fourni et l'accès aux données de production est interdit. Le lot livre ce qui permet de le faire (liste blanche, refus listables, clé et empreinte recalculées) ; l'inventaire se fera avec l'export de Florian (ET14).
10. **ETR-274 (« Retrouvés »)** : structurellement sans objet en ET1 (aucun élève dans un espace de personnels) ; la garde « éléments d'autrui » de la corbeille de dossier est en place et testée.
11. **Journal des téléchargements** : écrit seulement pour un espace avec élèves (§17.2 ETR-118) ; aucun en ET1.
12. **Notifications** : la nature `espaces.invitation` passe par la notification simple du socle 8 (même chemin que les accords), non par le catalogue de natures du moteur N1 : le catalogue du moteur n'a pas de nature « personnel sans élève » ; à reprendre avec N-lots si le pilote le souhaite.
13. **Droits par défaut** : 4 actions (`espaces.utiliser`, `creer`, `partager`, `journal.lire`) ; `espaces.animer`, `contribuer`, `membres`, `moderer` sont des **rôles d'espace** relus en base, pas des actions (« implicite par rôle d'espace », §20.1). `TOUTES` : 244 → 248.
14. **Catalogue de conservation** : catégorie `esp.espace` ajoutée (sans source, durées soumises au DPO, Q-ET7) et `VERSION_CATALOGUE` passée à `2026-11-17-et0` ; une autre branche qui l'incrémente produira un conflit à résoudre (règle 28).

## 7. Registres et gouvernance

- **Registre des tables** (`registre_donnees.rs`, domaine `espaces`) : +16 tables ; réservations neutralisées à la restauration (libérées, jamais supprimées), envois classés « intention » ; dérivés « reconstructibles » ; aucune exception D-036.
- **Registre RGPD** : traitement `espaces_documents` (personnels), ajouté à la mention des personnels ; catégorie de conservation `esp.espace`.
- **Compte RLS** (`tests_rls.rs`) : +16. **`MIGRATION_TAG`** inchangé. **`TOUTES`** : 248. **Matrice de séparation** : domaine `espaces`, `SOURCES` 58 → 59.
- **Garde statique** (`garde_espaces.rs`) : SQL du domaine confiné ; droit par commande ; aucun octet de fichier écrit par l'API hors deux exceptions motivées ; aucun lien sans contrôle des droits ; aucune horloge ; migrations sans JSON ni tableau ni BYTEA.

## 8. Limites connues (honnêtes)

- Aucun écran Leptos : la conception rattache les écrans §23 à plusieurs lots ; l'écran « Mes espaces » d'ET1 n'est **pas fait** (l'API est complète et testée). À planifier avec le pilote.
- L'envoi **local** (`PUT /espaces/envoi/{jeton}`) charge le corps en mémoire (64 Mio au plus) : labo seulement ; en production le navigateur envoie directement au S3, qui impose la taille signée. **L'envoi S3 réel n'a pas été essayé** (aucun accès au S3 réel) : la signature est vérifiée par un faux fournisseur de test qui recalcule SigV4.
- Pas de test HTTP dédié des routes (les services sont testés, la matrice de séparation couvre l'authentification et le cloisonnement).
- Antivirus : sans `CLAMAV_ADDR`, aucun fichier déposé ne devient visible (fermé par défaut, voulu) ; en labo, il faut un analyseur de test.
- Le chemin « partage vers un élève suivi par le professeur » (ETR-130) est ET2.

## 9. Vérifications

Voir la fin de ce fichier (renseignée à la livraison) : clippy 1.99.0 (0 nouvel avertissement), tests locaux, fusion d'`origin/ENT`, SHA et numéro de run de la CI.
