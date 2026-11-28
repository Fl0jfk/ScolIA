# Socle 11 — palier MB2 : fixtures dorées, parité web/appli, liste blanche (livraison)

Branche `ent-socle-11-palier-mb2` (base `origin/ENT` = `6c40bc86`), règle 39 de `PILOTAGE.md` : une branche, lots sans PR avec contre-revue par lot, une seule PR et une seule CI à la fin. Conception : `11_APPLICATIONS_MOBILES.md` §9 (MB2), §10 ; restes de `11_MB1_CLIENT_LIVRAISON.md` (session.expiree côté serveur, M2 à M5, clé gardée par les écrans, écran 426).

**Aucune migration, aucun `Cargo.toml` ni `Cargo.lock` modifié.** Rust touché : `ent-api` seulement (`enveloppe.rs`, deux handlers de `routes.rs`, tests).

## Lots
| Lot | Contenu |
|---|---|
| 1 (serveur) | 401 `session.expiree` sur jeton mort, banc `tests_contrat_mobile.rs`, fixtures `ent-rust/contrat/mobile/`, liste blanche `routes.txt`, filtre CI |
| 2 (contrat dans les cœurs) | `ContratMb2Test` (Kotlin), `ContratMb2Tests` (Swift) ; iOS rattrape Android ; 3 écrans cassés corrigés |
| 3 (restes) | écran 426, `ClesIntention`, M2 invitation, M3 session courte, M4 doc, M5 appareil sans verrou |
| Correctifs de contre-revue | voir plus bas |

## Fait
| Réf. | Livrable | Où |
|---|---|---|
| Reste MB1 (E1) | Un 401 avec jeton donne `session.expiree`, y compris sur `/auth/me` et `/auth/password` ; `auth.refusee` seulement sans jeton, sur les routes sans session (connexion, récupération, SSO), ou quand la route le pose elle-même (`refus_identifiants` : mot de passe actuel refusé sur `/auth/password`, attestation refusée sur `/auth/passkey/register/finish`). Les deux cœurs se fient au code sur ces routes | `enveloppe.rs`, `routes.rs`, `ApiClient.kt`, `ApiClient.swift` |
| MB2 fixtures | 35 fixtures : chaque route des applis jouée en HTTP sur un établissement de labo jetable (déclaré et rattaché à un domaine comme en production), réponse normalisée (UUID renumérotés, instants et dates fixes, secrets et courriels masqués), comparée **par forme** (mêmes clés, mêmes types, liste vide d'un seul côté signalée). `ENT_CONTRAT_ECRIRE=1` régénère | `ent-api/src/tests_contrat_mobile.rs`, `ent-rust/contrat/mobile/*.json` |
| MB2 liste blanche | `routes.txt` : chaque route jouée y figure, chaque route listée est jouée ou expliquée (`NON_JOUEES` : fin de cérémonie passkey, MB4) ; aucune route historique fermée | idem |
| MB2 parité | 9 personas (prof, autre prof, CPE, direction, administratif, parents de Harry et de Draco, élèves Harry et Draco) × les lectures des applis : même statut et même corps avec et sans `X-ScolIA-Appli` ; en erreur, l'appli reçoit toujours l'enveloppe codée ; **cloisonnement explicite** (parent sur l'enfant d'un autre, élève sur la vie scolaire, professeur d'un autre établissement) | idem |
| MB2 cœurs | Kotlin et Swift décodent chaque fixture par la vraie méthode du client, vérifient la route et la liste blanche ; fixture sans cas, cas sans fixture ou (Kotlin) méthode publique sans cas = rouge. Vérifié : retirer un champ requis d'une fixture rend le test rouge | `ContratMb2Test.kt`, `ContratMb2Tests.swift` |
| MB2 trouvailles | Le banc a trouvé 3 écrans cassés : **saisie de notes** (`POST /eleves/{id}/notes` fermée en 410 : écran retiré, à porter sur `/eva/*`) ; **cahier de textes du personnel** (403 sans `classe` : choix parmi les classes des séances du jour) ; **annonces** pour l'élève et la famille (`/annonces` : 403 ; désormais `/eleves/{id}/annonces`) | écrans Android et iOS |
| iOS = Android | `moyennes`, `devoirsEleve`, `notifications` (`NotificationEnt`), `absencesATraiter(jour:)`, `annoncesEleve`, `cahierTexte(classe:)` | `ScolIACore` |
| 426 | Écran « Mise à jour nécessaire » (lien magasin, changement d'établissement) dès qu'une réponse le dit (chargement, action, reprise de session, écran d'appel) | `Commun.kt`, `Racine.kt`, `AppModele.*`, `Racine.swift` |
| §5.5 (M3 lot 2 de MB1) | `ClesIntention` : clé gardée après un échec incertain (réseau, 5xx, 429, 409 `idempotence.en_cours`), oubliée après succès ou refus définitif ; intentions sœurs oubliées après un succès (changer d'état d'un élève) ; branché sur ouvrir/marquer/clore l'appel, justifier, répondre | `HorsLigne.kt`, `HorsLigne.swift`, écrans |
| M2 | Code d'invitation lu et validé, jamais gardé ni envoyé (pas de route d'activation serveur) | `Rattachement.kt`, `AppModele.kt` |
| M3 | Bandeau « Session courte » (session sans passkey : durée web) ; ajout de passkey depuis Mon compte | `Avertissement`, `EcranProfil` |
| M4 | Build Debug et passkeys documentés | `MOBILE.md` §8 bis |
| M5 | Bandeau « Téléphone sans verrouillage », recalculé à chaque reprise | `MainActivity.kt`, `Racine.swift`, `Securite.swift` |
| CI | Jobs mobiles requis aussi quand `ent-rust/contrat/**` change | `.github/workflows/ent-rust.yml` |

## Règles non tranchées → valeur par défaut prudente (D-069)
1. **Saisie de notes mobile** : retirée plutôt que laissée en échec ; le portage sur la grille `/eva/*` est un palier à part.
2. **Comparaison par forme, pas par valeur** : les valeurs varient avec le jour ; seules les clés, les types et le vide/non-vide comptent.
3. **Intention d'une réponse ou d'une justification** : le texte normalisé lui-même (pas un hachage).

## Non fait (et pourquoi)
- `:app` Android et `ScolIAUI` non compilés en local (SDK Android et Mac indisponibles) : `:app` est compilé par la CI ; SwiftUI reste en passe Mac.
- Fin de cérémonie passkey (`login/finish`, `register/finish`) sans fixture : authentificateur réel requis (MB4).
- `ignoreUnknownKeys` côté Kotlin et décodage Swift tolérant : un champ **ajouté** côté serveur ne casse pas les cœurs (voulu) ; un champ renommé qui a une valeur par défaut côté client non plus (limite connue, à traiter par une liste des clés attendues par DTO si besoin).
- Opérations différées durables (MB7), SBOM et versions minimales (MB14), lecture des notes sur le modèle `eva_*`.
- Liste blanche : `GET /note-periodes`, `/devoirs`, `/classes/{classe}/effectif` restent permises (lectures de notes et de devoirs) ; à réduire si aucun écran ne s'en sert après le portage de la saisie.

## Tests (local, 2026-11-27)
- Rust : banc MB2 (fixtures, parité, cloisonnement), tests 401 (unitaires et HTTP), `tests_mobile_mb1` : verts ; suite complète du workspace avec labo et clippy : voir la PR.
- Cœur Kotlin : **65/65** ; cœur Swift : **58/58** (`swift:6.1-noble`).

## Contre-revues
- Lot 1 : aucun élevé. M1 (parité sans cloisonnement prouvé) **corrigé** (cellules de refus explicites, autre établissement) ; M2 (listes vides) **corrigé en partie** (vide d'un seul côté signalé) ; M3 (exceptions 401 par chemin, dont `register/finish` en 403) **corrigé** (code posé par les routes, chemins d'exception supprimés côté serveur, cœurs alignés sur le code) ; M4 (passkey sans fixture) **corrigé** pour `login/start` et `register/start` ; F1 (en-tête résiduel sur la connexion) **corrigé** ; F2 (masquage) **corrigé** ; F3–F5 en file.
- Lots 2 et 3 : 1 élevé (clé gardée réutilisée après un changement d'état) **corrigé** (intentions sœurs) ; M2 (bandeau sans verrou après connexion fraîche), M3 et M4 (426 partout, sortie de l'écran), M5 (409 définitif), F8 (hachage), F9 (bouton passkey), F10 (accessibilité du bandeau) **corrigés** ; M6 (limites du décodage tolérant) documenté ; M7 (tests Robolectric des nouveaux écrans) et F11–F13 en file.
