# Socle 27, palier G5 — complément : console de l'éditeur, fiches PDF, écran mobile (livraison)

Branche `claude/project-thread-5nex6p`, **empilée sur la PR 166** (`claude/project-thread-rrdacj`, palier G5). Aucune
migration. Complète les trois points laissés hors périmètre au §4 de `27_G5_LIVRAISON.md`.

---

## 1. En bref

| | |
|---|---|
| Migration | aucune |
| Fiches PDF | `ent-db/src/violations/pdf.rs` (rendu pur, brique `ent-pdf`, D-062) ; `violations_fiche_pdf`, `violations_registre_pdf` dans `violations.rs` ; routes `GET /rgpd/violations/{id}/fiche.pdf` et `GET /rgpd/violations/registre.pdf` |
| Console de l'éditeur | `ent-api/src/console_violations.rs`, sous-commande `ent-api plateforme-violation …` (aiguillage dans `main.rs`) ; lectures ajoutées dans `violations_plateforme.rs` |
| Mobile | écran « Signaler un incident » dans les deux applis (`Signaler.kt`, `Signaler.swift`), cœur `Signalement` (Kotlin, Swift), deux routes ajoutées au contrat MB2 (`contrat/mobile/routes.txt`, fixtures dorées) |
| Web | liens « Registre en PDF (filtre appliqué) » et « Fiche PDF » sur les écrans du registre |
| `#[ignore]` ajoutés | aucun |

## 2. Fait

### 2.1 Fiches PDF du registre (NX27-18)

* **Fiche d'une violation** : en bref (référence, origine, état, nature, prise de connaissance, échéance CNIL et
  retard éventuel), signalement de l'éditeur pour une violation de plateforme, qualification (catégories avec leur
  libellé du catalogue, nombre de personnes, mineurs, conséquences), risque (proposition de la grille et raisons,
  avis du DPO, décision), notification CNIL (initiale et complémentaires, ou non-notification motivée),
  information des personnes (ou exception motivée), mesures, clôture et retour d'expérience, chronologie. Une
  rubrique non renseignée le dit, et la fiche reprend « ce qui manque pour clore ».
* **Registre** : un tableau paginé (référence, connue le, nature, risque, état, CNIL), le filtre appliqué écrit en
  clair en tête (« état « à qualifier » », « à notifier »…).
* Droits : `violations.gerer` (comme la fiche à l'écran). Professeur : 403 ; violation d'un autre établissement :
  404 (cloisonnement RLS, test).
* Instants rendus dans le **fuseau de l'établissement** (une seule requête SQL pour toute la fiche).
* Réponse `application/pdf`, `Content-Disposition: attachment`, `Cache-Control: no-store`.

### 2.2 Console de l'éditeur (RGP-254, RGP-255, RGP-311)

Commande d'opérateur, lancée sur un serveur de l'éditeur avec la base de l'environnement visé :

```
ent-api plateforme-violation lister
ent-api plateforme-violation voir P-2027-002
ent-api plateforme-violation declarer --intitule … --description … --connue-le 2027-03-01T09:00:00+01:00 \
        --atteinte confidentialite,integrite --par "nom du déclarant" [--couche-compte]
ent-api plateforme-violation propager P-2027-002 <etablissement-uuid> --elements … [--personnes N] [--confirmer <slug>]
ent-api plateforme-violation notifier-cnil P-2027-002 --le 2027-03-03T10:00:00+01:00 --reference <référence CNIL> [--confirmer <référence CNIL>]
ent-api plateforme-violation clore P-2027-002 --confirmer P-2027-002
```

* `voir` liste les propagations : établissement, instant où il a été informé, référence locale, nombre de
  personnes transmis. **Jamais** la qualification, le risque ni la décision de l'établissement.
* `propager` vérifie les éléments dès l'aperçu et dit si l'établissement est déjà informé. Sans `--confirmer`, il affiche un **aperçu** (établissement ciblé, éléments transmis) et sort en code 2 ;
  un slug de confirmation différent est refusé. La propagation reste idempotente (verrou de G5).
* `notifier-cnil` : ScolIA saisit sa propre notification, couche compte seulement, une fois et sans correction
  possible : aperçu (code 2) puis `--confirmer` avec la même référence CNIL.
* `clore` exige la référence en confirmation ; les violations déjà propagées restent instruites par chaque
  établissement.
* Codes de sortie : 0 succès, 1 erreur, 2 aperçu (confirmation attendue).
* Instants en RFC 3339 **avec décalage** obligatoire ; options inconnues ou répétées refusées.

### 2.3 Déclarer depuis le téléphone (RGP-240 à RGP-242, D-003)

* Module « Signaler un incident » dans les deux applis, accordé par `violations.declarer` (tout personnel), rangé
  dans « Plus ».
* Une phrase obligatoire (3 à 200 caractères), des précisions facultatives (4 000 au plus), la case « aucune
  donnée personnelle n'est touchée ». Mêmes bornes que le serveur, testées dans le cœur.
* **Confirmation** avant l'envoi (« Prévenir la direction ? ») ; envoi avec `Idempotency-Key` (`ClesIntention`) :
  une nouvelle tentative après une coupure ne crée pas deux violations.
* Reçu (référence et message), puis « Mes signalements » avec l'avancement seul (reçue, en cours, traitée).
* Hors ligne : rien ne part, message explicite (« en cas d'urgence, appelez la direction »).
* Contrat MB2 : `POST` et `GET /rgpd/violations/declarations` ajoutés à la liste blanche, fixtures dorées
  générées par le parcours de `tests_contrat_mobile.rs` et décodées par les deux cœurs.

## 3. Défauts prudents (D-069), à relire par Florian

1. PDF **rendus à la demande, jamais stockés** ; en-tête textuel (« {établissement} — registre des violations de
   données ») en attendant le gabarit du socle 7 ; mention « Document interne … Ne pas diffuser » en pied.
2. Les PDF ne nomment **ni le déclarant ni aucune personne concernée** (minimisation) : nombres seulement.
3. La console est une **commande d'opérateur sans route HTTP** : pas de surface d'attaque web pour l'éditeur tant
   que la console plateforme web n'existe pas.
4. Propagation confirmée par le slug de l'établissement, notification CNIL par sa référence, clôture par la
   référence de la violation (erreur de copier-coller). `declarer` n'a pas de confirmation (rien ne part encore) ;
   une relance après coupure peut créer un doublon, visible par `lister`.
5. L'éditeur ne voit de chaque établissement que la propagation elle-même (quand, référence locale).
6. Mobile : pas de champ « depuis quand » (le web le garde) ; pas de cache hors ligne des déclarations ; pas de
   brouillon envoyé plus tard (une déclaration doit partir tout de suite ou par téléphone).

## 4. Non fait (et pourquoi)

* Qualification, risque, CNIL et registre sur mobile : restent sur le web (D-003).
* Gabarit graphique des PDF : socle 7.
* Console plateforme web (routes de l'éditeur, authentification forte de l'opérateur) : à concevoir avec le socle
  des opérations ; la commande en tient lieu.

## 5. Vérifications locales

PostgreSQL 16 local (`ent_ci_test`), Rust 1.99.0, `ENT_ENV=test`, compilation sur un ou deux cœurs (mémoire).

| Vérification | Résultat |
|---|---|
| `ent-db` (labo) : tests `violations` et `g5` (dont `g5c_*` et les rendus PDF), `garde_rls_stricte` | verts |
| `ent-api` (labo) : `g5_http_*` (dont `g5_http_fiches_pdf`), `contrat_mobile_fixtures_dorees`, `contrat_mobile_banc_de_parite` | verts |
| `ent-api` (binaire) : analyse des arguments de la console | verts |
| Fixtures MB2 régénérées (`ENT_CONTRAT_ECRIRE=1`) | 2 nouvelles ; les autres fixtures ne changeaient que par l'ordre du labo local, non reprises |
| clippy `-D warnings` (`ent-db`, `ent-api`, labo, lib, binaires, tests ; `ent-web-app`, `ent-web-api`) | verts |
| Frontend : tests `rgpd_violations` d'`ent-web-app`, `cargo fmt --check` du frontend | verts |
| Android : `:core:test` (`SignalementTest`, `ContratMb2Test`…) | verts ; tests Compose de l'appli : CI (pas de SDK) |
| iOS : `swift test` | CI (pas de Swift dans le conteneur) |
| Suite complète | CI |

## 6. Contre-revue indépendante

Rien de critique. Corrigé :

| Constat | Correction |
|---|---|
| Heure d'édition des PDF en UTC brut (`Utc::now()` en nanosecondes, introuvable après l'aller-retour Postgres) | instants retrouvés par rang (`WITH ORDINALITY`) |
| iOS : appli trop ancienne non renvoyée vers la mise à jour | `appliTropAncienne` traité comme ailleurs |
| Registre PDF plafonné à 1 000 lignes sans le dire | avertissement en tête, test |
| Longueurs comptées autrement que le serveur (UTF-16, graphèmes), caractères de contrôle refusés par le serveur | scalaires Unicode des deux côtés, tabulations et retours chariot normalisés, tests |
| Clé d'idempotence Kotlin tirée d'un `hashCode` 32 bits | contenu entier, comme en Swift |
| Reçu et erreur précédents laissés à l'écran pendant un nouvel envoi | effacés au départ, des deux côtés |
| Aperçu de `propager` trompeur si l'établissement est déjà informé ; éléments validés trop tard | dit « déjà propagée », valide dès l'aperçu |
| `notifier-cnil` définitif sans confirmation | aperçu puis `--confirmer <référence CNIL>` |

Test labo : la console lit la fiche `ent_etablissement` (slug, nom) ; la liste des propagations la joint à gauche
pour ne rien perdre si une fiche manque.

## 7. Conflits attendus à la fusion

* `ent-api/src/main.rs` (aiguillage de la sous-commande) : touché aussi par d'autres PR ; ajout de quelques lignes.
* `Cargo.lock` (`ent-pdf` en dépendance de test d'`ent-api`).
* `contrat/mobile/routes.txt` et fixtures `contrat/mobile/*.json` si une autre PR ajoute des routes mobiles.
* `Capacites.kt` / `Capacites.swift` (ordre des modules).
