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
