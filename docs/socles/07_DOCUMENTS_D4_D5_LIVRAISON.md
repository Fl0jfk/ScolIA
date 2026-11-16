# Socle 7 — lots D4 (coffre) et D5 (pièces réelles) : livraison

Conception suivie : `docs/socles/07_DOCUMENTS.md` §11, §12, §14.6 (DCT-83 à 92), §17, §18, §19, §23 (lignes D4 et D5) (branche `ent-mc0hoe`), D-005, D-062, règle 21 de pilotage.
Branches : `ent-socle-7-code-d4`, puis `ent-socle-7-code-d5` **empilée** dessus (une seule CI, au sommet).
Code : `ent-db/src/documents_coffre.rs` (D4), `documents_zip.rs` (D4, archive de l'export), `documents_pieces.rs` (D5), `ent-object/src/clamav.rs` (D5, client ClamAV),
`ent-pdf/src/piece.rs` (D5, photos → PDF, PDF désarmé). Migration **3704**. Droits : `socle_07_documents` (`ent-auth/src/actions.rs`).
Tests : `tests_documents_d4.rs`, `tests_documents_d5.rs`, tests unitaires des modules, et les tests de D2/D3 inchangés.

## Lot D4 — le coffre

### Implémenté et testé (labo local)

| Sujet | Où | Test |
|---|---|---|
| **Coffre** du responsable (par enfant, « pour moi »), de l'élève (mineur : sa scolarité jusqu'à C2 ; majeur : tout ce qui lui est adressé) et de l'agent (« Mes documents ») ; filtres par enfant, année, famille de natures | `coffre_for_actor`, `ContexteCoffre::autorise` | `t_parents_separes_deux_coffres`, tests unitaires du droit |
| **Droit relu à chaque lecture, au jour de l'établissement** : lien avec autorité parentale, non suspendu, actif ; une mise à disposition ne vaut jamais droit. Aucun C4, aucun document annulé ni remplacé | `charger`, `autorise` | `t_parent_sans_autorite`, `t_lien_suspendu_retrait_et_rendu` |
| **Parents séparés** : deux exemplaires, deux coffres, chacun n'ouvre que le sien (un identifiant de l'autre répond « introuvable ») | `ouvrir_du_coffre_for_actor` | `t_parents_separes_deux_coffres` |
| **Lecture historique (D2)** d'un titulaire déléguée au coffre : même droit, même trace | `lire_document_for_actor` → `ouvrir_du_coffre_for_actor` | `t_parents_separes_deux_coffres`, tests D2 |
| **Accusé de première ouverture** posé **une seule fois**, même sous deux ouvertures simultanées (`UPDATE … WHERE premiere_ouverture_le IS NULL`, événement `documents.ouvert` publié par l'ouverture gagnante) ; visible du personnel (`documents.registre.lire` + droit du domaine), jamais de la famille ; liste des **non-ouverts** (relance proposée, jamais envoyée) | `accuses_for_actor`, `non_ouverts_for_actor` | `t_accuse_une_seule_fois` |
| **Export** du coffre : archive ZIP (PDF + `index.csv`, cellules protégées contre les formules), limité au droit **du jour**, borné (500 documents, 200 Mio), consultations « téléchargement » tracées pour C2 et plus, export journalisé ; archive écrite à la main, déterministe | `exporter_coffre_for_actor`, `documents_zip.rs` | `t_export_du_coffre`, `documents_zip::tests` |
| **Notifications par l'outbox du socle 8** : consommateur `d4.documents_coffre` sur `documents.mis_a_disposition` → `notifier_personne` ; **aucun contenu** (ni nature, ni titre, ni nom d'élève), **aucune pièce jointe** (les tables des canaux n'ont aucune colonne capable d'en porter : testé), regroupable ; un document retiré avant la distribution n'envoie rien | `ConsommateurCoffre` | `t_notification_sans_contenu_ni_piece_jointe` |
| **Bascule à la majorité** (`eleve.majeur`, socle 2) : mises à disposition des parents retirées (motif « majorité de l'élève »), l'élève reçoit les siennes sur un exemplaire **sans adresse**, idempotente, élève prévenu (`documents.coffre_bascule`) ; le droit relu ferme **déjà** le coffre des parents le jour des 18 ans, avant même la bascule ; le payeur (domaine financier) garde son accès | `basculer_majorite_tx`, `basculer_coffre_majorite` | `t_majeur_recoit_seul_et_bascule`, `t_evenement_majorite_bascule_et_previent` |
| **Retrait sur suspension de lien** (`lien.suspendu`) : les mises à disposition de la personne, pour cet élève, sont retirées (motif « lien suspendu », Q-D12) ; **réconciliation** pour les deux chemins du socle 4 qui suspendent sans publier ; **rendre explicitement** (`documents.classer`, droit du destinataire relu aujourd'hui) | `retirer_liens_invalides_tx`, `reconcilier_liens_suspendus`, `rendre_mise_a_disposition_for_actor` | `t_lien_suspendu_retrait_et_rendu`, `t_evenement_lien_suspendu_retire` |
| **Mettre à disposition** un document existant (DCT-87) : `documents.emettre`, droits du destinataire à la date, exemplaire du destinataire sinon exemplaire **sans adresse** (jamais celle d'un autre) ; **retirer** (DCT-88) : motif obligatoire, ouverture antérieure signalée | `mettre_a_disposition_for_actor`, `retirer_mise_a_disposition_for_actor` | `t_mise_a_disposition_et_retrait` |
| **Séparation entre établissements**, rôle sans privilège (`NOBYPASSRLS`) en mode strict | tout | `t_separation_entre_etablissements` (D4 et D5) |

### Écarts et choix à valider
- **Aucune migration pour D4** (la conception n'en prévoit pas) : « rendre » une mise à disposition retirée crée une **nouvelle** mise à disposition sur une **copie de l'exemplaire** (même fichier, rang suivant), car le déclencheur 3703 interdit d'annuler un retrait et l'index d'unicité n'est pas partiel.
- **Élève mineur** : il ne reçoit que si la nature est « scolarité » et la sensibilité au plus C2 ; à l'émission (D3) rien ne lui est adressé : il faut `mettre_a_disposition_for_actor(Eleve)`. À valider avec le socle 2 (« selon âge et nature »).
- **Après la majorité** : les parents ne gardent que le domaine « financier » (le payeur) ; aucune nature de ce domaine n'existe encore.
- **Natures qui demandent l'accusé** : reconnues par leur code (`…convocation…`, `…notification…`) ; la déclaration propre (`NatureDecl`) viendra avec le socle 13.

### Limites connues
- Export **synchrone** (tâche de fond et lien unique avec les écrans, lot D12) ; la bascule ne réémet pas de document au nom de l'élève : les documents sans exemplaire sans adresse sont comptés (`sans_exemplaire_neutre`) et à réémettre.
- Pas de coffre « après le départ » (DCT-90, Q-D17 non tranchée) ; pas de routes HTTP (écrans : D12).
- Le consommateur de notification ne relit pas le droit du lien (il vérifie seulement que la mise à disposition est toujours active) ; le droit est de toute façon relu à l'ouverture.

## Lot D5 — pièces réelles

### Implémenté et testé (labo local)

| Sujet | Où | Test |
|---|---|---|
| **Catalogue** fermé livré par ScolIA (21 types + « autre » motivé + type technique de reprise), portées foyer / élève / personne / agent, validités (3 mois, année scolaire, année civile, variable…), copie gardée ou non ; **interdits** (titre de séjour, carte Vitale, RIB, casier, religion, photo de domicile) absents du catalogue et refusés dans le motif d'« autre » | `documents_pieces.rs` | `catalogue_coherent`, `motifs_interdits`, `validites`, `t_demander_et_droits` |
| **Demandes** (`demander_piece_for_actor`) : droit `documents.pieces.demander`, droit du domaine pour les types C3, sujet cohérent avec la portée, idempotent, **proposition** d'une pièce valable déjà au dossier (jamais utilisée en silence) ; annulation motivée | idem | `t_demander_et_droits` |
| **Dépôt** : photos (1 à 50, JPEG/PNG) ou un PDF ; type réel par signature, taille, nombre, débit (40/jour/compte) ; **quarantaine** : original scellé, **aucun fichier servi** (CHECK de la base) ; famille (autorité parentale, lien actif non suspendu ; élève majeur pour lui-même), agent au guichet (`documents.deposer`), élève mineur refusé | `deposer_piece_for_actor` | `t_depots_refuses_a_l_entree`, `t_debit_des_depots` |
| **Analyse en tâche de fond** : antivirus (client ClamAV maison, protocole `clamd` INSTREAM, **fermé par défaut**) puis conversion ; **antivirus indisponible → quarantaine maintenue** (réessais comptés, une seule ligne d'historique) ; infectée → jamais servie | `analyser_piece`, `analyser_pieces_en_attente`, `ent-object::clamav` | `t_eicar_refuse_et_jamais_servi`, `t_antivirus_indisponible_quarantaine_maintenue`, `clamav::tests` |
| **Photos → un PDF** : JPEG repris sans réencodage mais **nettoyé** (EXIF dont la **position GPS**, XMP, ICC, miniature, commentaires, données après la fin d'image retirés), **orientation EXIF appliquée** (8 orientations, matrice de placement), PNG repris sans décompression (blocs annexes jetés) ; déterministe | `ent-pdf::piece` | `t_photos_en_un_pdf_sans_exif`, `piece::tests` |
| **PDF désarmé** : `/JavaScript`, `/JS`, `/Launch`, `/OpenAction`, `/AA`, `/URI`, `/EmbeddedFile`, `/XFA`… (y compris écrits avec `#xx`) **neutralisés sur place** à longueur égale (xref exacte), flux binaires jamais touchés ; chiffré ou à objets compressés **refusé** | idem | `t_pdf_javascript_desarme`, `t_pdf_non_inspectable_rejete`, `piece::tests` |
| **Contrôle en rafale** : file des pièces reçues (ancienne d'abord, avec ce que la démarche attend et ce que le désarmement a neutralisé), accepter (validité calculée) / refuser (motif type, commentaire exigé pour « autre ») / sans objet ; **une seule décision** même sous concurrence (verrou de ligne + garde de la base) ; l'accueil ne contrôle que l'identité ; C3 : droit du domaine en plus | `pieces_a_controler_for_actor`, `controler_piece_for_actor`, `controle_en_rafale_for_actor` | `t_controle_en_rafale`, `t_controle_concurrent_une_seule_decision` |
| **Réutilisation** confirmée par la famille ou l'agent, aucune nouvelle pièce ; **pièce de foyer pour deux enfants** (même `foyer_id`) : une acceptation satisfait les demandes de tout le foyer, jamais d'un autre foyer ; pièce périmée jamais proposée | `reutiliser_piece_for_actor` | `t_piece_de_foyer_pour_deux_enfants`, `t_reutilisation_confirmee` |
| **« Ce qui manque »** : à fournir / en analyse / reçue en attente / refusée (motif) / périmée (à renouveler), échéance d'abord, pièce à proposer ; filtré par le droit du domaine ; `dossier_complet_pieces` | `ce_qui_manque_for_actor` | `t_ce_qui_manque` |
| **Identité : vue seulement** : après décision, original et fichier servi marqués supprimés au registre des objets, la décision est conservée | `controler_piece_for_actor` | `t_identite_sans_copie` |
| **Retrait par le déposant** tant que la pièce n'est pas contrôlée | `retirer_depot_for_actor` | `t_retrait_par_le_deposant` |
| **Port `PortPiecesReprise`** pour la reprise N1-5 : `PiecesReprises` (table préparée d'avance, le port est synchrone et sans accès base) ; les pièces passent par le même chemin que les dépôts ; seules les saines entrent, les autres sont **signalées** (`None`) ; idempotent | `preparer_reprise_pieces` | `t_port_reprise_n1_5` |
| **Aucun fichier servi hors quarantaine levée** : la lecture refuse quarantaine, infectée, rejetée, retirée ; jamais l'original ; chaque lecture C2+ au journal | `lire_piece_for_actor`, CHECK 3704 | `t_photos_en_un_pdf_sans_exif`, `t_eicar_refuse_et_jamais_servi` |
| **Séparation entre établissements** (rôle sans privilège en mode strict) | tout | `t_separation_entre_etablissements` |

### Migration 3704 (`ent_doc_piece_demande`, `ent_doc_piece`, `ent_doc_piece_decision`) — +3 au compte RLS
Sans JSON, sans tableau, sans BYTEA, sans `current_date` (le service passe le jour de l'établissement). Garde-fous **dans la base** : fichier servi `NULL` en quarantaine / infectée / rejetée (CHECK) et écrit **une seule fois** (déclencheur) ; machine à états ; identité, sujet, déposant et original immuables ; pas de DELETE ; décisions en ajout seul ; une demande ouverte par type et sujet.

### Écarts et choix à valider
- **PDFium non retenu (D-062)** : on ne « re-rend » pas page à page ; on **neutralise sur place** et on **refuse** les PDF qu'on ne peut pas inspecter (objets compressés, chiffrés). Beaucoup de PDF modernes (Word, certains exports) utilisent des flux d'objets : ils sont **refusés avec un message** (« prenez une photo ou imprimez-le en PDF simple »). Si cela gêne en pratique, la décision à prendre est celle d'un rastériseur (D-062 : « seulement sur besoin réel »).
- **Pas de PDF/A** pour les pièces (le moteur n'écrit que du PDF 1.4) ; **WebP, GIF, HEIC refusés** (conversion côté appli) ; **PNG transparent ou entrelacé refusé**.
- **Pas de reconnaissance de caractères**, ni d'aides à la lecture (date lue, nom lu) : le contrôleur reçoit le nombre de pages et ce que le désarmement a neutralisé.
- **Foyer** = même `foyer_id` sur les liens ; deux parents séparés ont deux foyers.
- **Pièce d'identité** : portée « personne » ; la vérification d'identité « sans copie » de la conception (§11.2) est tenue par la non-conservation après décision ; l'objet n'est marqué supprimé que **logiquement** (`supprime_le`), la purge physique relève de D9.
- **Dépôts multiples de photos** : l'original est l'archive ZIP « stored » des photos reçues (type MIME `application/zip` admis par la migration).
- **Reprise** : les pièces de l'ancienne application sont **acceptées d'office** (type technique `piece_de_demande`, sujet : la personne qui les avait déposées) ; le rattachement à la demande reste celui de N1-5.

### Limites connues (à ne pas présenter comme faites)
- Aucun **déclencheur** de la tâche de fond : `analyser_pieces_en_attente` est prête, mais son branchement à l'ordonnanceur et la variable `CLAMAV_ADDR` (`ClamAv::depuis_env`) ne sont pas câblés dans `main` (lot D12 / exploitation). **Sans démon configuré, aucune pièce ne sort de quarantaine** (fermé par défaut).
- Pas de routes HTTP ni d'écrans (D12d) ; pas de notification de famille sur refus (l'événement `documents.piece_decidee` existe, le consommateur du socle 8 reste à écrire) ; pas de relance des demandes.
- Les objets d'une pièce **infectée** ou **rejetée** restent au magasin (registre immuable) : purge à D9.
- L'antivirus analyse les fichiers **reçus** ; le PDF produit n'est pas rescanné.
- Aucune **revue sécurité** (`security-review`) ni `review-sonnet` : à lancer par le pilote (D5 est listé en §23).
