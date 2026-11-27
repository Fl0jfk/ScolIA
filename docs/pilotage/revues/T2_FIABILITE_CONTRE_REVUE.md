# Contre-revue — T2 fiabilité (PR 147, tête 363e918e) — 2026-11-24

Relecteur `review-sonnet`, lecture seule. **Aucun bloquant.**

## Conforme
- Variantes `_conn` (`exiger_eleve_visible_conn`, `a_action_etablissement_conn`) : logique d'autorisation identique, même contexte RLS ; aucun emprunt du pool resté dans une transaction dans le périmètre modifié (vie.rs 654, 1680, 1687, 1798, 1804, 2193 ; groupes.rs 741). Témoin réel : `t2_lever_une_vigilance_sur_un_pool_d_une_connexion`.
- Contrôle de démarrage : requête paramétrée sur le catalogue, aucune donnée personnelle, ne bloque pas le démarrage.
- Pas d'heure réelle introduite dans les tests internat.

## Moyen
- **M1** : `assiduite_tableau` lit `portail_aujourdhui().jour`, qui est le jour de Paris côté serveur (`maintenant_local`), pas `ent_aujourdhui_etab` ; commentaire et compte rendu faux ; cause de l'échec CI non établie. → **Corrigé par le pilote** (commentaire et compte rendu rectifiés) ; en file : lire le jour par une route qui renvoie `ent_aujourdhui_etab`.

## Faibles (en file)
1. Détecteur de clair : la branche base64 est désactivée pour le nom (10 octets, seuil `> 12`) ; nom plus long ou seuil abaissé avec témoin.
2. Variantes `_conn` : aucune vérification que la connexion porte le bon établissement (sens permissif si mal appelée) ; assertion de debug ou signature `&mut TenantTx`.
3. Exemption de `controle_demarrage` dans `garde_rls_stricte` au niveau fichier : la remplacer par une exception ciblée sur la ligne.
4. Contrôle de démarrage peu testé (`FonctionAbsente`, émission de l'`error!`) ; dernière assertion tautologique.

## Verdict
Versable après M1 (fait).
