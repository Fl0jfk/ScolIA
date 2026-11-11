# Chantier D-053 — état des correctifs Next.js (2026-11-11, 12:40)

Session de correction : session_01KZCbFDpXEDs6D1GX3TCxCF. Branches : `nextjs-correctifs-securite-2` (sur main 8d626e9c) et `nextjs-correctifs-lot1`. Ignorer `nextjs-correctifs-securite` (obsolète).

**En production (main)** : 2a 7bb6e19a (NJ-04), 2b a6ce007f (NJ-33, NJ-46), 2c 188a1504 (NJ-36, NJ-37), 4a 2c9c7867 (NJ-09, NJ-29, NJ-39), 4c 8d626e9c (NJ-10 partiel).

**Débloqué** : Florian autorise le pilote à pousser sur main (2026-11-11). Poussés : main 0ead5b27 = les 8 invisibles ci-dessous.

**Invisibles, à pousser dans cet ordre (cherry-pick)** : a792f793, 47b5a268, 100457a8, 48f3bf13, 8b5e3eaf, 474718e9, e45fb957, 1194fc1e (6b : les routes refusaient déjà si le secret manque ; aucun changement pour une route qui fonctionne).

**En attente d'accord de Florian** (tableau a–o du message du 2026-11-11) : lot1 61e0abfb + 5f75064d (a) ; b à préparer ; f56ff6bf + 78b3a3c9 (c) ; 339e456c (d) ; 2f71d178 (e) ; 6f30c21d (f) ; fcdbf1a1 (g) ; dae4d703 (h) ; 7776a61c (i) ; 3ce284f5 (j) ; cf56dd5b (k) ; 077dfc75 (l) ; 76487d88 (m) ; d29aa6f5 (n) ; 06d8f0a2 (o).

**Variables à faire vérifier (noms seulement)** : OCR_WORKER_SECRET, TRAVEL_EMAIL_INGEST_SECRET ou INGEST_SECRET, PHOTOS_WORKER_SECRET, TRAVELS_CRON_SECRET, INTERNAT_CRON_SECRET, PORTES_OUVERTES_CRON_SECRET, RDV_INSCRIPTION_CRON_SECRET, OMOGEN_CRON_SECRET, BILLING_CRON_SECRET, EASYTRANSAC_WEBHOOK_SECRET ; secrets de repli : BIEN_ETRE_SESSION_SECRET, BETTER_AUTH_SECRET ou AUTH_SECRET (NJ-41, NJ-48).

**Décision Florian, non codés** : NJ-02 (révocation des liens M365), 05, 06, 08, 12, 16, 17, 18, 27, 35, 40, 51, 79. **Impossibles sans migration ou conception** (corrigés en Rust) : NJ-15 (journal et chiffrement), 18 (purge paie), 21, 23, 24, 26, 54, 56. **Non examinés** : NJ-13, 25, 28, 42, 43, 47, 55.

**D-054 (2026-11-11, 13:00)** : chantier recentré puis clos. Dernier push : main c23b1cd3 (secrets de repli retirés : bien-être, photocopies, supervision ; en production la fonction refuse si la variable manque). Tout le reste (tableau a–o, pages publiques, e-mails entrants, liens à jeton, fichiers des salons, export des invitations) est renvoyé à la version Rust. Les commits « en attente » des branches de correction ne seront pas poussés.

**Ajout 2026-11-11, 13:05 — faille grave sans connexion, corrigée** : `POST /api/auth/claim-migrated` (route publique) permettait à un inconnu de choisir le mot de passe d'un compte existant sans mot de passe (invité en attente, importé, SSO seul) en ne connaissant que l'e-mail. Fermée sur main 38d263c9 : la route ne pose plus de mot de passe (404 si adresse inconnue, 409 sinon ; activation par lien e-mail ou « Mot de passe oublié »). À faire vérifier par Florian : entrées `account_claimed` du journal de sécurité qu'il ne reconnaîtrait pas. Constats non vérifiés de l'inventaire, renvoyés à Rust sauf avis contraire : `/api/demo/enter` (identifiants de démo codés en dur, gardé par SCOLA_ENV/NODE_ENV), inscription Better-Auth ouverte sur tout établissement, `GET /rdv-inscription/[direction]` qui écrit la configuration sans session, `POST /api/public/preinscriptions` sans limite.
