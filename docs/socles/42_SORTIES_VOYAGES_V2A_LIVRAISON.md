# Socle 42 « Sorties et voyages » — lot V2a « suivi de l'argent » : livraison

Branche `ent-socle-42-code-v2a` (partie de `ENT`, tête 015bc4f6, run 391 vert). Conception : `docs/socles/42_SORTIES_VOYAGES.md` §32.5, §12.5, VOY-266 à VOY-268, VOY-270, VOY-273, VOY-286, R-SV44, R-SV45, S-437, S-438, CV-71, CV-81 ; état d'implémentation au §34 du même document.

**Statut : implémenté et testé en local, non validé par Florian, non prêt pilote, non prêt production.** Distinctions (CLAUDE.md) : ce qui est *implémenté et testé* est listé au §1 ; ce qui est *proposé ou non fait* au §3.

## 0. Principe

Tant que D-034 garde la comptabilité dans Charlemagne, ScolIA **décide** la part des familles, la **répartit**, **pointe** les versements, **propose** des avoirs et **exporte**. Il n'émet **aucune facture, aucun avis de prélèvement, aucun avoir comptable**. Preuve : le lot n'écrit dans aucune table `ent_fac_*` ni `ent_fin_*` (garde R-F1 verte ; seules lectures : `ent_fin_compte`, `ent_fin_entite_facturation`), et les tests vérifient qu'aucune facture n'existe après pointage, avoirs et export.

## 1. Livré

| Réf. | Ce qui est fait | Où |
|---|---|---|
| Part familles (R-SV15) | la part est le **prix annoncé figé** (`ent_sortie_prix_fige`), instantané daté par payeur (`ent_sortie_part`, ajout seul) ; une baisse ajoute de nouvelles lignes | `voyages_argent_repo.rs` (`synchroniser`) |
| Répartition entre payeurs | **port du 24** `facturation_payeurs::payeurs_a` (catégorie « voyages », jour de l'établissement) puis `finances_calcul::repartir` (plus fort reste) : **aucune logique de répartition recopiée** ; répartition incomplète = participant « à définir », jamais « au responsable 1 » (R-F4) | idem |
| VOY-267 pointer un versement | chèque, espèces, virement, chèques-vacances (si agrément ANCV, désactivé par défaut) ; montant texte → centimes (`checked_*`), `CHECK` de bornes en base, jamais plus que la part du payeur, date jamais dans le futur ; **ajout seul** : annulation par écriture inverse motivée, vérifiée par la base ; UPDATE/DELETE/TRUNCATE refusés | `voyages_argent_repo.rs`, migration 650 |
| Plan long sans que l'organisateur le voie | `ent_sortie_plan` : 1 à 3 fois standard, 4 à 10 fois **accordé** par la comptabilité ou la direction avec motif ; l'échéancier est **calculé** (jamais périmé) ; l'organisateur ne voit que « 1 plan en cours » | `voyages_argent.rs` |
| État agrégé (R-SV18) | nombres seulement (soldés, à jour, plans en cours, en retard, à définir, totaux) ; aucun nom, aucun champ nominatif dans le type | `voyage_argent_etat` |
| Nominatif | `voyages.argent` (comptabilité, direction, administrateur) ; le **porteur** seulement si la direction ouvre l'accès avec motif (VOY-223) ; **chaque accès journalisé** (`ent_sortie_argent_acces`) | `voyage_argent_paiements`, `voyage_nominatif_*` |
| Famille | un parent ne voit que **sa** part et **ses** versements : le filtre porte sur les comptes dont il est payeur (`comptes_du_user` du 24), jamais le compte de l'autre payeur du même élève | `voyage_argent_famille` |
| VOY-266 bordereau | l'organisateur (porteur) saisit nominatif : élève, n° de chèque, montant ; le nombre et le total annoncés doivent être ceux des lignes (double saisie + contrôle de la base au COMMIT) ; remise au secrétariat (autre personne que le saisisseur) ; **jour de l'établissement** ; **écart** et **retard** calculés par la base ; signalement (avis à la comptabilité, sans montant) ; liste des bordereaux non remis ; pointage ligne par ligne vers le bon payeur | `voyages_argent_flux.rs`, migration 651 |
| Public : régisseur seul | `ent_fin_entite_facturation.regime = public_eple` ⇒ l'organisateur ne collecte pas (non réglable) ; dans le privé, réglage `collecte_organisateur` | `est_public`, `voyages_argent_reglages` |
| VOY-270 acomptes fournisseurs | à payer / bientôt / en retard / payé / annulé ; alerte avant échéance (fenêtre réglable 1 à 90 jours, 14 par défaut) à la comptabilité et à la direction, **une seule fois** (passage idempotent par établissement et pour tous) ; prestataire d'un autre établissement : « introuvable » | `voyages_argent_flux.rs` |
| §12.5 avoirs proposés | désistement avant / après engagement des dépenses, motif légitime (santé, famille) = avoir total, annulation collective, départ décidé par l'établissement, baisse de prix ; **par payeur, jamais plus que ce qu'il a versé** (CF-69) ; la comptabilité retient (montant ≤ versé) ou écarte, avec motif ; un recalcul écarte l'ancienne proposition sans la supprimer ; **aucune écriture comptable** | `voyages_argent.rs` (`calculer_avoir`), repo |
| VOY-273 frais réels | billet nominatif, pénalité, hébergement non remboursable, autre ; répartis entre les payeurs au prorata ; annulation motivée ; recalcul de l'avoir tant qu'il n'est pas retenu | repo |
| VOY-286 départ en préparation | branché sur le désistement et le retrait : parts calculées si besoin puis avoir proposé ; plus aucun versement ne se pointe pour un participant parti ; quittances (versements) gardées | `voyages_repo.rs` (crochet), `avant_depart_participant` |
| VOY-268 export | CSV lisible (UTF-8 avec BOM, séparateur `;`, euros à la française, injection de formule neutralisée) : parts, versements, annulations, frais réels, avoirs **proposés** ; objet au magasin, **lien court** émis après contrôle des droits, export journalisé (empreinte, nombre de lignes, accès) | `voyage_argent_export` |
| NX-07 | motifs **codés** : transport en commun, à pied, véhicule de l'établissement (aucune justification), autre (texte obligatoire) ; texte seul = « autre » | migration 652, `contournement_motif` |
| CV-81 | sortie obligatoire : part 0 imposée, aucune part créée, pointage refusé | `synchroniser`, `pointer_tx` |
| Avis | `voyages.acompte_echeance` et `voyages.bordereau_signale` → comptabilité et direction (par droit `voyages.argent`), sans montant ni nom | `voyages_reactions.rs` |

**Droits** (`ent-auth`) : `voyages.argent` (comptabilité, direction, administrateur) et `voyages.remise` (secrétariat : reçoit les bordereaux et pointe leurs lignes, sans voir la liste des paiements). `Action::TOUTES` 269 → **271**.

**Isolation** : toutes les requêtes portent `etablissement_id` ; clés étrangères composites ; RLS forcée sur les 13 tables nouvelles ; un identifiant d'un autre établissement répond « introuvable » (testé pour la sortie, le prestataire, la famille, l'export).

**Outbox** : aucun montant ni nom. Seuls deux événements sont publiés (`voyages.bordereau_signale`, `voyages.acompte_echeance`) avec des identifiants et un code ; ils passent par `publier` du 42 avec une charge minimale écrite en dur (pas de liste de clés libre). Aucun événement de versement ni de part. Test sur les charges réelles.

## 2. Compteurs (règle 28)

| Compteur | Valeur |
|---|---|
| `Action::TOUTES` | **271** (269 + `voyages.argent`, `voyages.remise`, en fin de catalogue ; test de position des droits fins ajusté) |
| Tables avec RLS (`tests_rls.rs`) | base ENT **+ 13** (650 : 6, 651 : 7) |
| Registre des données | **+ 13** tables, domaine `sorties_voyages` |
| `SOURCES` / `NOMS_SOURCES` (séparation) | **65** (64 + `voyages_argent.rs`) |
| Routes | **+ 22** (`ent-api/src/voyages_argent.rs`) |
| Migrations | **650, 651, 652** (653–659 réservées) ; `MIGRATION_TAG` inchangé (3808) |
| `VERSION_CATALOGUE` (conservation) | **inchangée** : aucune catégorie nouvelle ; durées à faire valider par le DPO (écart 6) |

## 3. Vérifications (locales, base PostgreSQL 16 jetable démarrée, `ENT_EXIGER_BASE=1` exporté)

- `ent-db` : **28** tests V2a (`tests_voyages_v2a`, calcul pur et avis compris) ; **171** tests « voyages » (V0a à V2a) ; registre, RLS, conservation, finances, rôles (**219** + rejeu) ; gardes `jour_utc_interdit`, `isolation_sql_statique`, `garde_finances_ecritures`, `garde_inscriptions_ecritures` ;
- `ent-auth` : 64 ; `ent-api` : séparation 56, recette sécurité 10, golden finances 1, routes voyages 4 ;
- clippy en trois commandes de la CI : verts (**toolchain locale 1.97**, la CI utilise 1.99 : à confirmer par la CI) ; formatage (deux commandes) ; `--locked` sur les deux `Cargo.lock`.
- **Non fait localement** : la suite complète d'`ent-db` (interrompue par la limite de temps d'une session, remplacée par des tranches ciblées) ; la CI GitHub (20 tranches) fait foi.

## 4. Cas couverts par les tests V2a

Parts via le port du 24 (deux parents séparés 250/250, un payeur 500, un sans payeur « à définir ») ; état agrégé sans nom ; nominatif refusé au porteur, journalisé ; versements (bornes, moyens, dépassement, annulation inverse motivée, une seule annulation) ; plan long invisible ; bordereau (total, doublon, écart, retard, remise unique, pointage, public interdit, réglage) ; acomptes (statuts, alerte une fois, prestataire d'un autre établissement) ; avoirs (avant/après engagement, motif légitime, frais réels, décision, baisse de prix, annulation collective sans doublon) ; CV-81 ; NX-07 ; famille (parents séparés) ; ouverture nominative ; export CSV (BOM, injection de formule, journal) ; contraintes de la base (ajout seul, bornes, somme du bordereau) ; outbox sans montant ni nom.

## 5. Écarts, hypothèses et questions

1. **Part familles = prix annoncé figé** (pas la part de la version validée brute) : une hausse non décidée par la direction n'est donc jamais due. *À confirmer.*
2. **Changement de payeur après calcul** : les parts ne sont pas recalculées (comme CF-03 du 24 : la comptabilité décide). Un prix qui change garde les mêmes payeurs.
3. **Organisme payeur avec plafond (rang 2)** : ignoré en mode suivi (seuls les payeurs de rang 1 portent la part).
4. **Frais réels** : le montant saisi est la part **non remboursée** restant due ; une assurance ou un prestataire qui rembourse se traduit par une annulation de frais. *À valider avec la comptabilité.* « Après engagement » = une commande envoyée **ou** un frais réel saisi.
5. **Motifs légitimes** retenus pour l'avoir total : `sante` et `famille` ; « décision de l'établissement » et « financier » (retrait) donnent un avoir total ; le reste suit l'engagement. *Règle à relire avec un juriste (Q-V6).*
6. **Conservation (RGPD)** : durées des 13 tables non fixées ; 10 ans proposé pour versements, parts, avoirs (pièces comptables), 24 mois pour les accès. *À soumettre au DPO.*
7. **Ordonnanceur** : le passage quotidien des alertes d'acomptes existe (`voyages_acomptes_alertes_traiter_tous`) mais **n'est branché sur aucun planificateur**, comme les rappels du lot V0b ; déclenchable à la main (`POST /voyages/argent/acomptes/alertes`).
8. **Charges d'événements préexistantes (V0b)** : `voyages.prix_fige` et `voyages.budget_version_validee` portent `montant_centimes` dans l'outbox. **Non modifié** (hors périmètre, risque pour des consommateurs) ; à traiter par un correctif dédié. Les événements du lot n'en portent aucun.
9. **Liste d'attente / « changement de nom »** (VOY-286) : le lot propose l'avoir au départ ; le report automatique des versements sur l'élève qui prend la place n'est **pas** fait (la comptabilité retient l'avoir ou le reporte).
10. **Régisseur** : le rôle n'existe pas dans le modèle ; la règle « public : l'organisateur ne collecte pas » s'applique, la saisie des fonds par le régisseur reste via `voyages.argent` / `voyages.remise`. *Question : qui est le régisseur dans l'ENT ?*
11. **Espace famille** : route prête, **aucun écran** (Leptos non traité). VOY-273 : frais saisis par la comptabilité seule (pas par l'organisateur).
12. **Format d'export** : colonnes choisies sans voir la saisie de Charlemagne (`[NV]`) ; à ajuster sur un exemple réel.
13. Aucune dépendance ajoutée ; aucune migration versée modifiée ; Next.js, base et secrets de production : **non touchés**.
