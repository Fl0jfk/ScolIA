# Contre-revue — voyages V3b, correctifs (PR 145, tête 6ead853b) — 2026-11-24

Relecteur `review-sonnet`, lecture seule. Moyens 1 (autorisation « en blanc ») et 3 (rattrapage sous RLS stricte) **fermés** ; Moyen 2 incomplet.

## Vérifié sans défaut
- `portee_chk` : plafond et prestataire obligatoires pour créer/payer, cible pour payer/annuler ; NULL à l'écriture = échec fermé.
- UPDATE direct de paiement/annulation gardé (cible = l'acompte, plafond) ; usage unique (index) ; `paye_le` immuable.
- Horodatages posés par déclencheur ; UPDATE/DELETE bloqués (ajout seul).
- Rattrapage : `ent_etablissement UNION ent_membership`, réglage restauré, transactionnel, idempotent.
- `search_path` et relations `public.` partout (sauf `ent_internat_attendu_calcule`, famille documentée) ; aucun SECURITY DEFINER ; 656 en fin de tableau ; pas d'heure réelle dans les tests.

## Bloquant
- **B1** : `ent_sortie_habilite_argent` ne vérifiait que le rôle ; un accès suspendu, fermé ou échu (`ent_membership.etat`, `date_fin`, 350/352) habilitait encore, à la décision et à l'usage ; commentaire et doc faux (« pas d'indicateur actif »). → **Corrigé par le pilote** : `AND public.ent_acces_ouvert(usr, etab)` ; test SQL direct (suspendu, fermé, échu).

## Moyens
- **M1** : un acompte INSÉRÉ déjà payé échappait à l'autorisation « payer » et à son plafond (seul « créer » était jugé, sur `montant_centimes`, et `cree_par` n'était pas évalué si `paye_par` était renseigné). → **Corrigé par le pilote** : insertion payée ou annulée refusée (23514), le créateur seul est évalué ; test SQL direct.
- **M2** : une autorisation « annulation collective » couvre plusieurs avoirs pendant 30 jours (la doc dit « une seule écriture »). Impact limité : avoirs seulement proposés, leur décision reste gardée. → **Décision D-069 du pilote** : choix assumé (une annulation collective porte par nature sur tous les participants), doc corrigée ; borne par versement en file.
- **M3** : tests manquants : expiration à la décision et à l'usage par les déclencheurs (horloge de base), UPDATE/DELETE des horodatages, `portee_chk` en SQL direct, annulation d'acompte hors cible en SQL direct, autorisatrice d'un autre établissement réel, restauration du réglage et idempotence du rattrapage. → **En file** (suites V3b, 2).

## Faibles (en file)
1. Garde d'avoir collectif fondé sur les versements, pas sur `NEW.participant_id`/`compte_id` ; la branche collective court-circuite le contrôle de `decide_par_user_id`.
2. `rentree_le` passé arbitraire accepté en SQL direct (choix documenté).
3. Une base de labo ayant appliqué l'ancienne 656 doit être recréée (empreinte modifiée) ; sans effet en production.
4. Rattrapage limité à `etat = 'rentree'` (repli applicatif sur `modifie_le`).
5. Une autorisation ne peut pas être révoquée par son autrice.
6. `ent_internat_attendu_calcule` : fonctions non qualifiées (famille documentée).

## Verdict du relecteur
Après correctifs (B1, M1, M2 tranché, M3). Suite donnée : B1, M1 corrigés ; M2 tranché ; M3 et faibles en file → versable.
