# Contre-revue — MB1 serveur (PR 157, tête 9a0d2aea) — 2026-11-25

Relecteur `review-sonnet`, lecture seule. **Aucun élevé** : aucun accès aux données d'autrui. Verdict : versable après correctifs.

## Conforme
- Cloisonnement : clé primaire `(etablissement_id, session_id, cle)`, `TenantTx` sous RLS, identifiants issus de la session résolue (jamais du client) ; test d'isolement entre sessions et personnes.
- Empreinte (méthode, chemin et requête, corps, préfixés par longueur) ; 409 sur corps différent ; clé validée (UUID) ; concurrence (`ON CONFLICT DO NOTHING`, un seul gagnant, attente ≤ 10 s puis 409 `en_cours`) ; échec non conservé ; taille bornée ; purge à horloge injectée ; registre : non exportée, neutralisée à la restauration ; RLS posée ; 2227 en fin de tableau.
- Routes `/mobile/labo/*` sous `#[cfg(feature = "labo")]`, interdite au Dockerfile de production.
- Enveloppe : 401, 403, 404, 500 neutres ; en-tête interne retiré ; web sans `X-ScolIA-Appli` inchangé ; en-tête forgé sans élévation ; `trace` filtrée.

## Moyens
1. **M1, réponses stockées en clair 24 h** (santé, psy, finances déchiffrées, liens pré-signés). → **Corrigé par le pilote** : routes sensibles (santé, psy, finances, facturation, paiement, IBAN, liens) : aucune réponse conservée (rejeu = 409 non rejouable, jamais une seconde exécution).
2. **M2, double exécution possible** (client déconnecté après le commit, échec de conservation sur `CHECK` de longueur ou `\0`, reprise après 150 s). → **Corrigé par le pilote** : exécution et conservation dans une tâche détachée ; valeurs hors bornes → non rejouable ; échec de conservation → seconde tentative « non rejouable », jamais « en cours ». Reste en file : écriture de la clé dans la transaction métier pour les écritures critiques ; délai de reprise > délai effectif de requête.
3. **M3, empreinte non salée sur des corps secrets** (`/auth/password`). → **Corrigé par le pilote** : `/auth/*` hors registre ; routes sensibles : empreinte sans le corps. HMAC : en file.

## Faibles (en file)
F1 messages 400/409/422/503 relayés (test négatif contre les fragments SQL) ; F2 rejeu à l'intérieur des garde-fous de tenant / santé / casquette (à documenter ou tester) ; F3 interrogation en boucle et absence de plafond par session ; F4 retrait de `x-scolia-code` dépendant de l'ordre des couches (test) ; F5 `/mobile/labo` sans garde `ENT_ENV=lab` et absent des motifs interdits du Dockerfile ; F6 lecture en mémoire de tout corps avec clé ; F7 purge périodique en `Utc::now()`. Tests manquants : 5xx, non rejouable, reprise après abandon, échec de conservation, délai d'attente, autre établissement, rejeu `?x=1` et DELETE, web avec clé, absence de fragment SQL.
