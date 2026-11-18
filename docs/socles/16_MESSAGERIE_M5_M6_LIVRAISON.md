# Socle 16 — lots M5 (pièces jointes) et M6 (lecture et avis) : livraison

Conception suivie : `docs/socles/16_MESSAGERIE.md` §12, §13, §23, §26, règles R-M14 à R-M16, R-M19, R-M22, R-M31 à R-M33, cas CM-46, CM-57 à CM-62, D16 ; socle 7 (D5, D5b) réutilisé sans rien recréer.
Branche : `ent-socle-16-code-m5m6`, créée depuis `ENT` (e5ddda1e), puis `origin/ENT` fusionné (lettre 0.11.24).
Migrations **488** `msg_m5_pieces` et **489** `msg_m6_lecture` (plage 488–493, voir `MIGRATIONS.md`). `MIGRATION_TAG` inchangé.
Niveau : **implémenté et testé en local, non validé par Florian, non prêt pilote.** La base PostgreSQL 16 était **démarrée** pour tous les tests (`ENT_EXIGER_BASE=1`).

## Fait

### M5 — pièces
| Sujet | Où | Test |
|---|---|---|
| Quarantaine : toute pièce naît `en_quarantaine`, rien n'est servi avant l'analyse ; original scellé (`storage_key`), fichier servi écrit **une fois** (CHECK + déclencheur) | `msg_m5.rs`, migration 488 | `m5_quarantaine_puis_pdf_desarme_servi` |
| Analyse : antivirus `clamd` du socle 7 (fermé par défaut), validation de forme relue, PDF désarmé, JPEG sans EXIF/GPS, PNG sans blocs annexes | `analyser_piece_message` | `m5_quarantaine_…`, `m5_photo_sans_gps_png_sans_texte` |
| CM-58 infectée : refusée, jamais servie, état terminal, événement neutre | idem | `m5_cm58_infectee_jamais_servie` |
| Antivirus indisponible : quarantaine maintenue, la passe s'arrête après 3 échecs, une pièce en échec ne bloque pas les autres | `analyser_pieces_messagerie_etablissement` | `m5_antivirus_indisponible_quarantaine_maintenue` |
| CM-59 : exécutable, archive, DOCX à macro refusés **à l'entrée** ; PDF chiffré refusé à l'analyse ; PDF à JavaScript refusé à l'entrée, PDF à action d'ouverture désarmé | politique de dépôt D2 + désarmement D5 | `m5_cm59_pdf_chiffre_et_formats_dangereux` |
| D16 : 20 Mio par pièce (le plafond de 256 Kio tenait au stockage en base), 5 par message | `service.rs`, migration 488 | `m5_d16_taille_et_nombre` |
| URL pré-signée de 60 s, émise après contrôle des droits ; expirée refusée ; lien vers le fichier **servi** ; chaque lien et chaque téléchargement journalisés (`ent_msg_piece_acces`, ajout seul, ni nom ni clé) | `lien_piece_message_for_actor` | `m5_url_60_secondes_expiree_refusee_et_journal` |
| Participant retiré : plus rien ; tiers : **404 uniforme** (même réponse qu'une pièce inexistante) ; pièce d'un message retiré conservée, plus servie (R-M32) | `piece_a_servir` | `m5_participant_retire_et_tiers`, `m5_r_m32_message_retire` |
| Reprise des `BYTEA` : idempotente, rejouable, simulation, rapport ; analyse a posteriori (état `saine` après analyse, §26) ; option `ent-api reprise-pieces --analyser` | `reprendre_et_analyser_pieces_messagerie` (réutilise `reprise_bytea.rs`) | `m5_reprise_bytea_rejouable_avec_rapport` |
| Carte « document » : jointe par renvoi, jamais en fichier ; droit **du lecteur** relu à chaque lecture ; ouverture avec les droits du lecteur (trace du socle 7) ; un personnel qui déclare une nature du coffre est refusé (CM-60) | `joindre_carte_document_for_actor`, `cartes_du_fil_for_actor`, `ouvrir_carte_document_for_actor` | `m5_cm60_carte_document_droits_relus` |
| Isolation entre établissements dans toutes les requêtes | tout | `m5_isolation_entre_etablissements`, `isolation_sql_statique` |

### M6 — lecture et avis
| Sujet | Où | Test |
|---|---|---|
| « Lu jusqu'à » par participant (personne, élève, **boîte** : premier membre qui ouvre), ne recule jamais | `msg_m6.rs`, migration 489 | `m6_lu_jusqua_ne_recule_pas`, `m6_r_m19_…` |
| R-M19 : seule une participation active pose la lecture ; direction hors boîte, membre d'une autre boîte, supervision : 404 et rien posé ; un auteur n'accuse pas ses messages | `poser_lu_jusqua` | `m6_r_m19_premier_membre_boite_et_hors_participation`, `m6_cm62_…` |
| CM-62, Q-M8, R-M22 : la famille voit « lu par l'établissement » ou « lu par Prénom X. », **sans heure**, jamais le nom d'un membre de boîte ; le personnel voit jusqu'où et depuis quand | `lecture_du_fil_for_actor` | idem |
| CM-61 : « non lu après 3 jours » côté personnel | idem | `m6_cm61_non_lu_apres_trois_jours` |
| Prise en charge visible : état neutre côté famille (clé `pris_par` absente), nom pour les membres de la boîte | idem | `m6_prise_en_charge_visible_et_reponse_attendue_neutre` |
| Disponibilité (horaires de la boîte, plages de joignabilité d'un unique interlocuteur), à l'heure de l'établissement | `disponibilite_a` | `m6_disponibilite_horaires_de_la_boite` + 3 tests unitaires |
| Natures du §23 déclarées au socle 8 : `nouveau_message`, `nouveau_message_boite`, `reponse_attendue`, `participant_ajoute` (enveloppes sans variable) ; lint d'assemblage vert | `ntf/catalogue.rs` | `m6_natures_declarees_et_aucun_contenu_dans_l_outbox` |
| `reponse_attendue` émise à la famille (texte du gabarit, une relance au plus) ; aucun contenu, objet ni nom dans l'outbox ni dans les notifications ; événements M5/M6 sans élève (C3) ni lecteur | `avertir_reponse_attendue` | idem |

### Routes (5) et tâche de fond
`POST /messages/{id}/cartes/document`, `GET /messages/fils/{id}/cartes`, `GET /messages/cartes/{id}/document`, `GET /messages/fils/{id}/lecture`, `POST /messages/fils/{id}/lu` ; `MsgPieceDto` porte `etat` et `libelle_etat` (« pièce refusée (sécurité) » pour CM-58) ; l'ordonnanceur lance l'analyse des pièces de messagerie à côté de celle du socle 7 (**seulement si `CLAMAV_ADDR` est configurée**).

### Fichiers réels
`ent-rust/crates/ent-db/fixtures/messagerie/` : PDF (reportlab), PDF chiffré et à action d'ouverture (pypdf), JPEG à EXIF/GPS et PNG à bloc texte (Pillow), DOCX (python-docx), exécutable ELF (gcc). `generer.py` les reproduit ; un seul fichier est retouché à la main (`lettre_macro.docx`).

## Vérifications exécutées
- `cargo fmt` : `ent-types` et les cinq paquets web (`--check`) ; **jamais `--all`**.
- clippy **1.99** `-D warnings` : workspace, workspace `--features ent-api/labo,ent-lab/labo`, puis `-p ent-types` : **0 erreur, 0 nouvel avertissement** dans les lignes ajoutées (rapport par règle 31). Puis `ent-db --features ent-db/labo` après fusion d'ENT.
- Tous les `--locked`. `web/Cargo.lock` : `cargo +1.99.0 metadata --locked` vert ; lettre 0.11.24 dans les deux verrous.
- ent-db, feature `labo`, base démarrée, **passe finale : 227 réussis, 0 échec** (messagerie M0a à M6, gardes, registre, RLS, reprise des `BYTEA`, ntf, documents D4 et D5). Un échec transitoire `tests_rls_lots::lot_eleves_responsables` (« tuple concurrently updated », concurrence de migrations sur la base partagée) n'est pas revenu. `jour_utc_interdit` et `isolation_sql_statique` verts. Clippy `ent-db` (labo) revérifié après la fusion d'ENT.
- ent-api (labo) : `fichiers_liens` 10/10, `fichiers_depot` 7/7, `separation` 56/56.
- Aucun test à l'heure réelle ni au jour UTC (instants fixes, règle 33).

## Compteurs (règle 28)
Recalculés : tables avec RLS **+3** (`tests_rls.rs`, registre +3) ; `Action::TOUTES` inchangé (aucune action nouvelle) ; routes +5 (inventaire lu dans `routes.rs`) ; `MIGRATION_TAG` inchangé (3808, la plus haute du répertoire).

## Écarts et choses à ne pas présenter comme faites
1. **Gardes E7-E8 (élève mineur protégé)** : les routes de pièces passent par `actor_can_access_fil`, qui appelle l'autorisation du socle 2, mais **aucun test dédié** n'a été écrit pour un élève protégé.
2. **Avis par `aviser` non branchés** : le socle 8 résout les destinataires par les liens de l'élève, la messagerie par les participants du fil. Les natures sont déclarées (lint vert), mais l'émission passe par `notifier_personne` avec le texte du gabarit. Il faut une extension « destinataires explicites » du socle 8 (lot N11). `participant_ajoute` et `nouveau_message*` ne sont pas émises par ce chemin (les chemins existants, déjà neutres, restent en place).
3. Natures du §23 **non déclarées** ici : `mot_a_signer` (M7), `signalement` et `restriction` (M10), `publications.nouvelle` (M15).
4. **HEIC non pris en charge** (CM-57 le prévoit converti) : le moteur de conversion du socle 7 ne lit que JPEG et PNG ; le HEIC est refusé à l'entrée (conversion côté appli).
5. **403 conservé sur la lecture du fil** (`list_messages_for_actor`, M1) : seuls les pièces, les cartes et les nouvelles routes répondent 404 uniforme. Étendre au fil casserait de nombreux tests M0a à M3 ; à décider.
6. **Un message peut recevoir une pièce de n'importe quel participant du fil** (pas seulement de son auteur) : comportement antérieur conservé, à resserrer.
7. Pas d'accusé « refus » côté destinataire : R-M19 ne prévoit aucun réglage de refus ; interprété comme « une lecture qui n'est pas celle d'un destinataire ne pose rien ». S-145 reste écarté.
8. Libellé de lecture « Prénom X. » et non « Mme B. » (aucune civilité en base).
9. PNG/WebP/GIF : seuls JPEG et PNG sont nettoyés de leurs métadonnées ; GIF et WebP sont servis tels quels après l'antivirus.
10. **Aucun écran Leptos** : carte « document », états de pièce, « lu par », prise en charge et disponibilité sont exposés par l'API seulement.
11. **Sans démon antivirus, aucune pièce ne sort de quarantaine** (fermé par défaut), y compris les pièces historiques reprises.

## Questions au pilote
- Q-M15 (moteur antivirus) : le client `clamd` du socle 7 est réutilisé sans nouvelle dépendance ; le déploiement du démon reste à décider.
- Faut-il étendre le 404 uniforme à la lecture du fil (écart 5) et resserrer l'attachement (écart 6) dans un lot dédié ?
- Veux-tu l'extension « destinataires explicites » du socle 8 avant M7 (écart 2) ?

## Incident de méthode
Une première compilation a été lancée avec `&` (interdit), arrêtée aussitôt et relancée avec `run_in_background` ; aucune autre.
