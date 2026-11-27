# Contre-revue — socle 27, G3a, guichet des droits RGPD (branche ent-socle-27-code-g3a, fa00fed1 + fusion d'ENT par le pilote) — 2026-11-25

Relecteur `review-sonnet`, lecture seule, rien exécuté. **Verdict : après correctifs (2 élevés).** Lot **retenu**.

## Conforme
- Dépôt sans compte : établissement tiré de l'hôte, aucun oracle d'existence (message invariable), qualités bornées, aucun accusé à l'adresse fournie, tailles bornées.
- Qualité à la date (lien actif, non suspendu, fiche non fusionnée ; protection → `Restreinte`) ; élève majeur et mineur distingués.
- Actions `droits.recevoir|traiter|remettre|arbitrer` dans `Action::TOUTES`, fonction `socle_27_rgpd` du modèle par défaut ; `exiger_action_etab` sur chaque service ; 404 neutres et cloisonnement testés.
- 2544 : RLS sur 5 tables, `search_path` et `public.`, fin de tableau, registre et conservation à jour, journal en ajout seul ; compteur atomique sans trou ni doublon.
- Horloge (échéance +1 mois, prolongation unique motivée ≤ +2 mois) ; accusé sans nom d'élève ; frontend sans logique de droit, pièce jamais renvoyée.

## Élevés
- **E1, la pièce d'identité n'est jamais détruite** (`droits_demandes.rs:2413-2456`) : seule la référence est effacée (`piece_document_id = NULL`), l'objet du coffre reste, orphelin ; le journal et l'écran affirment « détruite » (R-RG16, CL27-70). Le test ne vérifie que la colonne.
- **E2, un parent sans autorité parentale ou dont la qualité est restreinte peut déposer comme titulaire** (`exiger_coherence_qualite`, `:2337-2353`, seule `Absente` est refusée) ; la demande reçoit « identité vérifiée » (session forte). Rien ne bloquerait la remise d'un extrait en G3b.

## Moyens
1. Aucun garde-fou entre identité / qualité et décision, clôture (`droits_verifier_identite`, `droits_decider`, `droits_clore`).
2. Qualité d'un demandeur sans compte jamais constatée (`sans_objet` trompeur) ; pas de rapprochement du sujet `inconnu`.
3. `demandeur_compte_id` saisi au guichet sans contrôle d'appartenance à l'établissement (accusé envoyé à un compte d'ailleurs).
4. Dépôt public : anti-abus contournable (contact non normalisé), aucun plafond par établissement ; numéro officiel et délais consommés par le spam.
5. Droit `sans_objet` décidable sans `droits.arbitrer` ni motif (contourne R-RG17 pour un effacement).
6. CL27-04 : information des titulaires du mineur ni forcée ni bloquante.

## Faibles
F1 oracle faible 403/404 sur le dépôt connecté ; F2 accès suspendu ou fermé accepté ; F3 réouverture d'une demande refusée sans nouvel examen ; F4 alertes J-10 / J-3 en égalité stricte ; F5 `now()` dans la logique (à confirmer) ; F6 purge des dépôts spam (G6) ; F7 mode « pièce consultée » inutilisable depuis l'UI, page publique de dépôt absente ; F8 `piece_document_id` jamais validé. Remarque : motif de prolongation en texte libre dans la notification.
