# Contre-revue de la passe 2 d'I4 (PR 122, commit a9a9b1e1) — synthèse du pilote

Revue indépendante en lecture seule. **Aucun élevé.** É1, M1-M5, M7 et F-g sont vérifiés dans le code. 654 est intégralement reprise dans 2829 et l'ordre de `migrate.rs` est correct.

## Moyens (à faire dans « suites I4 »)

1. **2828 et 2829 modifiées avant versement.** Une base de labo déjà migrée refuse de démarrer (`FichierModifie`), et le CHECK en ligne de `motif_code` (2828:99-102, dans un `CREATE TABLE IF NOT EXISTS`) ne se met pas à jour au rejeu.
   - Les migrations sont versées depuis : passer par une migration nouvelle (contrainte nommée, `DROP CONSTRAINT IF EXISTS` puis `ADD`), comme 654:12-27.
2. **Élève protégé listé au temps du soir mais impossible à pointer par l'encadrant.**
   - Cause : `pointer_temps_a` appelle `exiger_eleve_visible` (`vie.rs` ~2455), qui renvoie `NotFound` pour un protégé caché.
   - Correctif : pour un élève inscrit au temps, contrôler l'inscription et non la visibilité. L'opacité envers un tiers est assurée par « n'est pas inscrit ».
3. **Test `m6_la_tâche_planifiée…`** : `internat_sorties_tache_a` agit sur **tous** les établissements de la base partagée et peut casser les comptes exacts des tests voisins (`m9` : « == 2 »).
   - Correctif : une variante limitée à un établissement, ou un appel direct des trois `*_bilan_a`.
4. **É1, marque « en activité » sur un temps annulé** : non assertée (`tests_internat_i4_passe2.rs:153`). Ajouter le test d'ouverture d'appel avec un temps annulé.
5. **M1, réouverture chaque nuit.** La nuit change à midi (`ent_nuit_etab`), et le filtre `nuit = $3` laisse ouvrir un nouveau cas chaque jour tant que `rentre` n'est pas pointé.
   - Correctif : ne pas rouvrir si un cas clos existe pour (élève, origine), toutes nuits confondues, ou borner l'âge de la ligne.

## Faibles

- Retour sur inscription retirée refusé par le déclencheur 2829:409-411 : la clause `retire_le IS NULL OR $4` est du code mort. `ORDER BY retire_le DESC NULLS FIRST` choisit la ligne active en cas de réinscription.
- Course veille / retour : un cas peut s'ouvrir pour un élève déjà rentré, faute de relecture de `rentre_le` sous verrou (fausse alerte seulement).
- `search_path` non fixé sur les gardes de 2828 et 2829, et sur `ent_internat_attendu_calcule` (table temporaire homonyme) → chantier transverse.
- M5 seulement déclaratif. Restent exportables : `ent_internat_cas_etape.note`, `accord_hors_ligne` (refus d'un parent) et `reponse_qualite` → lot G3.
- Avis M3 non regroupés : un avis par cas et par détenteur (risque de saturation).
- Marquage « protégé » différent entre temps du soir (`proteges ∪ caches`) et sorties (`proteges`).
- F-g, « toujours » exagéré : un refus téléphonique après le départ ou hors `en_attente`/`autorise` n'est pas enregistré. **Décision D-069** : un refus après le départ s'enregistre comme fait tracé et déclenche le retour, sans annuler rétroactivement le départ.
- Tests lâches : `fg_*:1015` (`assert_ne!` au lieu de `refuse`), `m3_*:554` (assertion vide), `m5_*:669` (`unwrap_or(true)`).
- Nouveau consommateur `i4.internat_alertes` : vérifier qu'il ne rejoue pas l'historique au premier passage.
