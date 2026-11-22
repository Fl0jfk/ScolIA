# Socle 42 « Sorties et voyages » — lot V3 « argent relié » : livraison

Branche `ent-socle-42-code-suite` (partie de `ENT`, tête 02659006c). Conception : `docs/socles/42_SORTIES_VOYAGES.md` §12.4, §12.5, §6.2, VOY-224, VOY-286 ; D-068 (la comptabilité des voyages est épaissie et reliée au reste de l'ENT, pas refondue) ; écart n° 9 du rapport V2a.

**Statut : implémenté et testé en local, non validé par Florian, non prêt pilote, non prêt production.**

## 0. Choix de périmètre (D-069, valeur prudente, aucune question posée)

Le « V3 » du tableau §27.1 (EDT et appel : neutralisation fine, non-partants, appel de départ, conflits) **n'est pas livré ici** : la consigne demande d'épaissir et relier la comptabilité (D-068). Ce lot ferme donc les trois liens les plus utiles laissés ouverts par V2a ; le V3 « EDT et appel » reste à faire (renommé **V3b** par convention de ce rapport).

## 1. Livré

| Lien | Ce qui est fait | Où |
|---|---|---|
| **Aides** (VOY-224, socle 26 à venir) | aide inscrite par la comptabilité sur la part d'**un** payeur (APEL, fonds social, collectivité, entreprise, don affecté, autre) ; elle réduit le reste dû, **n'est jamais de l'argent versé** : un avoir ne la rend pas au payeur (CF-69) ; plafonnée au reste dû (versé + aides) ; les versements aussi tiennent compte des aides ; annulation par écriture inverse motivée ; aide d'un participant parti signalée « à restituer ou réaffecter » ; liste nominative réservée à `voyages.argent`, consultation journalisée | `voyages_argent_v3.rs`, migration 653 |
| **Reports** (écart 9) | report d'un versement d'un participant à un autre **du même payeur** (même compte des deux côtés, imposé par la base) ; jamais plus que le versé du premier ni que le reste dû du second ; l'avoir du partant est recalculé (ancienne proposition écartée « signée système », nouvelle proposée) ; refusé si un avoir est déjà retenu ; annulation inverse motivée, refusée si l'argent n'est plus chez le receveur, et avoirs du receveur mis « à revoir » | idem |
| **Familles** | un payeur voit sa part, **son** aide (jamais sa source), son versé, son reste, les reports de ses enfants (sans le nom de l'autre enfant) ; l'autre payeur du même élève ne voit rien | `voyage_argent_famille` |
| **Responsable désigné** | voit ce qui couvre la part (aide comprise) sans savoir qu'une part est une aide ; l'état agrégé de l'organisateur ne contient aucune aide | `LignePaiementResponsable` |
| **Export** | CSV : lignes `aide`, `aide_annulee`, `report_cede`, `report_annule` | `construire_csv` |
| **Internat** (socle 41) | `POST /voyages/{id}/internat/synchroniser` : chaque **interne** confirmé (prêt ou parti) est retiré de l'attendu des nuits du séjour (nuit du départ → veille du retour, jours de l'établissement), motif `sejour_scolaire`, source `voyage` ; idempotent ; dates changées = ancien retrait annulé, nouveau posé ; désistement/retrait d'un participant = retrait annulé automatiquement (jamais supprimé) ; événement `internat.attendu_modifie` sans nom ni montant | idem, migration 654 |
| **Autorisations** (socle 18) | pas de nouveau lien : le départ d'un participant bloqué par une exigence reste couvert par le motif `exigence_manquante` → avoir (V2a) ; un parent séparé ne voit toujours que son compte | — |

Règles tenues : argent au centime (texte → `centimes_de_texte`, `CHECK` de bornes, `checked_*`) ; aucune donnée de santé ; ajout seul pour tout mouvement (UPDATE/DELETE/TRUNCATE → `42501`, testé) ; aucune écriture dans `ent_fac_*` ni `ent_fin_*`.

## 2. Compteurs (règle 28)

| Compteur | Valeur |
|---|---|
| `Action::TOUTES` | **271** (inchangé : droits `voyages.argent` et `VoyagesModifier` réutilisés) |
| Tables avec RLS (`tests_rls.rs`) | base ENT **+ 3** (`ent_sortie_aide`, `ent_sortie_report`, `ent_sortie_internat_retrait`) |
| Registre des données | **+ 3** tables, domaine `sorties_voyages` |
| `SOURCES` / `NOMS_SOURCES` | 69 inchangé (routes ajoutées dans `voyages_argent.rs`) |
| Routes | **+ 5** : `…/argent/aides`, `…/aides/{aid}/annulation`, `…/argent/reports`, `…/reports/{rid}/annulation`, `…/internat/synchroniser` |
| Migrations | **653** et **654** (plage 650-659 du socle 42 ; 655-659 libres). 654 est appliquée **après 2827** dans `migrate.rs` (elle référence `ent_internat_retrait`) ; `MIGRATION_TAG` inchangé (3808) |

## 3. Écarts, hypothèses et questions

1. **Aide d'un participant parti** : l'aide n'est pas annulée automatiquement ; la comptabilité décide (restituer à la source, réaffecter). Signalée `a_restituer`.
2. **Report** : seulement au sein d'un même payeur (prudence : pas de transfert d'argent d'une famille à une autre sans accord). Un remplaçant d'une autre famille passe par un avoir puis un nouveau versement.
3. **Internat** : seuls les internes dont un séjour d'internat couvre une nuit sont retirés ; la dispense de tarif d'internat/pension (socle 24/40) n'est **pas** faite. La synchronisation est déclenchée par le porteur ou la direction ; aucun planificateur (comme V2a).
4. **Source `voyage`** ajoutée à la contrainte des retraits du socle 41 par remplacement de contrainte (migration additive ; 2824 non modifiée).
5. **Non fait** (suite du socle) : écran famille (Leptos), clôture financière (VOY-227/230/231, V11), demande d'aide par la famille (socle 26), V3b EDT/appel.

## 4. Vérifications (locales, base PostgreSQL 16 neuve `ent_test_v3`, `ENT_EXIGER_BASE=1`)

- `ent-db` : 8 tests V3 (`tests_voyages_v3`) ; 195 tests « voyages » verts ; registre, RLS, conservation, internat, finances, rôles et gardes statiques : voir la fin de ce rapport ;
- clippy 1.99.0 et formatage ciblé : voir la fin de ce rapport.
