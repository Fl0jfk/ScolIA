# Contre-revue — cahier suites 2 (PR 154, tête 95c715be) — 2026-11-25

Relecteur `review-sonnet`, lecture seule. **Aucun élevé.** Verdict : versable après correctifs mineurs.

## Conforme
- 2420 : gel de `ent_cdt_echeance_eleve` et `ent_cdt_delai_demande` équivalent à 2419 (NEW et OLD examinés, `FOR SHARE` sur l'année, échec fermé, TRUNCATE refusé) ; `search_path` et `public.` ; fin de tableau ; rejouable.
- REVOKE de `ent_cdt_reprise` : énumération par `aclexplode`, sans nom codé en dur, `rolname <> current_user`.
- 409 lisible : message fixe, sans identifiant ni texte de la base.
- Test de déplacement discriminant (témoin, code et message, avant/après).

## Moyens
1. **Fusion** : une demande de délai **décidée** était déplacée vers le conservé alors que la prolongation qu'elle a produite (`ent_cdt_echeance_eleve`, ajout seul) reste sur l'absorbé. → **Corrigé par le pilote** (D-069) : seules les demandes **ouvertes** d'une année non figée suivent ; test étendu (demande refusée : reste).
2. **2420 recrée `ent_cdt_ecriture_gardee_fille()` déjà recréée par 2419** : pendant un rejeu sur base partagée, la version de 2419 sert un instant les déclencheurs des deux nouvelles tables (`record "r" has no field "cours_id"`) : cause plausible des 7 rouges locaux. Sans effet en production et sur base neuve. → **En file** (garde de rejeu ou rejeu en série) ; règle pour la suite : ne pas recréer dans un lot une fonction recréée par la migration précédente du même lot sans garde de rejeu.

## Faibles (en file sauf mention)
- Séparation propriétaire / rôle applicatif à écrire comme condition d'exploitation ; vérification de déploiement `has_function_privilege(<rôle applicatif>, 'public.ent_cdt_reprise(uuid)', 'EXECUTE') = false` (à ajouter à `A_FOURNIR`/déploiement).
- Test du REVOKE : verrou consultatif 7770042 non libéré en cas de panique (cascade de blocages).
- `verifier_gel` : code seul, non discriminant pour UPDATE/DELETE de `echeance_eleve` (ajout seul passe avant).
- Garde JSON limitée à 2400–2419. → **Corrigé par le pilote** (2400–2429).
- Pas de vraie fusion testée avec une année close ; garde de source des imports fragile.
- Export / effacement RGPD : une ligne restée sur l'absorbé (année close) doit être trouvée depuis le conservé (chaîne de fusion) : à vérifier.
