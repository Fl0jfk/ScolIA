# Socle 10 — lots T5a (matérialisation) et T5 (publication) — rapport de code

Branches empilées `ent-socle-10-code-t5a` puis `ent-socle-10-code-t5`, base `origin/ENT` 6d50d785 (2026-11-15).
Conception : `docs/socles/10_EMPLOI_DU_TEMPS.md` §8.1, §14.1, §21.2, EDT-150 à 156, R-E7, CL-60 (sur `origin/ent-mc0hoe`).
**Niveau : implémenté + testé en local (PostgreSQL 16) ; CI : une seule, au sommet de la pile ; non validé par Florian, pas prêt pilote.**

## Lot T5a — les séances viennent des placements

Le rapport T4 signalait le préalable : la publication suppose que les séances soient produites à partir des placements (T1), alors
qu'elles venaient encore des créneaux historiques.

| Point | Réalisation |
|---|---|
| Source des séances | `ref_generer_seances` lit, pour chaque date, les **placements de la version de base publiée en vigueur**, surchargés par les **versions temporaires** publiées (cours du périmètre : cours, intervenants, lieux) ; à la fin de la plage, le plan de base revient seul. `edt_materialisation.rs` (sources par tranche de version), `referentiel_seances.rs` (cœur `generer_seances_dans`, dans la transaction de l'appelant) |
| Établissements sans version | La version « reprise » (M2) garde les créneaux historiques comme source **exacte** : aucune régression ; les créneaux **sans cours** (« listés, jamais devinés ») produisent leurs séances comme avant |
| Identité stable | Un placement identique à celui d'une autre version (même cours, jour, plage, semaine type, lieux) **réutilise le même créneau** : la séance garde son identifiant, donc l'appel, le cahier, le travail à faire et le **journal des modifications T3** (annulation, lieu, horaire…) |
| Pont jusqu'à T11 | `ent_edt_seance.creneau_id` reste `NOT NULL` : un placement sans créneau historique reçoit un créneau **projeté** (`ent_edt_creneau.projete`, migration 448), ignoré par la reprise. Retrait au lot T11 avec la bascule des lecteurs |
| Retrait sans perte | Une séance qui n'est plus attendue et porte un contenu ou un travail est **retirée** (état `retiree`), jamais supprimée (D1) ; vide, elle est supprimée |
| Cours vacant | Un cours sans intervenant n'est **pas projeté** (pas de séance inventée) : compté dans `RapportSeances::non_projetes`, ses séances apparaissent au recalcul dès qu'un service est ouvert |
| Idempotence | Recalculer ne change rien (testé : 0 créée, 0 mise à jour, 0 supprimée) |

Migration **448** `edt_t5a_materialisation` (colonne `projete`). Aucune migration existante modifiée.

## Lot T5 — publication

### Migrations
| N° | Fichier | Contenu |
|---|---|---|
| 449 | `449_edt_t5_publication` | colonnes de `ent_edt_version` (`annoncee_le/par`, `empreinte_controles`, `remplace_version_id`, `remplace_valable_au_avant`, `remplace_etat_avant`, `copie_de_version_id`, `retiree_le/par`) ; `ent_edt_version_journal` (**ajout seul**, déclencheur) ; `ent_edt_individuelle` et `ent_edt_individuelle_ligne` (version individuelle) |
| 455 | `455_edt_t5_rls_permissive` (après 454) | RLS d'établissement des 3 tables de 449 |

`MIGRATION_TAG` inchangé (`3703_…`, la plus haute du répertoire). Aucune colonne JSON ni tableau, aucune date UTC du jour (`ent_aujourdhui_etab`, `jour_paris`).
Paramètres ajoutés au registre : `edt.publication_deux_temps` (oui) et `edt.publication_delai_jours_ouvres` (2).

### Actions EDT-150 à 156
| Action | Fonction (`ent-db::edt_publication`) | Route | Droit |
|---|---|---|---|
| 150 contrôler (sans écriture) | `edt_publication_controler` | `GET /edt/versions/{id}/controles` | `edt.publication_lire` |
| aperçu | `edt_publication_apercu` | `GET /edt/versions/{id}/apercu` | `edt.publication_lire` |
| 151–152 publier (provisoire) | `edt_publier` | `POST /edt/versions/{id}/publier` | `edt.publier` (+ `edt.forcer_conflit` pour un effet le jour même) |
| 153 diffusion | `edt_diffuser`, `edt_diffusion_regler`, `edt_diffuser_echues` | `POST …/diffuser`, `POST …/diffusion`, `POST /edt/publications/diffuser-echues` | `edt.publier` |
| 154 historique, différences | `edt_publications_historique`, `edt_versions_difference` | `GET /edt/annees/{id}/publications`, `GET /edt/versions/difference` | `edt.publication_lire` |
| 155 retour arrière | `edt_retour_arriere`, `edt_annonce_retirer` | `POST …/retour-arriere`, `DELETE …/annonce` | `edt.revenir_version` |
| 156 version individuelle | `edt_publier_individuelle` | `POST /edt/individuelles` | `edt.publier` |

Droits ajoutés (fonction `socle_10_edt`, une ligne d'appel — règle 21) : `edt.publication_lire` et `edt.publier` (direction, secrétariat), `edt.revenir_version` (direction). Professeur, vie scolaire, familles : aucun.
Un contrôle bloquant répond **409** avec la liste (codes, nombres, identifiants de cours et de placements : jamais un nom).

### Règles réalisées
- **Contrôles bloquants** : conflits durs de cours, d'intervenant et de lieu (pour une temporaire, avec le plan de base hors périmètre) ; version vide ; date hors de l'année ; publication qui précéderait une version déjà publiée ; temporaire sans plage ou sans base publiée ; cours sans classe (ses séances ne pourraient pas exister).
- **Avertissements** : cours vacants ; heures placées ≠ volume du cours ; placement sans créneau de grille ; trou entre deux versions.
- **Aperçu = la vraie bascule, annulée** (point de sauvegarde) : séances créées, retirées, conservées, modifiées ; cours, classes, intervenants touchés ; modifications ponctuelles conservées ou **à revoir** ; messages qui partiront. Aucune écriture ne reste (testé).
- **Deux temps** : *annoncer* fige la version (placements non modifiables, testé) et pose effet et diffusion, sans toucher au réel ; *diffuser* est une seule transaction (version précédente tronquée à la veille, nouvelle `publiee`, séances matérialisées, événement). Publication immédiate = les deux. Si le délai ne laisse pas de jour avant l'effet, la diffusion est immédiate.
- **Lendemain** (CL-60) : une publication demandée aujourd'hui (jour de Paris) prend effet **demain**, journalisé « décalée » ; la direction peut forcer l'effet du jour avec un motif (journalisé) : les séances déjà commencées ne bougent pas (`fige_avant`).
- **Atomicité** (R-E7) : la bascule est sérialisée avec la génération (même verrou) ; test **concurrent** : 20 lectures pendant la bascule, jamais d'ancienne et de nouvelle version ensemble.
- **Retour arrière** : annonce retirée → brouillon modifiable ; effet pas commencé → version remplacée rétablie, nouvelle archivée, séances recalculées ; version en vigueur → **copie** de la précédente publiée à partir de demain (le passé ne change pas).
- **Version individuelle** : inclusions et exclusions datées du socle 5 (`ent_cours_membre`), refus si l'ajout crée un conflit d'horaire dans l'emploi du temps de l'élève (409), une clé = une version, message à l'élève et à ses responsables si mineur **sans le motif**.
- **Notifications** (`d4.notifications_edt_t5`, socle 8 seulement) : un message par intervenant à l'annonce (avec la date de diffusion), un par élève et responsable (mineur, sans interdiction de communication) à la diffusion, jamais de nom d'intervenant ni de motif.

### Tests (`tests_edt_t5.rs`, 12 scénarios Postgres + tests unitaires)
T5a : identité stable et journal conservé, changement de position (contenu retiré, vide supprimé), temporaire puis retour de la base, cours vacant, séparation et heure de Paris à travers le changement d'heure. T5 : contrôles et droits sans écriture, publication immédiate (effet demain, journal, historique, différences, idempotence), effet forcé, deux temps (figée, diffusion réglée puis échue, un message par personne), retour arrière (trois cas), lecture concurrente, version individuelle, séparation entre établissements. Unitaires : jours ouvrés, date d'effet, comparaison des séances et des plans.
Non-régression : T0–T4, référentiel S2, EDT S2, cahier, assiduité AS2, absences de personnel (183 tests verts avant T5).

### Limites et reports (honnêtes)
- **Pont des créneaux** : un placement sans créneau historique reçoit un créneau projeté ; les listes de créneaux historiques (administration) montrent donc aussi les créneaux projetés, et les versions remplacées gardent les leurs (retrait au lot T11).
- **Intervenant** : le titulaire du service (socle 6) ; un suppléant daté ou un co-intervenant ne donne pas encore une séance propre (les remplacements passent par T3/socle 15).
- **Contrôles non faits** : alignements (barrettes), trous au-delà du plafond, pause méridienne, capacité des salles, quotité des temps partiels (H-14) ; jours fériés non retirés du délai en jours ouvrés.
- **Diffusion échue** : `edt_diffuser_echues` est appelée par le recalcul de la fenêtre et par une route ; **aucun ordonnanceur périodique ne l'appelle encore**.
- **Lecture « avant diffusion »** : les personnels lisent leur EDT à venir par l'aperçu et l'historique ; l'écran « mon EDT de l'année suivante » (EDT-157) n'existe pas encore. Les familles et élèves ne voient rien tant que la version n'est pas diffusée (le réel ne change qu'à la diffusion).
- **Version individuelle** : refus d'une collision d'horaire sur le plan de base seulement ; pas de levée (annulation) ni d'écran.
- **Explication figée** (R-E23) : relève de la génération (T8b). Aucun écran Leptos (T12f).
