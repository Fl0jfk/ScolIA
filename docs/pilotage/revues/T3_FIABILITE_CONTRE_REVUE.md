# Contre-revue — T3 fiabilité (PR 155, tête 7fef39c9) — 2026-11-25

Relecteur `review-sonnet`, lecture seule. **Aucun élevé.** Verdict : versable après correctifs.

## Conforme
- Assertions de contrat des variantes `_conn` sous `#[cfg(debug_assertions)]` (vides en release, le Dockerfile construit en `--release`) : aucune panique ni requête en production ; lecture sur la connexion reçue.
- Limiteur : instant injecté, verrou jamais tenu pendant un `await`, test déterministe.
- Tests nouveaux discriminants (détecteur base64, contrat `_conn`, journal de `signaler`, famille avec attribution, avis relu, annulation après report).
- Test web `assiduite_tableau` : le jour est bien `ent_aujourdhui_etab` (localisation posée sans jour).

## Moyens
- **M1** : le limiteur, clé par contexte, écartait sans trace les échecs suivants (autres établissements, autres causes) ; compteur non observable. → **Corrigé par le pilote** : les lignes écartées sont comptées par contexte et le nombre (`supprimees`) part avec la ligne suivante. Clé par classe d'erreur : en file.
- **M2** : `lock_timeout = '3s'` dans le test de santé de la sentinelle (DROP TRIGGER = ACCESS EXCLUSIVE) : faux rouge possible derrière un écrivain voisin. → **Corrigé par le pilote** (15 s).

## Faibles (en file)
- Exemption de `controle_demarrage` dans `garde_rls_stricte` par extrait de ligne (`contains`) : exiger un nombre exact d'occurrences.
- Commentaire inexact (« établissement de labo partagé ») dans `tests_internat_i4_suites3.rs:103`.
- Assertion `Accorde` du point d'entrée : suppose que le labo se connecte en propriétaire.
- Constante « report < 5 min » non vérifiée dans le test E1.
