# Socle 7 — lots D2 (fondations) et D3 (moteur et premières natures) : livraison

Conception suivie : `docs/socles/07_DOCUMENTS.md` §5.3, §6 à §9, §14, §17 à §19, §22, §23 (branche `ent-mc0hoe`), D-026, D-062, D-063.
Code : `ent-db/src/documents.rs` (types, natures, ports, règles pures), `documents_repo.rs` (service de la brique),
`ent-auth/src/actions.rs` (`socle_07_documents`). Migrations **3700** à **3703**. Tests : `tests_documents_d2.rs`.

## Lot D2 — fondations de la brique

### Implémenté et testé (labo local)

| Sujet | Où | Test |
|---|---|---|
| Documents, versions **immuables** (ajout seul : triggers), données figées **scellées** (`champs_chiffres`), exemplaires nominatifs | migration 3701 | `t_version_emise_immuable`, `t_document_identite_immuable`, `t_emission_numero_code_exemplaire_et_journal` |
| **Numérotation sans trou** sous concurrence (une transaction d'émission, un compteur par entité × série × année) ; un échec (rendu, cachet, moteur) n'use aucun numéro | migration 3702, `ent_doc_numero_suivant` | `t_numerotation_concurrente_sans_trou`, `t_echec_n_use_pas_de_numero` |
| Mises à disposition (coffre, papier, guichet…), accusé d'ouverture posé une fois, consultations C2+ tracées, journal en ajout seul | migration 3703 | `t_emission_…`, `t_consultations_c2_tracees` |
| Natures déclarées par le code, paramétrage (activation, style, textes encadrés), identité (4 styles, contraste AA), entités émettrices et séries par entité (R-D13) | `documents.rs`, migration 3700 | `t_parametrage_identite_et_natures`, tests unitaires de `documents.rs` |
| **RLS forcée** sur les 14 tables, registre des tables (E6, domaine `documents`), compte RLS de `tests_rls.rs` (+14) | 3700–3703, `registre_donnees.rs`, `tests_rls.rs` | `tests_rls::rls_politiques_presentes_et_forcees`, `registre_donnees::…chaque_table…`, `t_separation_entre_etablissements` |
| **D7** : un exemplaire ne porte jamais que l'adresse de SON destinataire (CHECK en base) ; la version de base n'en porte aucune | 3701 `ent_doc_exemplaire_adresse_chk` | `t_d7_adresse_jamais_celle_d_un_autre` |
| **D8** : parents séparés → un exemplaire par titulaire, même numéro, rangs distincts, deux coffres ; chacun ne lit que le sien | `emettre_document_for_actor` | `t_d8_deux_titulaires_deux_exemplaires` |
| **D9** : réémission = nouvelle version, motif obligatoire, même numéro, **un nouveau code par version** (celui de la version remplacée répond « remplacé le … »), ancienne mise à disposition retirée, coffre sur la nouvelle ; refusée si la nature se corrige par document compensatoire | `reemettre_document_for_actor` | `t_d9_reemission` |
| **D10** : annulation ★ (motif, journal, retrait des mises à disposition, `documents.annule`), **aucun message automatique** | `annuler_document_for_actor` | `t_d10_annulation` |
| Droits : `documents.emettre`, `.annuler`, `.registre.lire`, `.modeles.parametrer`, `.restreints.lire`, `.financier.lire`, `.personnels.lire` (fonction `socle_07_documents`) ; le **domaine** du document choisit l'action de lecture (R-D6) ; titre neutre hors droit (RP1-05) ; C4 servi à personne | `actions.rs`, `lire_document_for_actor`, `registre_for_actor` | `t_droits_emission`, `t_registre_titre_neutre`, `t_c4_servi_a_personne`, `ent-auth::documents_d2_modeles_par_defaut` |
| Événements `documents.emis`, `.remplace`, `.annule`, `.mis_a_disposition`, `.ouvert` (charge utile : identifiants, nature, sensibilité ; jamais de titre ni de contenu), agrégat `Documents` | `documents_repo.rs`, `ent-events` | `t_emission_…` |

### Écarts assumés avec la conception (à valider)

- **D-036** : pas de JSON, de tableau ni de BYTEA. `textes_encadres` devient la table `ent_doc_nature_texte` ; `identifiants` de l'entité, la table
  `ent_doc_entite_identifiant` ; `donnees_scellees BYTEA`, des lignes `ent_doc_version_donnee` (valeur scellée) ; `langues TEXT[]` de l'identité n'est pas reprise
  (la langue est portée par la version ; les natures D3 sont en français).
- **Fichiers** : le noyau n'a pas de `ent_fichier` (lot noyau, Q-D4) ; une version et un exemplaire pointent un objet du registre `ent_objet` (migration 500) par
  clé composite. Les octets ne sont jamais en base. Pas de chiffrement par fichier ni de verrouillage d'objet (§17.4) à ce stade.
- **Portées** : les sept actions ne sont admises qu'à l'échelle de l'établissement (la conception parle aussi de « désignés » pour `emettre`) ; le droit par élève
  reste exercé par `autoriser_eleve` sur le sujet du document.
- **Doublure de modèle** : `documents.registre.lire` n'est donné qu'à la direction et au secrétariat ; le CPE et la comptabilité émettent mais ne relisent pas
  (le modèle historique `documents.lire` ne couvre que Admin, Direction, Administratif : il n'a pas été modifié, règle 21).
- **Code de vérification par version** (et non par document, §8.3) : D9 exige que le code de l'ancienne version réponde « remplacé le … » ; le code est donc une colonne de `ent_doc_version`.
- **Brouillon** : l'émission écrit directement un document « émis » (pas de brouillon sans parapheur, qui vient avec D6) ; le modèle accepte déjà `brouillon`.
- **Fusion d'élèves** : `ent_doc_document` et `ent_doc_mise_a_disposition` sont déclarées `Reste` (immuables) ; le rattachement à la fiche conservée
  (§19.2) sera la commande de classement `documents.classer`, à ajouter avec D4.

### Non fait dans D2 (volontairement)

Brouillons et parapheur (D6), circuits et signatures (D6), pièces et dépôts (D5), coffre complet et notifications par événements (D4), lots et courrier (D7),
conservation, gels et sort final (D9), registre du courrier (D10), reprise des anciennes tables (D15), contrat IA (D14), écrans (D12, D-063), routes HTTP.

---

## Lot D3 — moteur et premières natures (certificat de scolarité, attestation d'inscription)

Code : `ent-db/src/documents_natures.rs`, routes `ent-api/src/documents_verif.rs`. **Aucune migration** (tables de D2). Tests : `ent-db/src/tests_documents_d3.rs`,
`ent-api/tests/documents_verif_publique.rs`, matrice de séparation (`separation.rs` : deux routes publiques inscrites).

### Implémenté et testé (labo local)

| Sujet | Où | Test |
|---|---|---|
| **Certificat de scolarité** (`eleve.certificat_scolarite`, série SCO) et **attestation d'inscription** (`inscriptions.attestation_inscription`, série ADM) : un geste, cachet de l'établissement, code de vérification, exemplaire nominatif par titulaire, coffre | `emettre_certificat_scolarite_for_actor`, `emettre_attestation_inscription_for_actor` | `t_certificat_pour_les_titulaires`, `t_controles_bloquants` |
| **Contrôles bloquants avant tout rendu** : élève non scolarisé à la date (certificat), sans scolarisation en cours ni à venir (attestation), date de naissance exacte absente, aucun destinataire ; **aucun numéro consommé, le moteur n'est pas appelé** | `donnees_attestation` | `t_controles_bloquants` |
| **Niveau seul, jamais la classe** (D-027 B : ni SEGPA, ULIS, UPE2A) : la classe n'est ni lue ni transmise au moteur | `donnees_attestation`, `en_donnees` | `t_certificat_pour_les_titulaires` (classe « ULIS-ZZ9 » absente du rendu), `donnees_sans_classe_ni_adresse` |
| **Noms de toutes origines** : octets UTF-8 transmis au moteur tels quels, données figées scellées puis rouvertes à l'identique, désignation publique en capitales | `donnees_figees` | `t_diacritiques_de_toutes_origines` (vietnamien, œ, polonais, turc, allemand, nordique, espagnol…) |
| **Adresse protégée jamais rendue** : la famille « attestation » n'imprime aucune adresse ; `adresse_postale_permise` ne renvoie jamais celle d'une personne qui a une adresse protégée (socle 4) | `adresse_postale_permise` | `t_adresse_protegee_jamais_rendue` |
| **Destinataires** : titulaires de l'autorité parentale, lien non suspendu et actif à la date ; l'élève majeur reçoit lui-même ; un parent sans le droit ne reçoit ni ne lit | `titulaires_destinataires` | `t_parent_sans_droit_de_recevoir` |
| **« Par délégation »** quand l'émetteur n'est pas le signataire désigné pour la nature (`signataire_defaut_agent_id`) | `emettre_attestation_for_actor` | `t_par_delegation` |
| **Vérification publique par code** : 3 modes (`complete`, `authenticite_seule`, `aucune`), états valide / remplacé le … / annulé le …, désignation minimale `L. DUPONT, né(e) en 2012`, réponse **identique** pour code inconnu, mal formé, d'un autre établissement ou de nature sans vérification ; le numéro séquentiel n'est jamais un code ; le motif d'annulation ne fuit pas | `verifier_code_public` | `t_verification_publique_par_code`, `t_verification_trois_modes` |
| **Vérification par dépôt** : empreinte du fichier comparée aux versions et exemplaires émis (10 Mio au plus, rien conservé) ; « remplacé » et « annulé » aussi | `verifier_par_depot` | `t_verification_par_depot` |
| **Routes sans session** `GET /verifier/{code}` et `POST /verifier` : établissement de l'**adresse**, `noindex`, `no-store`, débit limité par IP (`anti_abus`), 404 sur adresse inconnue, 413 au-delà de 10 Mio | `ent-api/src/documents_verif.rs`, `anti_abus.rs` | `documents_verif_publique.rs`, `separation.rs` (`PUBLIQUES`) |

### Contrat avec le moteur de rendu (D1)

* Le rendu passe par le trait `RenduDocument` (`documents.rs`) : le moteur reçoit des **valeurs** (`DemandeRendu` : nature, style, identité, entité, données figées,
  textes encadrés, numéro affiché, code et URL du QR, motif, mention « par délégation », destinataire et sa propre adresse). Clés de données de D3 :
  `titulaire.nom`, `titulaire.prenom`, `titulaire.naissance` (ISO), `scolarite.niveau`, `scolarite.annee`, `scolarite.date_entree` (ISO).
* Le cachet passe par `CachetEtablissement`. **Par défaut, `RenduIndisponible` et `CachetIndisponible` refusent** : tant que la brique PDF maison (`ent-socle-7-code-d1`,
  D-062) et la clé du cachet ne sont pas branchées, l'émission réelle répond « moteur de rendu indisponible » / « cachet indisponible » **sans rien écrire** (§9.5, Q-D7).
  Les tests branchent des doublures déterministes ; le PDF n'est pas réécrit ici.
* **Branchement attendu** : une session d'assemblage construit `Moteur { rendu, cachet, url_verification }` avec le moteur D1 et le cachet, et le passe aux commandes
  `emettre_*_for_actor`. Aucune route d'émission n'existe encore (écrans et API d'émission : lot D12).

### Limites connues (à ne pas présenter comme faites)

- **Rendu et cachet réels absents** (voir ci-dessus) : le critère « certificat < 1 s, PDF/A et PDF/UA valides, QR vérifiable » n'est pas démontrable avant D1. Mesure avec la
  doublure : ~80 ms de base de données pour l'émission complète (un geste).
- **Droit de lien `recevoir_documents`** (socle 4, Q-D29) non codé : en attendant, **l'autorité parentale vaut droit de recevoir** (`titulaires_destinataires`, une seule fonction à changer).
  La bascule à la majorité du coffre (D4) n'est pas faite : un élève majeur reçoit seul, ses parents plus rien, sans transition ni notification.
- **Données lues en SQL** dans `ent_eleve_identite`, `ent_eleve_scolarisation`, `ent_eleve_scolarite_annee`, `ent_eleve_lien`, `ent_personne`, `ent_adresse_protegee` :
  les lectures publiées typées du noyau (`eleves().resume`, `scolarite_au`, `droits_lien`, `adresse_postale`) n'existent pas ; les lectures sont isolées dans `documents_natures.rs`.
  L'émission lit les données dans une transaction, puis émet dans une autre (la lecture et l'écriture ne sont pas atomiques ; l'écart est de quelques millisecondes).
- **Pas de bilinguisme** (français + anglais, §9.4) : les natures déclarent `langues: ["fr"]`.
- **Pas de `libre_service`**, de comptoir (écran), de lot, ni de parapheur (D6) ; `signataire_defaut_agent_id` n'est pas vérifié contre `peut_signer` (socle 6, P6).
- **Dépôt de vérification** : le corps est lu en mémoire (10 Mio au plus) puis haché, pas en flux ; la limitation de débit est par IP seulement (pas par code : un code de 60 bits ne s'énumère pas).
- **Pas de consultation tracée** pour une vérification publique (`ent_doc_consultation` exige un acteur connecté).
- Aucune **revue sécurité** (skill `security-review`) ni revue `review-sonnet` n'a été faite : à lancer par le pilote (D2 et D3 sont listés en §23).
