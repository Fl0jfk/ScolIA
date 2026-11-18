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

À recalculer après chaque fusion d'`origin/ENT` ; valeurs au moment de la livraison : voir le §4.

## 3. Non fait, écarts, hypothèses (à valider)

Voir le §5 « Écarts et questions ».
