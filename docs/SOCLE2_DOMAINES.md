# Socle 2 — Domaines, actions métier et toile des événements

Date : 2026-11-05. Statut : **conception, non décidée** (à valider par Florian). Aucun code modifié, rien compilé ni testé pour ce document.
Reprend et étend `docs/CARTOGRAPHIE_ENT.md` (16 domaines D0–D15) ; s'inscrit dans le socle 2 de `docs/ROADMAP.md`. Ne remplace aucune décision de `docs/DECISIONS.md`.

**Conventions.** **[F]** fait vérifié dans le dépôt (chemin cité) ; **[H]** hypothèse (mémoire, non vérifiée : à confirmer) ; **[R]** recommandation (non décidée). États dans les tableaux d'actions : **I** = implémenté côté back (route + repo), **P** = partiel, **A** = absent. « Implémenté » ne veut jamais dire « validé pilote » (cf. `PROJECT_STATUS.md`).

Sommaire : §0 Faits de départ · §1 Benchmark et exigences · §2 Carte des domaines · §3 Actions métier · §4 Toile des événements · §5 Au-delà d'un ENT classique · §6 Lots de réalisation · §7 Décisions à soumettre · §8 Faits / hypothèses / questions.

---

## 0. Faits de départ (delta par rapport à la cartographie du 2026-11-04)

Relevés par `ls`/`grep` sur la copie de travail (branche locale `ent-socle1-sso-webauthn`, dernier commit `0beefb3`, travaux socle 1 en cours).

| Sujet | Fait [F] | Preuve |
|---|---|---|
| Migrations | 95 fichiers, `001` → `096` (**`094` absent** de la séquence) ; 126 tables `ent_*` créées | `ent-rust/migrations/` |
| Référentiel | Créés depuis la cartographie : `ent_annee_scolaire` (une seule `courante` par établissement), `ent_classe` (`niveau` en **texte**), `ent_groupe` (rattachable à une classe), `ent_affectation` (élève → classe **ou** groupe, datée, `source` saisie/amorçage/sync), `ent_service_enseignement` (prof × matière × classe/groupe × année, `professeur_principal` unique par classe) | `migrations/092_referentiel_scolaire.sql` |
| Établissement | `ent_etablissement` (slug, nom, actif) + `ent_etablissement_domaine` (D-012) : la dette V10 est en cours de résorption | `migrations/096_etablissement_domaines.sql` |
| Socle 1 en cours | passkey rattachée à la personne (093), sessions web/mobile + niveau d'assurance + `ent_politique_securite` (095) ; `perimetre.rs` expose `prof_suit_eleve(_le)`, `eleves_suivis_par_prof`, `eleve_est_majeur` | `migrations/093,095`, `ent-db/src/perimetre.rs` |
| Appel | unique par `(établissement, créneau, jour)` (index 021, la contrainte classe+jour de 009 est supprimée) | `migrations/021_edt_creneau_appel.sql` |
| Absence | **granularité journée** : `debut DATE`, `fin DATE`, pas d'heure ni de demi-journée ; statut de justification (010), décision (011) | `migrations/004,010,011` |
| Retard | fait distinct, `source_appel_id`, minutes | `migrations/087_retard.sql` |
| Événements | 151 constantes de types, 57 variantes `AggregateKind` ; `MetierEvent` = enveloppe + `payload: serde_json::Value` (non typé) | `ent-events/src/lib.rs` |
| Publication | `PgEventStore::publish` insère dans `ent_metier_event` **sur le pool**, hors de toute transaction métier, puis appelle les handlers **de façon synchrone** dans le même processus ; `service.rs` compte 164 appels `.publish(` pour **1** seul `.begin()` | `ent-db/src/event_store.rs:28-60`, `ent-db/src/service.rs` |
| Abonnés | un seul : le Brain ; `react()` ne réagit qu'à `appel.closed` → `BrainAction::EnsureAbsence` (une seule variante d'action). La cartographie mentionnait aussi `eleve.created` : non retrouvé dans `react.rs` | `ent-brain/src/react.rs`, `actions.rs` |
| Rôles | 10 rôles fixes, 32 fonctions `can_*` ; pas de rôle accueil, surveillant (AED), restauration, documentaliste | `ent-auth/src/roles.rs` |
| Types partagés | `ent-types` : appel, auth, edt, referentiel, vie_scolaire (33 types publics) | `ent-types/src/` |
| Leptos | modules absences, accueil, appel, classes, connexion, edt, messagerie, vie_scolaire | `web/crates/ent-web-app/src/modules/` |
| Taille | `service.rs` 17 196 lignes, `routes.rs` 11 384 lignes | `wc -l` |

Conséquences directes pour le socle 2 :
1. **Aucune garantie de livraison des événements** : un échec entre l'écriture métier et l'insertion de l'événement perd l'événement ; un handler en erreur fait échouer `publish` après le commit (risque déjà signalé pour la clôture d'appel). La toile (§4) commence donc par une outbox transactionnelle.
2. **Absences à la journée** : incompatible avec le décompte réglementaire en demi-journées [H] et avec l'appel par créneau. Le modèle d'absence doit passer à l'heure/créneau (additif).
3. **Le dossier élève n'existe que comme agrégat de lecture** (`domain-eleve/src/dossier.rs`) ; il n'y a pas de chronologie, ni de point d'entrée unique des actions.

---

## 1. Benchmark fonctionnel et exigences réglementaires

### 1.1 Ce que couvrent les concurrents [H — de mémoire, sans recherche web]

| Produit | Positionnement | Couverture connue | Incertitude |
|---|---|---|---|
| **Pronote** (Index Éducation) | Vie scolaire + pédagogie, public et privé, très majoritaire dans le second degré ; couplé au logiciel EDT du même éditeur | Appel par cours (y compris sur mobile), absences/retards/punitions/sanctions, cahier de textes avec travail à rendre en ligne et QCM, notes, compétences, bulletins, LSU, conseils de classe (saisie des appréciations, décisions), orientations, stages, rencontres parents-professeurs (planification), casier numérique, discussions/messagerie, informations et sondages, agenda, réservation de salles et de matériel, absences et remplacements de professeurs, menus, applications mobiles, imports SIECLE/STS, connecteurs ENT | moyenne : modules exacts selon licences |
| **École Directe** (Aplim) | Portail web/mobile familles-élèves-profs de Charlemagne, surtout privé sous contrat | Notes, vie scolaire, cahier de textes, messagerie, documents, **paiement en ligne et factures**, **réinscription en ligne**, réservation cantine, sondages, espace de travail/cloud, QCM | moyenne |
| **Charlemagne** (Aplim) | Logiciel de gestion (back-office) de l'établissement privé | Administratif (inscriptions, dossiers, responsables, foyers), **facturation OGEC**, comptabilité, paie (séparées), vie scolaire, notes, EDT, internat, cantine/badge, stages/CFA, liaisons Éducation nationale. 659 tables observées (`docs/ent-metier-checklist.md`) | faible pour la couverture, [F] pour le catalogue de tables |
| **École Futée** | [H] solution plus récente, orientée établissements privés et/ou premier degré (gestion + portail) | Non connu avec précision : ne pas s'en servir comme référence tant que non vérifié | **forte** |
| Autres (contexte) | ENT régionaux/départementaux du public (Skolengo/Kdecole, Atrium, Monlycée.net…) [H] | Portail, messagerie, ressources (GAR), souvent branchés sur Pronote pour la vie scolaire | moyenne |

Lecture [R] : le marché attend **deux couches** — un logiciel de gestion (Charlemagne, SIECLE côté public) et un portail de vie scolaire/pédagogie (Pronote, École Directe). ScolIA doit couvrir les deux dans un seul modèle : c'est son argument (une source de vérité, zéro double saisie).

### 1.2 Fonctions attendues pour être commercialisable

Priorité : **C** = condition de vente (sans elle, pas de signature), **A** = attendu (dans l'année), **D** = différenciant. État ScolIA = back Rust.

| # | Fonction | Prio | État ScolIA [F] | Domaine |
|---|---|---|---|---|
| 1 | Appel par cours (web + mobile), vérification des appels non faits | C | appel par créneau I ; vérification A | D5 |
| 2 | Absences/retards, justificatifs, motifs, décompte demi-journées, signalement | C | I à la journée ; demi-journées A ; signalement A | D5 |
| 3 | Déclaration d'absence par l'accueil / la famille (avant l'appel) | C | justification I ; déclaration préalable A | D5 |
| 4 | Punitions, sanctions, incidents, procédures disciplinaires | C | incidents/sanctions I ; procédure (conseil de discipline) A | D5 |
| 5 | Cahier de textes (séance + travail à faire), pièces, travail à rendre | C | embryonnaire (1 table, 1 route) | D7 |
| 6 | Notes, devoirs, moyennes, appréciations | C | I | D6 |
| 7 | Compétences, socle, bilans périodiques et de fin de cycle | C (collège) | P (`ent_competence_*`) | D6 |
| 8 | Bulletins PDF, conseils de classe | C | P (`bulletin_pdf.rs`, `ent_conseil_classe`) | D6 |
| 9 | LSU / LSL / Notanet | C | exports CSV P [F] ; format officiel à vérifier [H] | D6, D23 |
| 10 | Emploi du temps : consultation, import, exceptions, remplacements | C | I | D4 |
| 11 | Messagerie, annonces, notifications mail/push/SMS | C | messagerie I ; SMS/push simulés | D8, D0 |
| 12 | Applications mobiles (tous rôles) | C (D-003) | prototypes non compilables | transverse |
| 13 | Inscriptions, réinscriptions en ligne, pièces | C (privé) | réinscription I ; admission A | D3 |
| 14 | Facturation, échéanciers, prélèvements SEPA, relances | C (privé) | I | D10 |
| 15 | Paiement en ligne par carte | A | A | D10 |
| 16 | Restauration : grilles, badge, menus, réservations | A | I sauf réservation | D11 |
| 17 | Documents, documents à signer, GED élève | C | I (local, sans URL pré-signée) | D9 |
| 18 | Import SIECLE/STS-Web, ONDE pour le premier degré | C (public), A (privé) | exports seulement | D23 |
| 19 | Connexion ÉduConnect / GAR (public) | C (public) | A | D23, D15 |
| 20 | Rencontres parents-professeurs (prise de rendez-vous) | A | A | D21 |
| 21 | Sondages, formulaires, autorisations, droit à l'image | A | A | D24 |
| 22 | Internat, sorties de l'établissement | A | I | D12 |
| 23 | Sorties et voyages scolaires | A | A | D20 |
| 24 | Stages, conventions | A | I | D12 |
| 25 | Orientation (vœux, Affelnet, Parcoursup) | A | vœux P, export Parcoursup P | D17 |
| 26 | Examens blancs, convocations, aménagements | A | A | D16 |
| 27 | Santé, infirmerie, PAI ; PAP/PPS | A | santé I ; PAP/PPS P (`ent_eleve_dispositif`) | D13, D18 |
| 28 | Absences et remplacements des personnels | A | P (`ent_absence_rh`) | D19 |
| 29 | Réservation de salles/matériel | A | A | D21 |
| 30 | Tableaux de bord direction | A | A | D14 |
| 31 | Élections (délégués, CA, CVL) | D | A | D25 |
| 32 | Activités, études, garderie, transports | D | A | D22 |

### 1.3 Exigences réglementaires et d'interopérabilité françaises

| Exigence | Contenu (à vérifier avant développement) | Impact |
|---|---|---|
| Obligation d'assiduité | [H] appel à chaque cours ; au-delà de 4 demi-journées d'absence sans motif légitime dans le mois, le chef d'établissement saisit l'autorité académique (Code de l'éducation, L131-8) | absences en demi-journées ; compteur mensuel ; action VS-21 |
| Discipline | [H] échelle des sanctions fixée par le Code de l'éducation (avertissement, blâme, mesure de responsabilisation, exclusions temporaires, exclusion définitive), procédure contradictoire, conseil de discipline, délais d'effacement du dossier | référentiel par défaut + paramétrage (privé) ; effacement planifié |
| LSU (collège) | [H] bilans périodiques et de fin de cycle (socle à 8 composantes), import dans l'application nationale par fichier XML au format ministériel | export XML conforme ; aujourd'hui CSV [F] (`lsun_export.rs`) |
| LSL (lycée), Parcoursup, Affelnet, Notanet/DNB | [H] remontées de notes et appréciations, fiche Avenir, vœux d'affectation post-3e, notes du brevet | exports tracés ; dépôt national hors périmètre |
| SIECLE / STS-Web / ONDE | [H] imports des bases élèves et responsables (SIECLE), des services et personnels (STS-Web), premier degré (ONDE) | domaine D23 ; rapprochement par INE (`ine.rs` existe [F]) |
| ÉduConnect, GAR, SDET | [H] public : identité élèves/responsables via ÉduConnect, ressources numériques via le GAR, conformité au schéma directeur des ENT | condition d'entrée sur le marché public |
| Comptabilité publique | [H] dans un EPLE public, la facturation de la demi-pension et de l'internat relève de l'agent comptable et de son logiciel (Op@le) | facturation ScolIA = cœur pour le privé (OGEC) ; export/interface pour le public |
| SEPA | [H] pré-notification avant prélèvement, mandat propre à chaque créancier (déjà acté D-014) | FIN-10 à FIN-12 |
| RGPD | AIPD (données d'enfants), registre, minimisation, droits d'accès/rectification, durées de conservation par catégorie ; DPO à consulter (D-013, D-014) | action PLT-08, PIL-07 ; conservation par domaine |
| Données de santé | [H] question de l'hébergement certifié (HDS) pour les données infirmerie/PAI | décision Q7 |
| Accessibilité | [H] RGAA pour les services publics numériques | design system Leptos |
| Archives | [H] règles de tri et de conservation des archives des EPLE (public) | politique d'archivage par domaine |

---

## 2. Carte des domaines (26 domaines)

Reprise des 16 domaines de la cartographie (numéros conservés) et **10 domaines ajoutés** (D16–D25). Deux recadrages : D12 ne garde que internat, autorisations de sortie et stages (les sorties/séjours scolaires deviennent D20) ; D13 ne garde que santé et psychologie (l'orientation devient D17).

### 2.1 Le dossier élève comme centre : principe d'architecture [R]

Le dossier élève n'est **ni une table fourre-tout, ni un domaine qui possède tout**. Il est trois choses :
1. **Une clé de rattachement** : toute donnée qui concerne un élève porte `eleve_id` (+ `etablissement_id`), et tout événement le concernant porte `eleve_id` dans l'enveloppe (champ déjà présent dans `MetierEvent` [F]).
2. **Un point d'entrée des actions** : l'écran « Dossier » (web et mobile) liste les actions possibles sur cet élève selon les droits de l'acteur (catalogue d'actions §3, capacités du socle 1, lot B6). Les actions restent exécutées par le domaine propriétaire.
3. **Une chronologie** (projection D0) alimentée par les événements : absences, sanctions, notes publiées, documents, factures, passages infirmerie (titre seulement, sans détail, selon niveau d'identité)… filtrée par les droits du lecteur.

Ainsi « toutes les actions passent par le dossier » côté usage, sans casser la propriété des données par domaine (règle 1 de la cartographie §2.4).

### 2.2 Tableau des domaines

Légende état : **I** implémenté (back), **P** partiel, **E** embryonnaire, **A** absent ; **L** = module Leptos existant.

| N° | Domaine | Responsabilité | Données existantes [F] | À créer [R] | Acteurs | État [F] |
|---|---|---|---|---|---|---|
| D0 | Plateforme transverse | Événements, outbox, consommateurs, notifications, chronologie, fichiers, recherche, journal | `ent_metier_event`, `ent_journal_modif`, `ent_mail/sms/push_outbox`, `ent_push_device` | `ent_event_consommateur`, `ent_event_traitement`, `ent_notification`, `ent_notification_envoi`, `ent_preference_notification`, `ent_regle_notification` | système, admin | E (bus synchrone, publication hors transaction) |
| D1 | Référentiel structure | Établissement, sites, années, périodes, calendrier, niveaux/MEF, classes, groupes, matières, services, salles | `ent_annee_scolaire`, `ent_classe`, `ent_groupe`, `ent_affectation`, `ent_service_enseignement` (092), `ent_matiere`, `ent_salle`, `ent_periode_vacances`, `ent_note_periode`, `ent_filiere`, `ent_etablissement(_domaine)` (096) | `ent_site`, `ent_niveau`, `ent_mef`, `ent_periode` (périodes de l'année, rapatrier `ent_note_periode`), `ent_jour_calendrier` (fermetures, journées banalisées), `ent_grille_horaire` (M1…S4) | admin, direction, administratif | P (V1 092 ; `niveau` texte ; `classe` texte encore dans 9 tables) |
| D2 | Dossier élève, personnes et foyers | Identité élève, personnes responsables (avec ou sans compte), foyers, liens et autorité légale, contacts, photos, infos complémentaires | `ent_eleve`, `ent_eleve_periode`, `ent_foyer`, `ent_eleve_lien` (`user_id NOT NULL`), `ent_eleve_compte`, `ent_eleve_invitation`, `ent_info_comp_*`, `ent_user`, `ent_membership` | `ent_personne` (D-014 : responsable sans compte), `ent_personne_contact`, `ent_lien_responsabilite` (autorité, dates, suspension — B7), `ent_eleve_contact_urgence`, `ent_foyer_membre` | administratif, direction, CPE (lecture), parents, élève | I (lien = compte : dette V8) |
| D3 | Inscriptions, réinscriptions, scolarité | Admission, préinscription, inscription, réinscription, montée, changements, départ, historique | `ent_reinscription_*`, `ent_eleve_scolarite_annee`, `ent_eleve_option_annee`, `ent_inscription_suivi`, `ent_voeu_inscription` | `ent_dossier_admission`, `ent_admission_piece`, `ent_admission_etape`, `ent_admission_rdv`, `ent_depart` (motif, établissement d'accueil, exeat) | administratif, direction, parent | I réinscription ; A admission |
| D4 | Emplois du temps | Créneaux, semaines A/B, exceptions, import, contraintes, lecture par classe/salle/prof | `ent_edt_creneau`, `_exception`, `_calendrier_ab`, `_import`, `_contrainte` | `service_id` sur créneau ; `ent_edt_seance` (occurrence datée matérialisée : support de l'appel, du cahier, du remplacement) | administratif, direction, CPE, prof, famille | I ; L (lecture semaine) |
| D5 | Vie scolaire | Appel, vérification, absences, retards, déclarations, justificatifs, dispenses, incidents, punitions, sanctions, procédures, carnet, signalements, commissions | `ent_appel*`, `ent_absence*`, `ent_retard`, `ent_incident*`, `ent_sanction*`, `ent_carnet_entree`, `ent_vs_commission*`, `ent_dispense_vs`, `ent_alerte_cpe`, `ent_edition_vs`, `ent_absence_signal` | `ent_absence_declaration` (accueil/famille, avant constat), `ent_absence_creneau` (granularité séance), `ent_punition` (distincte de la sanction), `ent_procedure_disciplinaire`, `ent_signalement_assiduite`, `ent_appel_verification` | prof, CPE, AED, accueil, direction, famille, élève | I (le plus complet) ; L (appel, VS, absences) |
| D6 | Évaluations | Devoirs évalués, notes, moyennes, appréciations, compétences, bulletins, conseils de classe, livrets | `ent_devoir`, `ent_note`, `ent_note_periode`, `ent_note_appreciation`, `ent_competence_*`, `ent_conseil_classe*` | `ent_bulletin` (instance figée publiée), `ent_bilan_cycle`, `ent_appreciation_generale`, `ent_mention`, `ent_decision_conseil` | prof, PP, direction, CPE, famille, élève | I ; Leptos A |
| D7 | Cahier de textes et travail | Séances (contenu), travail à faire, travail à rendre, pièces, progression | `ent_cahier_texte` | `ent_seance_contenu`, `ent_travail` (à faire / à rendre), `ent_travail_remise`, `ent_travail_piece` | prof, élève, famille, direction | E |
| D8 | Communication | Messagerie, annonces, modèles, campagnes, accusés, visio (lien) | `ent_msg*`, `ent_annonce`, `ent_msg_modele` | `ent_campagne` (ciblage + confirmation humaine), `ent_annonce_lecture` | tous selon droits | I ; L (messagerie) |
| D9 | Documents et signatures | Pièces du dossier, grants, documents à signer, gabarits, publipostage, conversions (D-005) | `ent_eleve_document`, `ent_document_grant`, `ent_eleve_dispositif`, `ent_doc_a_signer`, `ent_pai_piece` | `ent_gabarit`, `ent_signature` (preuve), `ent_document_version` | administratif, direction, famille, élève | I (local) |
| D10 | Finances | Tarifs, factures, avoirs, échéanciers, encaissements, SEPA, relances, bourses, quotient, paiement en ligne, export comptable | `ent_fac_*`, `ent_taux_tva`, `ent_tarif_conso`, `ent_foyer_iban_histo`, `ent_bank_rapprochement`, `ent_bourse_type`, `ent_eleve_bourse`, `ent_foyer_quotient` | `ent_fac_article`, `ent_fac_formule` (scolarité, DP, internat…), `ent_fac_repartition` (répartition entre foyers payeurs), `ent_fac_avoir`, `ent_fac_paiement_en_ligne`, `ent_fac_remise` (fratrie, personnel) | compta, direction, parent payeur | I ; Leptos A |
| D11 | Restauration | Services, grilles, dispenses, réservations, passages, menus, effectifs cuisine, allergies (lecture) | `ent_repas_*`, `ent_eleve_grille_repas`, `ent_eleve_dispense_repas`, `ent_menu_cantine` | `ent_repas_reservation`, `ent_repas_effectif` (prévision par service) | restauration, CPE, compta, famille | I sauf réservation |
| D12 | Internat, sorties de l'établissement, stages | Chambres, affectations, appel du soir, suivi ; autorisations et contrôles de sortie ; entreprises et conventions | `ent_internat_*`, `ent_sortie_*`, `ent_entreprise`, `ent_convention_stage` | `ent_internat_appel`, `ent_stage_periode`, `ent_stage_evaluation` | CPE, AED/maître d'internat, prof référent, famille | I |
| D13 | Santé et psychologie | Fiche santé, infirmerie, PAI, protocoles, psy | `ent_fiche_sante`, `ent_infirmerie_passage`, `ent_pai`, `ent_psy_*` | `ent_traitement_medical`, `ent_protocole_urgence`, `ent_vaccination` | infirmerie, psy, famille | I |
| D14 | Pilotage | Indicateurs, tableaux de bord, éditions, exports réglementaires tracés | aucune table métier propre ; `ent_edition_vs` | projections `ent_kpi_*` reconstruites depuis les événements | direction, admin | E |
| D15 | Administration | Établissement, domaines, utilisateurs, rôles, capacités, délégations, politique de sécurité, PRA | `ent_param_etab`, `ent_membership`, `ent_etablissement*`, `ent_politique_securite`, `ent_session`, `ent_passkey_*`, `ent_oidc_*` | `ent_delegation` (datée), `ent_role_attribution` (historique, B5) | admin, direction | I auth ; P établissement |
| **D16** | Examens et certifications | Examens blancs, épreuves, convocations, salles, surveillances, aménagements, résultats | — | `ent_examen`, `ent_epreuve`, `ent_convocation`, `ent_surveillance`, `ent_amenagement_examen` | direction, administratif, profs, élèves | A |
| **D17** | Orientation et parcours | Projets, vœux (Affelnet, Parcoursup), fiche Avenir, entretiens, stages d'observation, parcours Avenir | `ent_filiere`, `ent_voeu_inscription` | `ent_projet_orientation`, `ent_voeu_orientation`, `ent_avis_orientation`, `ent_entretien_orientation` | PP, psy-EN, direction, famille, élève | P |
| **D18** | Accompagnement et besoins particuliers | PAP, PPS, PPRE, ULIS, AESH, aménagements pédagogiques, tutorat | `ent_eleve_dispositif` (016) | `ent_dispositif_amenagement`, `ent_aesh_affectation`, `ent_suivi_equipe_educative` | direction, PP, CPE, infirmerie, psy, famille | P |
| **D19** | Personnels, absences et remplacements | Fiche personnel (poste, quotité), absences des personnels, remplacements, heures supplémentaires simples | `ent_absence_rh`, `ent_membership` | `ent_personnel_fiche`, `ent_remplacement`, `ent_service_rh` | direction, administratif, personnel | P |
| **D20** | Sorties et séjours scolaires | Projet, budget, participants, autorisations, encadrement, paiement, absences générées, liste d'urgence | — | `ent_sejour`, `ent_sejour_participant`, `ent_sejour_encadrant`, `ent_sejour_etape`, `ent_sejour_budget` | prof organisateur, direction, compta, famille | A (le Brain prévoit le cas « séjour → absence », `actions.rs`) |
| **D21** | Agenda, rendez-vous et réservations | Calendrier d'établissement, réunions, rencontres parents-profs, prise de RDV, réservation de salles/matériel | `ent_psy_rdv` (spécifique) | `ent_evenement_agenda`, `ent_rencontre`, `ent_creneau_rdv`, `ent_ressource`, `ent_reservation` | tous | A |
| **D22** | Activités, études et transports | Clubs, AS, études surveillées, garderie, ateliers ; transports scolaires | — | `ent_activite`, `ent_activite_inscription`, `ent_activite_seance`, `ent_transport_ligne`, `ent_transport_abonnement` | CPE, animateurs, compta, famille | A |
| **D23** | Interopérabilité | Imports SIECLE/STS/ONDE, exports LSU/LSL/Notanet/Affelnet/Parcoursup, comptabilité, ÉduConnect, GAR, iCal, API partenaires | exports `*_export.rs` (domain-eleve), `ent_edt_import` | `ent_import_lot`, `ent_import_ligne` (rapprochement, rejets), `ent_export_trace` | administratif, direction, admin | P (exports CSV) |
| **D24** | Formulaires, sondages et consentements | Sondages, formulaires, autorisations récurrentes (droit à l'image, sortie, usage numérique), consentements | — | `ent_formulaire`, `ent_formulaire_reponse`, `ent_consentement` (objet, version, décision, date) | direction, CPE, profs, familles | A |
| **D25** | Instances et vie démocratique | Élections (délégués, CA, CVL, parents), composition des instances, convocations, comptes rendus | — | `ent_instance`, `ent_election`, `ent_candidature`, `ent_vote` (secret), `ent_mandat` | direction, CPE, électeurs | A |

Rôles à ajouter [R] (décision Q3) : **Accueil**, **Surveillant/AED** (inclut maître d'internat), **Restauration**, **Documentaliste**, **Psy-EN** (distinct du psy d'établissement, si nécessaire), **Intervenant** (animateur, AESH) ; plus des **délégations datées** (remplaçant, organisateur de séjour, professeur principal) plutôt que de nouveaux rôles.

---

## 3. Actions métier par domaine

**Notation des droits.** Rôles : ADM admin, DIR direction, ADF administratif, CPE, AED surveillant [R], ACC accueil [R], CPT compta, PRF prof, PP professeur principal (via `ent_service_enseignement.professeur_principal` [F]), INF infirmerie, PSY, RST restauration [R], PAR responsable, ELV élève, SYS système (consommateur d'événement). Périmètres : **ETAB** tout l'établissement (toujours déduit de la session, jamais du client) ; **CLS** élèves suivis (`prof_suit_eleve_le` [F]) ; **AUT** auteur ; **LIEN(d)** responsable lié avec le droit `d` (`peut_voir_notes`, `peut_voir_vs`, `peut_justifier_absence` [F] ; autres [R]) ; **SOI** l'élève lui-même ; **MAJ** élève majeur (`eleve_est_majeur` [F]) ; **SANTE** niveau d'identité santé. Toute action écrit au journal ; toute action sensible exige confirmation (et ré-authentification si marquée ★).

Les identifiants sont **stables** : on n'en réutilise jamais un supprimé.

### D0 — Plateforme (PLT)

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| PLT-01 | Publier un événement dans la transaction métier | SYS | écrit dans la **même transaction** que le fait ; enveloppe typée versionnée | ligne `ent_metier_event` + `seq` | — | A (hors transaction [F]) |
| PLT-02 | Distribuer aux consommateurs | SYS | ordre par agrégat ; au moins une fois ; consommateur idempotent | marque `traite` / `echec` | — | A |
| PLT-03 | Rejouer un consommateur depuis un point | ADM ★ | consommateur déclaré rejouable ; jamais les envois externes | projections reconstruites | `plateforme.rejeu_lance` | A |
| PLT-04 | Traiter une file d'échecs (relancer, ignorer avec motif) | ADM | motif obligatoire pour ignorer | état mis à jour | `plateforme.echec_traite` | A |
| PLT-05 | Notifier un destinataire selon règles et préférences | SYS | droits du destinataire revérifiés ; heures calmes ; regroupement | `ent_notification` + envois par canal | `notification.envoyee` / `notification.echec` | P (3 outbox [F]) |
| PLT-06 | Régler ses préférences de notification | tous · SOI | certains avis non désactivables (sécurité, absence non justifiée) | préférences | — | A |
| PLT-07 | Consulter la chronologie d'un élève | tout lecteur autorisé du dossier | filtrée par domaine et niveau d'identité ; santé/psy = titre neutre ou rien | lecture | — | P (`events_for_eleve` [F]) |
| PLT-08 | Produire l'extrait « droit d'accès » RGPD d'une personne | DIR ★ | demande tracée ; délai ; exclut données de tiers | archive privée (URL pré-signée) | `rgpd.extrait_produit` | A |
| PLT-09 | Appliquer la conservation (purge/anonymisation planifiée) | SYS | règles par catégorie validées par le DPO ; jamais sur un litige en cours | suppression/anonymisation tracée | `rgpd.purge_appliquee` | A |
| PLT-10 | Recherche globale (élève, personne, classe, document) | personnel · ETAB | résultats filtrés par droits ; aucune donnée santé indexée | lecture | — | A |
| PLT-11 | Déposer un fichier (contrôle du type réel, taille) | tout acteur d'une action avec pièce | clé jamais fournie par le client [F] | objet stocké | `object.stored` [F] | P |
| PLT-12 | Émettre une URL pré-signée courte | SYS après contrôle des droits | durée courte, une ressource | URL | — | A (en cours, socle 1 D1) |

### D1 — Référentiel structure (REF)

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| REF-01 | Créer/préparer une année scolaire | ADF, DIR · ETAB | dates non chevauchantes ; libellé unique [F] | année « en préparation » | `referentiel.annee_creee` | I (092) |
| REF-02 | Ouvrir l'année (devenir courante) | DIR ★ | une seule courante [F] ; classes et périodes définies | année courante ; l'ancienne passe en lecture seule | `referentiel.annee_ouverte` | P |
| REF-03 | Clôturer l'année | DIR ★ | conseils du dernier trimestre clos ; passages décidés | archivage, lecture seule | `referentiel.annee_cloturee` | A |
| REF-04 | Définir les périodes (trimestres/semestres) par niveau | ADF, DIR | couvrent l'année sans trou ni chevauchement | périodes | `referentiel.periodes_definies` | P (`ent_note_periode`) |
| REF-05 | Gérer le calendrier (vacances, fermetures, journées banalisées) | ADF | import du calendrier de zone possible | jours neutralisés | `etab.vacances` [F], `referentiel.jour_ferme` | P |
| REF-06 | Définir la grille horaire (M1…S4, demi-journées) | ADF | sert au décompte en demi-journées | grille | `referentiel.grille_horaire_definie` | A |
| REF-07 | Gérer niveaux et MEF | ADF | MEF importé (SIECLE) ou saisi | niveaux | `referentiel.niveau_cree` | A (niveau texte [F]) |
| REF-08 | Créer/modifier/archiver une classe | ADF, DIR | code unique par année [F] ; pas de suppression si données | classe | `referentiel.classe_creee` / `_modifiee` / `_archivee` | I/P |
| REF-09 | Créer un groupe (option, LV, demi-groupe) | ADF | rattaché à une ou plusieurs classes | groupe | `referentiel.groupe_cree` | I |
| REF-10 | Affecter un élève à une classe / un groupe (daté) | ADF · ETAB | une seule classe ouverte par année [F] | période ouverte, précédente fermée | `scolarite.classe_changed` [F], `referentiel.affectation_changee` | I |
| REF-11 | Affecter en lot (rentrée, montée) | ADF ★ | prévisualisation ; idempotent | affectations | `referentiel.affectations_lot` | P (montée lot [F]) |
| REF-12 | Créer un service d'enseignement (prof × matière × cible) | ADF, DIR | prof membre ; matière du référentiel | service ; ouvre les droits CLS | `referentiel.service_cree` / `_cloture` | I |
| REF-13 | Désigner le professeur principal | DIR | un PP par classe [F] | droit PP | `referentiel.pp_designe` | I |
| REF-14 | Gérer matières, salles, sites | ADF | codes nationaux si import STS | référentiels | `matiere.created`, `etab.salle` [F] | I |
| REF-15 | Lister les classes / résoudre « texte → classe_id » | tous personnels | sélecteur unique pour tous les modules | lecture | — | I (`/referentiel/*`) |

### D2 — Dossier élève, personnes et foyers (DOS)

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| DOS-01 | Créer un dossier élève | ADF · ETAB | recherche d'homonymes obligatoire [F] ; INE valide [F] | élève « prévu » ou « actif » | `eleve.created` [F] | I |
| DOS-02 | Modifier l'identité | ADF | champs d'état civil historisés | identité | `eleve.identity_changed` [F] | I |
| DOS-03 | Créer une personne responsable (avec ou sans compte) | ADF ; PAR via préinscription | une personne = une identité (D-013) ; sans e-mail possible (D-014) | `ent_personne` | `personne.creee` | A (dette V8) |
| DOS-04 | Créer/modifier un foyer (adresse, membres) | ADF | parents séparés = foyers distincts (D-014) | foyer | `foyer.cree`, `foyer.changed` (réservé [F]) | I |
| DOS-05 | Lier un responsable à un élève (lien, autorité légale, droits) | ADF, DIR ★ | autorité ≠ lien familial ; droits fins explicites | lien daté | `eleve.lien_updated` [F] | I (compte obligatoire) |
| DOS-06 | Suspendre/retirer un droit d'un responsable (décision de justice) | DIR ★ | pièce justificative ; effet immédiat sur sessions et notifications | lien suspendu | `lien.suspendu` | A (B7) |
| DOS-07 | Désigner payeur(s) et répartition | ADF, CPT | somme des parts = 100 % ; foyer débiteur | répartition | `foyer.payeur_defini` | P (invariant payeurs [F] `dossier.rs`) |
| DOS-08 | Gérer contacts d'urgence et personnes autorisées à récupérer l'élève | ADF ; PAR (proposition) | validation par l'établissement | contacts | `eleve.contacts_urgence_changes` | A |
| DOS-09 | Inviter un responsable / l'élève à créer son compte | ADF | invitation à usage unique, rattachement d'identité existante (D-013) | invitation | `eleve.compte_invite` [F] | I |
| DOS-10 | Mettre à jour ses coordonnées (parent) | PAR · SOI | proposé « aussi chez établissement B » (D-013 §2) | coordonnées ; notification établissement | `personne.coordonnees_modifiees` | A |
| DOS-11 | Poser la photo / trombinoscope | ADF | consentement droit à l'image (D24) pour usages hors administratifs | photo | `eleve.photo` [F] | I |
| DOS-12 | Saisir les informations complémentaires paramétrables | ADF | champ défini par l'établissement | valeur | `admin.info_comp_valeur` [F] | I |
| DOS-13 | Consulter le dossier 360° | selon rôle · CLS / LIEN / SOI | minimisation par niveau d'identité [F] | lecture + journal d'accès | — | P |
| DOS-14 | Passer l'élève majeur (droits propres) | SYS à la date anniversaire | règles de visibilité des parents à décider (Q10) | droits recalculés | `eleve.majeur` | A |
| DOS-15 | Fusionner deux dossiers en doublon | DIR ★ | même établissement ; trace complète ; réversible 30 j [R] | dossier unique | `eleve.fusionne` | A |
| DOS-16 | Séparer les fiches d'un foyer (séparation) | ADF ★ | un e-mail par parent (D-014) | deux foyers | `foyer.separe` | A |

### D3 — Inscriptions, réinscriptions, scolarité (INS)

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| INS-01 | Ouvrir une campagne d'admission | DIR | niveaux, capacités, pièces, dates | campagne | `admission.campagne_ouverte` | A |
| INS-02 | Déposer une préinscription (famille) | PAR (vérifié par code/passkey avant affichage, D-014) | réponse identique compte existant ou non | dossier d'admission « déposé » | `admission.deposee` | A |
| INS-03 | Instruire un dossier (pièces, étape, RDV) | ADF | étapes paramétrées ; motif à chaque refus | étape | `admission.etape_changee` | P (`ent_inscription_suivi` [F]) |
| INS-04 | Décider (admis / liste d'attente / refusé) | DIR ★ | motif ; courrier généré | décision | `admission.decidee` | A |
| INS-05 | **Valider l'inscription** | ADF, DIR ★ | dossier complet ; classe ou niveau cible ; payeurs | élève actif (DOS-01 si nouveau), affectation, régime | `inscription.validee` | A |
| INS-06 | Ouvrir une campagne de réinscription | DIR | exigences et acompte [F] | campagne | `reinscription.campagne_ouverte` [F] | I |
| INS-07 | Confirmer la réinscription (famille) | PAR · LIEN | documents requis [F] ; acompte si configuré | confirmation | `eleve.reinscription_confirmee` [F] | I |
| INS-08 | Ne pas se réinscrire / annoncer un départ | PAR · LIEN | motif ; date | départ prévu | `scolarite.depart_annonce` | A |
| INS-09 | Montée de niveau (projection puis officielle) | ADF ★ | officielle seulement après décision du conseil (D6) | scolarité N+1 | `eleve.montee_lot` [F], `scolarite.passage_valide` | P |
| INS-10 | Changer de régime (externe, DP, interne) | ADF · ETAB | date d'effet ; impacts facture/repas/internat | période régime | `eleve.regime_changed` [F] | I |
| INS-11 | Changer d'options / de MEF | ADF | cohérence MEF/options | période | `scolarite.mef_changed` [F], `eleve.option_added` [F] | I |
| INS-12 | Enregistrer un départ (radiation, exeat) | ADF, DIR ★ | date ; établissement d'accueil ; soldes signalés | statut sorti ; affectations fermées | `eleve.sortie` [F] | P |
| INS-13 | Inviter le second responsable à la validation | SYS sur `inscription.validee` | e-mail indiqué par le premier (D-014) | invitation | `eleve.compte_invite` | A |
| INS-14 | Reprendre l'IBAN masqué et signer un nouveau mandat | PAR | seulement si identité réutilisée (D-014) | IBAN copié, mandat à signer | `fac.sepa_mandat` [F] | A |
| INS-15 | Relancer les pièces manquantes | SYS planifié / ADF | pas plus d'une relance tous les N jours | notification | `admission.relance_envoyee` | A |

### D4 — Emplois du temps (EDT)

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| EDT-01 | Importer un EDT (fichier) | ADF ★ | prévisualisation ; rapprochement classes/services | créneaux | `edt.import` [F] | I |
| EDT-02 | Créer/modifier un créneau | ADF | fin > début [F] ; contraintes (prof, salle) | créneau | `edt.creneau_created` [F], `edt.creneau_modifie` | I |
| EDT-03 | Lier un créneau à un service d'enseignement | ADF | service de la même année | droits CLS cohérents | `edt.creneau_modifie` | A |
| EDT-04 | Déclarer une exception (annulation, déplacement, changement de salle) | ADF, CPE, DIR | unique par (créneau, jour) [F] | exception | `edt.exception` [F] | I |
| EDT-05 | Organiser un remplacement | DIR, ADF | remplaçant disponible ; délégation datée (droits CLS temporaires) | exception + délégation | `edt.remplacement_organise` | P |
| EDT-06 | Matérialiser les séances datées | SYS | à partir des créneaux, A/B, calendrier | `ent_edt_seance` | `edt.seances_generees` | A |
| EDT-07 | Consulter l'EDT (prof, élève, classe, salle) + iCal | tous · CLS/LIEN/SOI | exceptions visibles | lecture | — | P (classe/salle A) |
| EDT-08 | Annuler en masse (grève, fermeture, sortie de niveau) | DIR ★ | prévisualisation des classes touchées | exceptions en lot | `edt.annulation_masse` | A |

### D5 — Vie scolaire (VS)

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| VS-01 | **Faire l'appel** d'une séance | PRF · CLS (remplaçant par délégation) ; AED pour permanence | appel par (créneau, jour) [F] ; pré-rempli par déclarations, dispenses, stages, séjours | lignes présent/absent/retard | `appel.opened` [F] | I ; L |
| VS-02 | Clôturer l'appel | PRF | verrouillage après clôture [F] ; modifiable par la VS seulement | absences et retards créés **dans la même transaction** [R] | `appel.closed` [F] → `eleve.absence_declared` (source = appel), `vs.retard` [F] | I (effets après commit [F]) |
| VS-03 | Consulter un appel sans l'ouvrir | PRF, CPE | lecture seule (GET) | — | — | A (aujourd'hui `POST` [F]) |
| VS-04 | **Vérifier les appels** (non faits, incohérences) | CPE, AED · ETAB | tableau des séances sans appel à H+15 ; élève absent puis présent | relance au prof ; correction | `appel.non_fait_detecte`, `appel.corrige` | A |
| VS-05 | Corriger un appel clos | CPE ★ | motif ; historique | lignes corrigées | `appel.corrige` | A |
| VS-06 | **Déclarer une absence à l'accueil** (appel/visite d'un parent) | ACC, CPE, AED · ETAB | identité de l'appelant notée ; période (séances/demi-journées) | déclaration « prévenue » visible à l'appel | `vs.absence_declaree` (source = accueil) | A |
| VS-07 | Déclarer une absence (famille, en ligne) | PAR · LIEN(peut_justifier_absence) | avant ou après ; pièce jointe | déclaration | `vs.absence_declaree` (source = famille) | P (justification après coup [F]) |
| VS-08 | Rapprocher déclaration et constat | SYS sur `appel.closed` | même élève, séance couverte | absence marquée « prévenue » | `eleve.absence_justif_soumis` [F] | A |
| VS-09 | Justifier une absence (famille) | PAR · LIEN(peut_justifier_absence) ; ELV · MAJ | B3 : élève mineur exclu | justificatif soumis | `eleve.absence_justif_soumis` [F] | I |
| VS-10 | Accepter / refuser un justificatif | CPE · ETAB | motif réglementaire ; refus motivé | statut | `eleve.absence_justified` / `_refused` [F] | I ; L |
| VS-11 | Remettre présent (erreur d'appel) | CPE | historique conservé | absence annulée | `eleve.absence_annulee_present` [F] | I |
| VS-12 | Enregistrer un retard (hors appel, à la vie scolaire) | AED, CPE, ACC | billet d'entrée ; minutes | retard | `vs.retard` [F] | I |
| VS-13 | Poser une dispense (EPS, piscine, médicale) | CPE, INF · ETAB | certificat ; dates | dispense, prise en compte à l'appel | `vs.dispense` [F] | I |
| VS-14 | Déclarer un incident | PRF · CLS ; CPE, AED, DIR | faits datés, élèves impliqués, témoins | incident | `eleve.incident_declared` [F] | I |
| VS-15 | Donner une punition scolaire (retenue, devoir supplémentaire) | PRF · CLS, CPE, AED | distincte de la sanction [H] | punition ; retenue planifiée | `vs.punition_donnee` | A (confondue avec sanction) |
| VS-16 | Prononcer une sanction | DIR (CPE selon délégation) ★ | échelle réglementaire [H] ; procédure contradictoire | sanction ; points | `eleve.sanction_prononcee` [F] | I |
| VS-17 | Engager une procédure disciplinaire / conseil de discipline | DIR ★ | délais, convocations, droits de la défense [H] | procédure, convocations (D9) | `vs.procedure_engagee`, `vs.conseil_discipline_decide` | A |
| VS-18 | Annuler/effacer une sanction (recours, délai d'effacement) | DIR ★ / SYS planifié | délais réglementaires [H] | sanction effacée du dossier administratif | `eleve.sanction_annulee` [F], `vs.sanction_effacee` | P |
| VS-19 | Écrire au carnet de liaison / faire signer | PRF, CPE · CLS ; PAR signe | accusé de signature | entrée, signature | `eleve.carnet_entree`, `eleve.carnet_signee` [F] | I |
| VS-20 | Ouvrir/suivre un signal d'absentéisme | SYS sur seuil ; CPE | seuil paramétrable (défaut 10 [F]) ; une alerte famille par signal | signal | `eleve.absence_seuil_atteint`, `_signal_en_cours`, `_signal_clos` [F] | I |
| VS-21 | Préparer le signalement d'assiduité à l'autorité académique | CPE propose, DIR valide ★ | décompte en demi-journées non justifiées sur le mois [H] | dossier de signalement (PDF) | `vs.signalement_assiduite_emis` | A |
| VS-22 | Réunir une commission éducative / de suivi | CPE, DIR | convocation, décision | commission, dossiers | `vs.commission*` [F] | I |
| VS-23 | Émettre une alerte CPE | CPE | visibilité famille paramétrable [F] | alerte | `vs.alerte_cpe` [F] | I |
| VS-24 | Éditer un document VS (courrier, relevé d'absences) | CPE | modèles à placeholders [F] | PDF | `vs.edition` [F] | I |
| VS-25 | Tableau du jour (absents, retards, à traiter) | CPE, AED, ACC | liste du jour ; filtres | lecture | — | I ; L |

### D6 — Évaluations (EVA)

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| EVA-01 | Créer une évaluation (devoir noté) | PRF · service | période ouverte [F] ; coefficient, barème | devoir | `devoir.created` [F] | I |
| EVA-02 | Saisir les notes en grille | PRF · service | absent/dispensé/non noté distincts ; période ouverte | notes | `note.saisie` [F] | P (pas de grille) |
| EVA-03 | **Publier** les notes d'une évaluation aux familles | PRF | publication explicite ou différée [R] | visibles par LIEN(peut_voir_notes), SOI | `evaluation.publiee` | A |
| EVA-04 | Évaluer des compétences | PRF · service | référentiel socle [F] | niveaux de maîtrise | `note.competence` [F] | I |
| EVA-05 | Saisir les appréciations par matière | PRF · service | période `ouverte` ou `partielle_appreciations` [F] | appréciation | `note.appreciation` [F] | I |
| EVA-06 | Saisir l'appréciation générale | PP | toutes matières renseignées (alerte sinon) | appréciation | `evaluation.appreciation_generale` | A |
| EVA-07 | Changer le statut d'une période | DIR | ouverte → partielle → fermée [F] | verrouillage | `note.periode_statut` [F] | I |
| EVA-08 | Préparer le conseil de classe | PP, DIR | tableau automatique : moyennes, absences, sanctions, appréciations manquantes | dossier de conseil | `notes.conseil_classe` [F] | P |
| EVA-09 | Tenir le conseil : mentions, décisions, avis d'orientation | DIR (président), PP | par élève | décisions | `notes.conseil_dossier` [F], `conseil.decision_prise` | P |
| EVA-10 | Générer et publier le bulletin | DIR ★ | période fermée ; bulletin figé (instance immuable) | PDF publié | `note.bulletin_publie` [F] | P |
| EVA-11 | Bilan de fin de cycle (socle 8 composantes) | PP, DIR | fin de cycle 3/4 | bilan | `evaluation.bilan_cycle_valide` | A |
| EVA-12 | Exporter LSU / LSL / Notanet | ADF, DIR ★ | format officiel [H] ; trace d'export [F] | fichier | `export.realise` | P (CSV) |
| EVA-13 | Consulter notes, moyennes, bulletins | PAR · LIEN(peut_voir_notes) ; ELV · SOI | uniquement publiées | lecture | — | I |
| EVA-14 | Corriger une note après fermeture | DIR ★ | motif ; bulletin régénéré en nouvelle version | note | `note.corrigee` | A |

### D7 — Cahier de textes et travail (CDT)

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| CDT-01 | Renseigner le contenu d'une séance | PRF · service | rattaché à une séance datée (EDT-06) | contenu | `cahier.seance_renseignee` | P (`cahier.texte` [F]) |
| CDT-02 | Donner un travail à faire | PRF · service | date `pour_le` ≥ séance | travail visible | `cahier.travail_donne` | P |
| CDT-03 | Modifier/supprimer | PRF · AUT ; DIR | historique ; notification si déjà vu | — | `cahier.travail_modifie` | A |
| CDT-04 | Joindre des pièces / ressources | PRF | fichiers privés, URL pré-signées | pièces | `cahier.piece_jointe` | A |
| CDT-05 | Demander un travail à rendre en ligne | PRF | date limite ; format | travail « à rendre » | `cahier.travail_a_rendre` | A |
| CDT-06 | Rendre un travail | ELV · SOI | avant/après échéance (retard marqué) | remise | `cahier.travail_rendu` | A |
| CDT-07 | Corriger / rendre la copie | PRF | lien possible vers une note (EVA-02) | retour | `cahier.copie_rendue` | A |
| CDT-08 | Voir l'agenda « à faire » | ELV · SOI ; PAR · LIEN | inclut évaluations annoncées | lecture | — | P |
| CDT-09 | Marquer un travail comme fait (élève) | ELV | indicateur personnel | état | — | A |
| CDT-10 | Viser le cahier de textes (inspection, direction) | DIR | lecture par classe/prof | visa | `cahier.vise` | A |

### D8 — Communication (COM)

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| COM-01 | Envoyer un message (fil) | tous selon matrice de destinataires | parent → personnel autorisé ; prof → familles de CLS (B2) ; confirmation | fil, message | `msg.fil_ouvert`, `msg.envoye` [F] | I ; L |
| COM-02 | Répondre, joindre, archiver, classer | participants | pièces privées [F] | — | `msg.envoye`, `msg.piece`, `msg.fil_archive` [F] | I ; L |
| COM-03 | Accuser lecture | participant | horodaté | accusé | `msg.accuse` [F] | I |
| COM-04 | Publier une annonce ciblée | DIR, CPE, ADF | audience (tous, parents, classe, niveau, groupe) ; confirmation | annonce | `annonce.publiee` [F] | I |
| COM-05 | Lancer une campagne (mail/SMS/push en masse) | DIR ★ | prévisualisation des destinataires ; double confirmation ; coût SMS affiché | envois | `campagne.lancee` | P (`/mails`, `/sms` [F]) |
| COM-06 | Gérer des modèles | personnel | variables documentées | modèle | `msg.modele` [F] | I |
| COM-07 | Message « à propos d'un élève » visible dans son dossier | personnel · CLS/ETAB | rattaché à `eleve_id` | apparaît dans la chronologie | `msg.envoye` (eleve_id) | P |
| COM-08 | Bloquer/modérer un fil (abus) | DIR | motif | fil fermé | `msg.fil_modere` | A |
| COM-09 | Lien de visio pour un cours ou un RDV | PRF | service externe à décider (non par défaut) | lien | `visio.planifiee` | A |

### D9 — Documents et signatures (DOC)

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| DOC-01 | Déposer une pièce au dossier | ADF ; PAR (pièces demandées) | type réel vérifié ; catégorie | document | `eleve.document_uploaded` [F] | I |
| DOC-02 | Demander / décider l'accès à un document sensible | demandeur ; DIR | grant daté [F] | accès | `eleve.document_grant_*` [F] | I |
| DOC-03 | Publier un document à signer à une cible | ADF, DIR, CPE | cible = classe, niveau, liste ; date limite | demandes de signature | `doc.a_signer` [F] | I |
| DOC-04 | Signer (famille, élève majeur) | PAR · LIEN ; ELV · MAJ | preuve (horodatage, identité, empreinte du document) | signature | `doc.signe` [F] | I (accusé, pas de valeur probante) |
| DOC-05 | Suivre et relancer les non-signataires | émetteur | relance limitée | notifications | `doc.relance` | A |
| DOC-06 | Générer un document par gabarit / publipostage | ADF, CPE | gabarit versionné | PDF par élève | `doc.genere` | P (`edition_vs`, PDF dédiés [F]) |
| DOC-07 | Convertir un fichier (moteur maison D-005) | SYS | processus isolé, limites taille/temps | fichier converti | `doc.converti` | A |
| DOC-08 | Télécharger (URL pré-signée) | ayant droit | journal de téléchargement [F] | — | — | P |

### D10 — Finances (FIN)

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| FIN-01 | Définir articles, tarifs et formules (scolarité, DP, internat, études, séjours) | CPT, DIR | par année ; TVA [F] ; quotient [F] | tarifs | `fac.tarif_defini` | P (`ent_tarif_conso` [F]) |
| FIN-02 | Générer les factures annuelles d'une campagne | CPT ★ | élèves inscrits ; formules ; répartition payeurs | factures brouillon | `fac.facture_creee` [F] | A (unitaire seulement) |
| FIN-03 | Émettre une facture | CPT ★ | immuable après émission [F] | facture émise | `fac.facture_emise` [F] | I |
| FIN-04 | Créer un avoir | CPT ★ | référence à la facture | avoir | `fac.avoir_emis` | A |
| FIN-05 | Poser un échéancier | CPT | parts = total [F] | échéances | `fac.echeancier_pose` [F] | I |
| FIN-06 | Facturer une consommation (repas, activité, séjour) | SYS sur événement source | idempotent par (source, période) | ligne de facture | `fac.ligne_ajoutee` | A |
| FIN-07 | Appliquer une bourse / une remise (fratrie, personnel) | CPT | bourse décidée avant réduction [F] | réduction d'échéance | `eleve.bourse_decidee` [F], `fac.remise_appliquee` | P |
| FIN-08 | Encaisser (chèque, virement, espèces) | CPT | imputation sur échéances [F] | encaissement | `fac.encaissement` [F] | I |
| FIN-09 | Payer en ligne (carte) | PAR payeur | prestataire à décider (Q5) | paiement | `fac.paiement_en_ligne` | A |
| FIN-10 | Enregistrer un mandat SEPA | PAR payeur ; CPT | un mandat par créancier (D-014) | mandat | `fac.sepa_mandat` [F] | I |
| FIN-11 | Préparer / exporter les prélèvements | CPT ★ | pré-notification [H] | fichier SEPA | `fac.sepa_prelevement`, `fac.sepa_export` [F] | I |
| FIN-12 | Traiter un rejet | CPT | motif [F] ; frais éventuels | échéance rouverte | `fac.sepa_rejet` [F] | I |
| FIN-13 | Relancer un impayé (paliers) | SYS planifié / CPT | jamais avant échéance ; paliers paramétrés | relance | `fac.rappel_envoye` [F] | I |
| FIN-14 | Émettre une quittance / attestation fiscale | CPT ; PAR (téléchargement) | facture soldée | PDF | `fac.quittance_emise` [F] | I |
| FIN-15 | Rapprocher le relevé bancaire | CPT | connecteur simulé [F] | rapprochement | `bank.rapprochement` [F] | P |
| FIN-16 | Exporter vers la comptabilité (OGEC, interface publique) | CPT ★ | période close | fichier | `fac.export_comptable` | P (OGEC [F]) |
| FIN-17 | Consulter son compte famille (solde, échéances) | PAR payeur | seulement ses foyers payeurs | lecture | — | P |

### D11 — Restauration (RES)

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| RES-01 | Définir les services de repas | ADF, RST | jours, horaires | activité | `repas.activite_created` [F] | I |
| RES-02 | Poser la grille d'un élève (forfait) | ADF ; SYS sur changement de régime | régime DP par défaut [F] | grille | `eleve.grille_repas_posed` [F] | I |
| RES-03 | Réserver / annuler un repas (ticket) | PAR · LIEN ; ELV | délai de prévenance | réservation | `repas.reserve` / `repas.reservation_annulee` | A |
| RES-04 | Poser une dispense de repas | ADF ; SYS (séjour, stage, absence longue) | datée | dispense | `eleve.dispense_repas_posed` [F] | I |
| RES-05 | Scanner un passage | RST, AED | unique par élève/service/jour [F] | passage | `eleve.repas_passage` [F] | I |
| RES-06 | Publier les menus | RST | allergènes affichés | menu | `repas.menu` [F] | I |
| RES-07 | Prévoir les effectifs cuisine | RST | grilles + réservations − absences déclarées − sorties | prévision | `repas.effectif_prevu` | A |
| RES-08 | Afficher les alertes allergies au passage | RST · SANTE limité | uniquement « allergie/PAI : voir protocole » | lecture minimale | — | A |
| RES-09 | Valider le volume facturable du mois | CPT, RST | grille − dispenses [F] | volume figé | `repas.volume_valide` | P (calcul [F]) |

### D12 — Internat, sorties de l'établissement, stages (INT)

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| INT-01 | Gérer chambres et affecter | CPE | capacité [F] | affectation datée | `internat.chambre`, `internat.affectation` [F] | I |
| INT-02 | Faire l'appel du soir / pointage | AED | par soirée | présences | `internat.presence` [F] | I |
| INT-03 | Déclarer une absence d'internat (retour famille) | PAR · LIEN ; CPE | avant la soirée | absence internat, repas annulés | `internat.absence_declaree` | A |
| INT-04 | Suivi d'internat (observations) | AED, CPE | visibilité famille paramétrée | suivi | `internat.suivi` [F] | I |
| INT-05 | Poser l'autorisation de sortie (permanente) | PAR · LIEN ; CPE | règles niveau/régime/EDT [F] | autorisation | `sortie.autorisation` [F] | I |
| INT-06 | Demander une sortie exceptionnelle / décider | PAR ; CPE | décision VS [F] | décision | `sortie.demande`, `sortie.decision` [F] | I |
| INT-07 | Contrôler une sortie (badge) | AED, ACC | trace [F] | contrôle | `sortie.controle` [F] | I |
| INT-08 | Enregistrer entreprise et convention de stage | PRF référent, ADF | signatures : établissement, entreprise, famille | convention | `stage.entreprise`, `stage.convention` [F] | I |
| INT-09 | Ouvrir la période de stage | SYS sur convention signée | élève attendu absent des cours (appel pré-rempli) | période | `stage.periode_ouverte` | A |
| INT-10 | Évaluer le stage / attestation | PRF référent, tuteur (sans compte : lien à usage unique [R]) | | évaluation | `stage.evalue` | A |

### D13 — Santé et psychologie (SAN)

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| SAN-01 | Saisir/valider la fiche santé | PAR · LIEN ; INF valide | niveau SANTE ; consultations journalisées [F] | fiche | `infirmerie.fiche_sante` [F] | I |
| SAN-02 | Enregistrer un passage à l'infirmerie | INF | motif, soins, issue (retour en classe, famille appelée) | passage | `infirmerie.passage` [F] | I |
| SAN-03 | Informer la famille d'un passage | INF (selon règle) | aucun détail médical dans la notification | notification | `infirmerie.famille_informee` | I (S2-11) |
| SAN-04 | Renvoyer l'élève au domicile | INF, CPE ★ | responsable joint ; personne autorisée (DOS-08) | sortie tracée, absence créée | `infirmerie.renvoi_domicile` | P (S2-11 : renvoi consigné, absence non créée) |
| SAN-05 | Établir / signer un PAI | INF, médecin ; PAR signe | pièces [F] | PAI | `infirmerie.pai`, `infirmerie.pai_signe` [F] | I |
| SAN-06 | Publier une contrainte non médicale dérivée (« PAI alimentaire », « trousse d'urgence ») | INF | seule information partagée avec RST, séjours, profs | indicateur minimal | `sante.contrainte_publiee` | I (S2-11) |
| SAN-07 | Consigner un entretien psy / RDV / dossier | PSY | dossier psy visible du seul psy (+ DIR habilité, B8) | dossier | `psy.*` [F] | I |
| SAN-08 | Registre des traitements / protocoles d'urgence | INF | | registre | `sante.protocole_mis_a_jour` | I (S2-11) |

### D14 — Pilotage (PIL)

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| PIL-01 | Tableau de bord effectifs (classe, niveau, régime) | DIR | projection depuis affectations | lecture | — | A |
| PIL-02 | Absentéisme par période, classe, motif | DIR, CPE | définition documentée de chaque indicateur | lecture | — | A |
| PIL-03 | Impayés et encaissements | DIR, CPT | | lecture | — | A |
| PIL-04 | Appels non faits, cahiers non remplis | DIR | sans classement nominatif public des profs [R] | lecture | — | A |
| PIL-05 | Résultats (moyennes, compétences) | DIR | | lecture | — | A |
| PIL-06 | Consulter le journal d'audit | DIR, ADM | ajout seul [F] | lecture | — | I |
| PIL-07 | Journal d'accès à un dossier (qui a consulté) | DIR ; PAR pour son enfant [R] | sans exposer les personnels nominativement à la famille (Q9) | lecture | — | A |
| PIL-08 | Exports réglementaires tracés | ADF, DIR | | fichier | `export.realise` | P |

### D15 — Administration (ADM)

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| ADM-01 | Configurer l'établissement (nom, domaines, sites, logo) | ADM ★ | domaine unique (D-012) | établissement | `etab.param` [F] | P (096) |
| ADM-02 | Provisionner un personnel (compte + rôle) | ADM, DIR | identité unique (D-013) | membership | `compte.provisionne` | I |
| ADM-03 | Changer / retirer un rôle | ADM, DIR ★ | hiérarchie ; révocation des sessions (B5) | rôle historisé | `compte.role_change` | A |
| ADM-04 | Déléguer un droit daté (remplaçant, organisateur, PP intérim) | DIR | date de fin obligatoire | délégation | `delegation.accordee` / `_expiree` | A |
| ADM-05 | Désactiver un compte (départ) | ADM ★ | sessions révoquées | compte inactif | `compte.desactive` | P |
| ADM-06 | Politique de sécurité (passkey obligatoire, vagues) | ADM ★ | D-009 | politique | `securite.politique_changee` | P (095) |
| ADM-07 | Configurer le SSO | ADM ★ | découverte validée [F] | fournisseur | `oidc.fournisseur_configure` | I |
| ADM-08 | Paramètres métier (seuils, motifs, délais) | ADM, DIR | valeurs par défaut documentées | paramètres | `etab.param` [F] | I |

### D16 — Examens et certifications (EXA) — nouveau

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| EXA-01 | Créer une session d'examen (brevet/bac blanc, épreuves) | DIR, ADF | niveaux concernés ; dates hors vacances | session, épreuves | `examen.session_creee` | A |
| EXA-02 | Placer les candidats en salles | ADF | capacité ; aménagements (tiers-temps, salle à part) issus de D18 | placements | `examen.placement_fait` | A |
| EXA-03 | Planifier les surveillances | ADF | disponibilité EDT ; équité | surveillances ; exceptions EDT | `examen.surveillances_planifiees` | A |
| EXA-04 | Publier les convocations | ADF ★ | | convocations (D9) | `examen.convocations_publiees` | A |
| EXA-05 | Neutraliser les cours (EDT) pendant l'examen | SYS sur `examen.session_creee` | | exceptions EDT | `edt.exception` | A |
| EXA-06 | Saisir les résultats / les transformer en évaluation | PRF | lien vers EVA | notes | `note.saisie` | A |
| EXA-07 | Enregistrer les résultats officiels (DNB, bac) | ADF | import si disponible | résultats au dossier | `examen.resultat_officiel` | A |

### D17 — Orientation et parcours (ORI) — nouveau

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| ORI-01 | Saisir intentions puis vœux provisoires/définitifs | PAR · LIEN ; ELV | calendrier par niveau (3e, 2de, terminale) | vœux | `orientation.voeux_saisis` | P (`ins.voeu` [F]) |
| ORI-02 | Avis du conseil de classe sur les vœux | PP, DIR | lié à EVA-09 | avis | `orientation.avis_donne` | A |
| ORI-03 | Réponse de la famille (accord, désaccord, recours) | PAR | délais | réponse | `orientation.reponse_famille` | A |
| ORI-04 | Fiche Avenir / exports Affelnet-Parcoursup | PP, DIR | format officiel [H] | export | `export.realise` | P (Parcoursup CSV [F]) |
| ORI-05 | Planifier un entretien d'orientation | PP, PSY | D21 | RDV | `agenda.rdv_pris` | A |
| ORI-06 | Stage d'observation (3e, 2de) | PRF | lié à INT-08 | convention | `stage.convention` | P |

### D18 — Accompagnement et besoins particuliers (ACC) — nouveau

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| ACC-01 | Ouvrir un dispositif (PAP, PPS, PPRE, ULIS) | DIR, PP ; INF/médecin pour PAP | pièces ; dates de révision | dispositif | `eleve.dispositif_ajoute` [F] | I (S2-11) |
| ACC-02 | Définir les aménagements pédagogiques et d'examen | PP, DIR | liste normalisée (tiers-temps, police adaptée…) | aménagements visibles des profs de CLS | `accompagnement.amenagements_definis` | I (S2-11) |
| ACC-03 | Affecter un AESH | DIR | | affectation | `accompagnement.aesh_affecte` | I (S2-11) |
| ACC-04 | Réunir l'équipe éducative / de suivi | DIR, CPE | convocations, compte rendu | réunion | `accompagnement.equipe_reunie` | P (S2-11 : convocations non envoyées) |
| ACC-05 | Réviser / clore un dispositif | DIR | échéance | statut | `accompagnement.dispositif_revise` | I (S2-11) |

### D19 — Personnels, absences et remplacements (RH) — nouveau

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| RH-01 | Tenir la fiche personnel (poste, quotité, disciplines) | ADF, DIR | données RH minimales (pas de paie) | fiche | `rh.fiche_modifiee` | P |
| RH-02 | Déclarer une absence de personnel | personnel · SOI ; ADF | | absence RH | `rh.absence` [F] | I |
| RH-03 | Valider l'absence | DIR | | absence validée | `rh.absence_validee` [F] | I |
| RH-04 | Organiser les remplacements de courte durée | DIR, ADF | séances touchées listées automatiquement | exceptions EDT, délégation | `edt.remplacement_organise` | A |
| RH-05 | Informer les familles des cours annulés | SYS | familles des classes touchées, sans motif | notification | `notification.envoyee` | A |

### D20 — Sorties et séjours scolaires (SEJ) — nouveau

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| SEJ-01 | Créer un projet de sortie/séjour | PRF organisateur | objectifs, dates, classes, budget prévisionnel | projet « brouillon » | `sejour.cree` | A |
| SEJ-02 | Faire valider le projet (direction, CA si nécessaire) | DIR ★ | [H] vote du CA pour les voyages avec participation financière (public) | projet validé | `sejour.valide` | A |
| SEJ-03 | Inscrire les participants | organisateur | effectif ; élèves de CLS ou liste | participants | `sejour.participant_inscrit` | A |
| SEJ-04 | Collecter autorisations et documents (pièce d'identité, AST, fiche sanitaire) | PAR · LIEN | via D9 / D24 ; date limite | autorisations | `doc.signe` | A |
| SEJ-05 | Facturer la participation (échéancier, aides) | CPT | après validation ; aides (fonds social) | factures | `fac.facture_creee` | A |
| SEJ-06 | Désigner les encadrants | DIR | taux d'encadrement [H] | encadrants ; exceptions EDT des encadrants | `sejour.encadrants_designes` | A |
| SEJ-07 | Démarrer le séjour | organisateur | autorisations complètes | absences « séjour » des participants, dispenses repas, cours des encadrants annulés | `sejour.demarre` | A |
| SEJ-08 | Fiche d'urgence hors ligne (mobile) | encadrant | contacts, PAI minimal (SAN-06) ; données effacées après retour | accès temporaire | — | A |
| SEJ-09 | Clôturer (bilan, solde) | organisateur, CPT | | séjour clos | `sejour.cloture` | A |
| SEJ-10 | Annuler / désister un participant | organisateur ; PAR | règles de remboursement | avoir | `sejour.participant_desiste` | A |

### D21 — Agenda, rendez-vous et réservations (AGD) — nouveau

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| AGD-01 | Publier un événement d'établissement (réunion, portes ouvertes) | DIR, CPE | audience | événement | `agenda.evenement_publie` | A |
| AGD-02 | Organiser les rencontres parents-profs | DIR | profs de la classe (services), plages | créneaux proposés | `agenda.rencontre_ouverte` | A |
| AGD-03 | Prendre un rendez-vous | PAR · LIEN | un RDV par prof/enfant ; liste d'attente | RDV | `agenda.rdv_pris` | A |
| AGD-04 | Réserver une salle / du matériel | personnel | sans conflit avec l'EDT | réservation | `reservation.creee` | A |
| AGD-05 | Exporter son agenda (iCal) | tous | | flux | — | P (EDT `.ics` [F]) |

### D22 — Activités, études et transports (PER) — nouveau

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| PER-01 | Créer une activité (club, AS, étude, garderie) | CPE, DIR | places, tarif éventuel | activité | `activite.creee` | A |
| PER-02 | Inscrire un élève | PAR · LIEN ; CPE | places disponibles ; autorisation | inscription | `activite.inscription` | A |
| PER-03 | Faire l'appel d'une séance d'activité | animateur | même composant que VS-01 | présences | `activite.appel_cloture` | A |
| PER-04 | Facturer l'activité | SYS | FIN-06 | ligne | `fac.ligne_ajoutee` | A |
| PER-05 | Gérer lignes et abonnés de transport | ADF | [H] souvent géré par la région : import plutôt que saisie | abonnements | `transport.abonnement` | A |

### D23 — Interopérabilité (IOP) — nouveau

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| IOP-01 | Importer SIECLE (élèves, responsables, structures) | ADF ★ | prévisualisation ; rapprochement par INE puis identité ; jamais d'écrasement silencieux | créations/mises à jour via les commandes des domaines (DOS, REF) | `import.lot_applique` + événements métier | A |
| IOP-02 | Importer STS-Web (personnels, services) | ADF ★ | | services | `import.lot_applique` | A |
| IOP-03 | Importer ONDE (premier degré) | ADF ★ | si premier degré dans le périmètre (Q8) | | `import.lot_applique` | A |
| IOP-04 | Traiter les rejets d'un import | ADF | ligne par ligne | corrections | `import.rejet_traite` | A |
| IOP-05 | Exporter (LSU, LSL, Notanet, Affelnet, Parcoursup, ONDE) | ADF, DIR ★ | trace | fichier | `export.realise` | P |
| IOP-06 | Connexion ÉduConnect / FranceConnect | utilisateur | fédération (comme `ent-oidc`) ; aucun provisionnement implicite | session | `auth.connexion_federee` | A |
| IOP-07 | Accès aux ressources (GAR) | ELV, PRF | affectations transmises au GAR [H] | — | — | A |

### D24 — Formulaires, sondages et consentements (FRM) — nouveau

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| FRM-01 | Créer un formulaire / sondage ciblé | DIR, CPE, PRF (ses classes) | anonyme ou nominatif annoncé | formulaire | `formulaire.publie` | A |
| FRM-02 | Répondre | cible | une réponse par responsable ou par foyer (paramètre) | réponse | `formulaire.repondu` | A |
| FRM-03 | Recueillir un consentement versionné (droit à l'image, usage numérique, sortie) | PAR · LIEN ; ELV · MAJ | objet, version, retrait possible | consentement | `consentement.donne` / `_retire` | A |
| FRM-04 | Consulter qui consent (avant publication d'une photo, d'un séjour) | personnel | lecture du seul objet concerné | lecture | — | A |
| FRM-05 | Résultats et export | auteur | anonymat respecté | synthèse | — | A |

### D25 — Instances et vie démocratique (INSTA) — nouveau

| ID | Action | Acteur · périmètre | Préconditions / règles clés | Effets | Événements | État |
|---|---|---|---|---|---|---|
| INSTA-01 | Organiser une élection (délégués, CA, CVL, parents) | DIR, CPE | corps électoral calculé (classes, responsables) | élection | `election.ouverte` | A |
| INSTA-02 | Déposer une candidature | électeur éligible | | candidature | `election.candidature` | A |
| INSTA-03 | Voter (secret) | électeur | un vote ; séparation identité/bulletin | vote | `election.vote_enregistre` (sans contenu) | A |
| INSTA-04 | Proclamer les résultats, ouvrir les mandats | DIR | | mandats | `election.resultats`, `instance.mandat_ouvert` | A |
| INSTA-05 | Convoquer une instance et publier le compte rendu | DIR | membres = mandats actifs | convocation, PV | `instance.convoquee` | A |

**Bilan** : 26 domaines, 252 actions identifiées : 81 implémentées (I), 45 partielles (P), 126 absentes (A) côté back, d'après les routes et tables citées (état non exécuté).

---

## 4. La toile des événements

### 4.1 Règles de la toile [R]

1. **Intra-domaine = transaction ; inter-domaines = événement.** Exemple : appel clos → absences (tous deux D5) s'écrivent dans la même transaction (corrige le risque connu de la clôture d'appel) ; absences → notification famille (D5 → D0/D8) passe par un événement.
2. **Un événement est un fait passé, immuable**, nommé `domaine.fait` (les 151 noms existants sont conservés tels quels pour ne pas casser `ent_metier_event` ; les nouveaux sont en français).
3. **Charge utile minimale** : identifiants + ce qu'il faut pour décider sans relire ; **jamais** de donnée de santé, de motif médical, d'IBAN ou de texte libre sensible ; le consommateur relit ce dont il a besoin via le service du domaine propriétaire, avec ses propres droits.
4. **Un consommateur n'écrit que dans son propre domaine**, en appelant les commandes de ce domaine (qui émettent à leur tour leurs événements) : la toile reste traçable.
5. **Les envois externes** (mail, SMS, push) sont des effets de bord **finaux** : jamais rejoués lors d'un rejeu de projection.
6. **Les actions sensibles déclenchées par événement** (envoi en masse, facture émise, signalement) passent par une **file de validation humaine** et non par un effet automatique.

### 4.2 Catalogue des événements inter-domaines

Ordre = clé de partition (les événements d'une même clé sont traités dans l'ordre de `seq`). Existant = nom déjà présent dans `ent-events/src/lib.rs` [F].

| Événement | Producteur | Consommateurs | Charge utile minimale | Ordre par | Existant |
|---|---|---|---|---|---|
| `eleve.created` | D2 | D0 chronologie, D14 | eleve_id, statut | eleve | oui |
| `eleve.identity_changed` | D2 | D23 (exports), D0 | eleve_id, champs modifiés (noms seuls) | eleve | oui |
| `personne.creee` / `personne.coordonnees_modifiees` | D2 | D0 notif établissement, D10 (adresse de facturation) | personne_id | personne | non |
| `eleve.lien_updated` / `lien.suspendu` | D2 | D15 (révocation sessions), D0 (abonnements de notif), D8 (fils) | eleve_id, personne_id, droits | eleve | partiel |
| `admission.deposee` / `admission.decidee` | D3 | D8 (accusé, courrier), D14 | dossier_id, décision | dossier | non |
| **`inscription.validee`** | D3 | D1 affectation, D10 facturation, D11 grille repas, D12 internat, D2 invitation 2ᵉ parent, D8 message d'accueil, D9 documents à signer de rentrée, D24 consentements | eleve_id, annee_id, classe_id/niveau, régime, options, payeurs | eleve | non |
| `eleve.reinscription_confirmee` | D3 | D10 (acompte, facture N+1), D1 (projection N+1) | eleve_id, annee_cible | eleve | oui |
| `eleve.regime_changed` | D3 | D11 grille, D10 prorata, D12 internat, D5 règles de sortie | eleve_id, ancien, nouveau, date_effet | eleve | oui |
| `scolarite.classe_changed` / `referentiel.affectation_changee` | D1/D3 | D4 EDT élève, D7 cahier visible, D6 (bascule des notes de période), D8 (listes de diffusion), D0 | eleve_id, classe avant/après, date | eleve | oui (classe) |
| `eleve.sortie` | D3 | D10 solde/arrêt d'échéancier, D11 clôture grille, D12 internat, D15 désactivation des comptes, D23 exeat | eleve_id, date, motif | eleve | oui |
| `referentiel.annee_ouverte` / `_cloturee` | D1 | tous (bascule d'année), D10 campagne, D14 | annee_id | etablissement | non |
| `referentiel.service_cree` / `_cloture` | D1 | D15 (droits CLS), D4, D8 (listes), D6 | service_id, prof, cible | service | non |
| `edt.exception` / `edt.remplacement_organise` | D4 | D8/D0 familles et élèves touchés, D5 (appel du remplaçant), D7 (séance) | creneau_id, jour, nature | creneau | oui (exception) |
| `edt.annulation_masse` | D4 | D0 notification groupée, D11 (effectifs), D22 (études de remplacement) | liste de séances, motif public | etablissement | non |
| `appel.closed` | D5 | (intra D5 : absences/retards en transaction) ; D0 chronologie ; D11 effectifs du jour ; D14 | appel_id, creneau_id, jour, absents[], retards[] | appel | oui |
| `appel.non_fait_detecte` | D5 (planifié) | D0 rappel au prof, file CPE | creneau_id, jour, prof | creneau | non |
| **`eleve.absence_declared`** (source = appel/accueil/famille/séjour/stage/infirmerie) | D5 | D0 notification famille (si non prévenue), D7 (travail du jour pour rattrapage), D11 (repas si journée), D5 seuil (intra), D14 | eleve_id, absence_id, début, fin (séance/demi-journée), source, prévenue | eleve | oui (à enrichir) |
| `eleve.absence_justified` / `_refused` | D5 | D0 notification famille, D14 | eleve_id, absence_id | eleve | oui |
| `eleve.absence_seuil_atteint` | D5 | D0 alerte CPE + famille (une fois), D18 (équipe de suivi possible), D14 | eleve_id, compteur, période | eleve | oui |
| `vs.signalement_assiduite_emis` | D5 | D9 (document), D14 | eleve_id, mois | eleve | non |
| `vs.retard` | D5 | D0 notif famille (paramétrable), D5 cumul punition (intra) | eleve_id, minutes | eleve | oui |
| `eleve.incident_declared` | D5 | D0 CPE/PP, D14 | incident_id, eleve_ids | incident | oui |
| `eleve.sanction_prononcee` | D5 | D0 famille (courrier), D9 (document), D6 (conseil), D14 | sanction_id, eleve_id, type | eleve | oui |
| `vs.dispense` | D5 | D5 appel (intra), D7 (activité de remplacement), D16 (aménagement d'épreuve) | eleve_id, dates, objet (EPS…) | eleve | oui |
| `evaluation.publiee` | D6 | D0 notification élève/famille (regroupée), D14 | devoir_id, classe/groupe | devoir | non |
| `note.bulletin_publie` | D6 | D0 notification, D9 (archivage au dossier), D23 (LSU) | eleve_id, periode_id, bulletin_id | eleve | oui |
| `conseil.decision_prise` | D6 | D3 passage/redoublement, D17 avis d'orientation, D0 | eleve_id, décision | eleve | non |
| `cahier.travail_donne` / `cahier.travail_a_rendre` | D7 | D0 rappel élève la veille, D14 | travail_id, cible, pour_le | travail | partiel (`cahier.texte`) |
| `msg.envoye` | D8 | D0 notification destinataires (préférences), chronologie si eleve_id | fil_id, msg_id, destinataires | fil | oui |
| `annonce.publiee` | D8 | D0 diffusion | annonce_id, audience | annonce | oui |
| `doc.a_signer` / `doc.signe` | D9 | D0 rappels, D20 (dossier séjour complet), D13 (PAI), D3 (pièces) | doc_id, cible / signataire | doc | oui |
| `fac.facture_emise` | D10 | D0 notification payeur, D9 archivage PDF | facture_id, foyer_id, montant | facture | oui |
| `fac.sepa_rejet` / `fac.rappel_envoye` | D10 | D0 payeur, D14, D3 (blocage éventuel de réinscription : **décision Q11**) | facture_id, foyer_id | facture | oui |
| `fac.facture_soldee` | D10 | D3 (condition de réinscription), D0 | facture_id | facture | oui |
| `repas.volume_valide` | D11 | D10 facturation consommation | eleve_id, mois, quantité | eleve | non |
| `eleve.dispense_repas_posed` | D11 | D10 (prorata), D11 effectifs (intra) | eleve_id, dates | eleve | oui |
| `internat.absence_declaree` | D12 | D11 (repas), D0 | eleve_id, soirées | eleve | non |
| `stage.periode_ouverte` | D12 | D5 (appel pré-rempli « en stage »), D11 dispense, D4 | eleve_id, dates | eleve | non |
| `infirmerie.passage` | D13 | D0 famille (sans détail), D5 (absence au cours si l'élève n'y retourne pas) | eleve_id, début, fin, issue | eleve | oui |
| `sante.contrainte_publiee` | D13 | D11 (alerte passage), D20 (séjour), D16 (examen), D0 profs de CLS | eleve_id, type de contrainte non médicale | eleve | non |
| `examen.session_creee` / `examen.convocations_publiees` | D16 | D4 neutralisation, D0, D9 | session_id | session | non |
| `accompagnement.amenagements_definis` | D18 | D16 placements, D6 (affichage profs), D0 | eleve_id, aménagements (codes) | eleve | non |
| `rh.absence_validee` | D19 | D4 (séances touchées), D0 (familles), D5 (appel non attendu) | personnel_id, période | personnel | oui |
| `sejour.valide` / `sejour.demarre` / `sejour.cloture` | D20 | D10 facturation, D5 absences « séjour » des participants, D11 dispenses, D4 cours des encadrants, D0 | sejour_id, participants[] | sejour | non |
| `consentement.donne` / `_retire` | D24 | D2 (photo/trombi publié ou non), D20, D8 (publications) | eleve_id, objet, version | eleve | non |
| `import.lot_applique` | D23 | D0, D14 | lot_id, comptes | lot | non |
| `delegation.accordee` / `_expiree` | D15 | D15 (droits), D0 | délégué, portée, fin | delegation | non |

### 4.3 Qui réagit à quoi (lecture « par déclencheur »)

| Déclencheur | Réactions (domaine → effet) |
|---|---|
| `inscription.validee` | D1 crée l'affectation à la classe ; D10 génère facture(s) et échéancier selon formules et payeurs, puis propose le mandat SEPA (D-014) ; D11 pose la grille repas si DP ; D12 ouvre l'affectation internat si interne ; D2 invite le second responsable ; D9 publie les documents de rentrée à signer ; D24 demande les consentements ; D8 envoie le message d'accueil ; D0 ouvre la chronologie |
| `eleve.absence_declared` (source appel, non prévenue) | D0 notifie les responsables avec `peut_voir_vs` (SMS si paramétré) ; D5 recalcule le compteur (intra) → seuil éventuel ; D7 rattache le travail du jour à l'absence (« à rattraper ») ; D11 retire l'élève des effectifs si journée entière ; D14 met à jour l'absentéisme |
| `eleve.absence_declared` (source accueil/famille) | D5 marque les séances « prévenues » : l'appel les affiche pré-remplies ; D11 décompte le repas si délai respecté ; aucune notification famille (c'est elle qui a prévenu) |
| `eleve.absence_seuil_atteint` | D0 alerte le CPE et le PP ; une notification famille unique ; D18 propose une équipe de suivi ; D5 pré-calcule le décompte pour le signalement (VS-21) |
| `appel.non_fait_detecte` | D0 rappel push au professeur ; après N minutes, entrée dans le tableau de vérification CPE |
| `rh.absence_validee` | D4 liste les séances touchées et propose remplacement/annulation ; D0 prévient les familles des classes (après validation humaine) ; D5 n'attend plus d'appel sur ces séances |
| `edt.exception` (annulation) | D0 prévient élèves et familles concernés ; D7 marque le travail prévu « séance annulée » ; D22 propose l'étude |
| `eleve.regime_changed` | D11 ferme/ouvre la grille ; D10 calcule le prorata et l'avoir ou complément ; D12 affectation internat ; D5 règle de sortie recalculée |
| `eleve.sortie` | D10 arrête l'échéancier et calcule le solde ; D11 clôt la grille ; D12 libère la chambre ; D15 désactive le compte élève et ses accès ; D2 conserve le dossier en lecture (archivage) ; D23 prépare l'exeat |
| `stage.periode_ouverte` / `sejour.demarre` | D5 crée les absences « stage » / « séjour » (motif justifié d'office) ; D11 pose les dispenses repas ; D4 annule les cours des encadrants ; D0 prévient |
| `infirmerie.passage` (issue : famille appelée) | D5 crée une absence source infirmerie à partir de l'heure ; D0 notification famille sans détail médical ; sortie tracée (D12) |
| `sante.contrainte_publiee` | D11 affiche l'alerte au passage ; D20 la recopie (indicateur seulement) dans la fiche d'urgence du séjour ; D16 la prend en compte au placement |
| `conseil.decision_prise` | D3 prépare le passage/redoublement N+1 ; D17 enregistre l'avis d'orientation ; D0 notifie la famille à la publication du bulletin, pas avant |
| `note.bulletin_publie` | D0 notifie ; D9 range le PDF au dossier ; D23 alimente l'export LSU |
| `fac.sepa_rejet` | D0 notifie le payeur ; D10 rouvre l'échéance ; D14 impayés ; (Q11) D3 peut conditionner la réinscription |
| `consentement.retire` (droit à l'image) | D2 masque la photo hors usage administratif ; D8 empêche son usage dans de nouvelles publications |
| `lien.suspendu` | D15 révoque les sessions du responsable sur cet élève ; D0 retire ses abonnements ; D8 le retire des fils concernant l'élève |
| `referentiel.annee_cloturee` | tous les domaines passent l'année en lecture seule ; D14 fige les indicateurs ; D0 planifie les purges (PLT-09) |

### 4.4 Architecture technique proposée (compatible `ent-events`) [R]

**Évolution, pas remplacement.** `ent_metier_event` reste le journal ; `MetierEvent`, `MetierEventDraft`, `AggregateKind` et les constantes restent valides.

1. **Outbox transactionnelle** (migration additive) :
   - `ent_metier_event` + colonnes `seq BIGSERIAL UNIQUE` (ordre global), `schema_version SMALLINT NOT NULL DEFAULT 1`, `correlation_id UUID` (référence de requête de L6), `causation_id UUID` (événement déclencheur), `profondeur SMALLINT DEFAULT 0` (garde-fou contre les boucles, refus au-delà de 5).
   - Nouvelle méthode `publish_tx(&mut sqlx::Transaction<'_, Postgres>, draft)` : insère dans la transaction du fait métier, **sans** appeler de handler. L'ancienne `publish` reste pour la compatibilité et est retirée progressivement (164 appels [F], migrés domaine par domaine, en commençant par la clôture d'appel).
   - `pg_notify('ent_evenement', seq)` émis dans la même transaction (réveil des consommateurs ; le polling reste le filet de sécurité).
2. **Consommateurs** (inbox pattern) :
   - `ent_event_consommateur (nom PK, actif, rejouable, derniere_seq)` ; `ent_event_traitement (consommateur, event_id, statut traite|echec|ignore, tentatives, derniere_erreur, traite_le, PRIMARY KEY (consommateur, event_id))`.
   - Chaque consommateur fait **dans une seule transaction** : son effet métier + l'insertion dans `ent_event_traitement` → **idempotence** par la clé primaire (un second passage est un no-op).
   - Distributeur : tâche de fond dans le processus axum (puis processus séparé si besoin), lecture par lot `WHERE seq > derniere_seq ORDER BY seq` avec `FOR UPDATE SKIP LOCKED` sur un verrou par consommateur ; **ordre garanti par clé de partition** (agrégat) ; parallélisme par hachage de la clé.
   - Échecs : reprise avec délai croissant, puis file d'échecs (PLT-04) visible des administrateurs ; un échec ne bloque que sa partition.
3. **Événements typés** : module `ent-types::evenements` (sans dépendance serveur, partagé avec Leptos et mobile pour la chronologie) : un `enum` par domaine, `#[serde(tag = "type")]`, une structure par événement, `schema_version` ; conversion `TryFrom<&MetierEvent>` ; les consommateurs ne lisent plus de JSON libre. Tests de compatibilité : chaque type a un exemple JSON figé.
4. **Notifications (D0)** : un seul consommateur « Notifications » transforme les événements en `ent_notification` selon `ent_regle_notification` (événement → destinataires → gabarit → canaux) et `ent_preference_notification` ; il **revérifie les droits** du destinataire au moment de l'envoi (lien actif, `peut_voir_vs`…) ; regroupement (digest), heures calmes, plafond SMS ; les trois outbox existantes deviennent les canaux de sortie de `ent_notification_envoi`.
5. **Projections rejouables** : chronologie du dossier élève, indicateurs D14, effectifs cuisine ; reconstruites par rejeu (PLT-03) ; jamais d'envoi externe pendant un rejeu (drapeau `rejouable` faux pour les consommateurs d'envoi).
6. **Cloisonnement** : le consommateur agit avec un acteur « système » **borné à l'`etablissement_id` de l'événement** (type `Tenant` du socle 1, C3) ; jamais d'événement d'un établissement traité dans le contexte d'un autre ; test de séparation dédié.
7. **Traçabilité** : chaîne `correlation_id` / `causation_id` consultable (« pourquoi ai-je reçu ce message ? », « d'où vient cette facture ? ») ; le journal d'audit reste distinct (actions humaines).
8. **Rétention** : événements = données personnelles : durée alignée sur le domaine producteur (PLT-09) ; purge = suppression de la charge utile en gardant l'enveloppe technique si nécessaire au rejeu.
9. **Brain** : `react()` devient un consommateur comme les autres ; `EnsureAbsence` après `appel.closed` repasse **en transaction dans D5** (intra-domaine) ; le Brain garde les réactions vraiment inter-domaines.

Points d'accroche socle 3 (IA « chat first ») à préparer sans rien implémenter : le **catalogue d'actions** (§3, identifiants stables) = liste des outils que l'IA pourra proposer, exécutés par les mêmes services et droits ; la **chronologie typée** = contexte lisible par l'IA dans le périmètre de l'utilisateur ; les **files de validation humaine** (règle 6) = point de confirmation obligatoire ; la **chaîne de causalité** = explicabilité.

---

## 5. Au-delà d'un ENT classique (dans le socle 2)

Effort : **S** ≤ 3 jours, **M** 1–2 semaines, **L** > 2 semaines (développement assisté, tests compris) [H].

| # | Amélioration | Pourquoi c'est mieux que l'existant du marché | Effort | Dépend de |
|---|---|---|---|---|
| A1 | **Appel pré-rempli** : absences déclarées (accueil, famille), dispenses, stage, séjour, infirmerie, examen affichés « attendus absents » | moins d'erreurs, moins d'appels téléphoniques de la VS | M | VS-06, §4 |
| A2 | **Vérification d'appel en temps réel** (séances sans appel à H+15, relance push, tableau CPE) | obligation tenue sans pointage manuel | S | VS-04, EDT-06 |
| A3 | **« À faire » famille unifié** : une seule liste (signer, justifier, payer, répondre, réinscrire, prendre RDV) avec échéances | les familles ne cherchent plus dans 10 menus | M | D0, D9, D10, D24 |
| A4 | **Chronologie du dossier élève** filtrée par droits | vue 360° réelle, que les ENT cloisonnés par module n'ont pas | M | PLT-07, événements typés |
| A5 | **Décompte réglementaire automatique** (demi-journées, mois glissant) et dossier de signalement pré-rempli | sécurise le chef d'établissement | M | absences par séance |
| A6 | **Détection précoce explicable** (règles : absences répétées le même jour, chute de moyenne, cumul de punitions) → alerte PP/CPE avec raisons | prévention du décrochage sans IA opaque | M | D5, D6, §4 |
| A7 | **Conseil de classe préparé** : tableau automatique, appréciations manquantes, propositions de mentions selon règles | gain de temps PP/direction | M | EVA-08 |
| A8 | **Facturation pilotée par les faits** (repas, études, séjours, régime) avec prorata automatique | fin des ressaisies compta | M | FIN-06, §4 |
| A9 | **Garde alternée et multi-payeurs** (répartition %, envois aux deux foyers, droits distincts) | cas fréquent mal géré | M | DOS-07, DOS-16 |
| A10 | **Délégations datées** (remplaçant, organisateur, PP intérim) qui expirent seules | moins de comptes sur-privilégiés | S | ADM-04 |
| A11 | **Journal d'accès visible par la famille** (« le dossier de votre enfant a été consulté par : vie scolaire, 3 fois ») | transparence RGPD rare sur le marché | M | PIL-07, Q9 |
| A12 | **Droit d'accès RGPD en un clic** | obligation légale outillée | M | PLT-08 |
| A13 | **Notifications sobres** : regroupement, heures calmes, canal préféré, aucune notification en double | moins de fatigue des familles | M | D0 |
| A14 | **Annulation en masse** (grève, intempéries) avec effets automatiques (familles, cantine, études) | réaction en 2 minutes au lieu d'une matinée | M | EDT-08, §4 |
| A15 | **Contrôles de cohérence inter-domaines** (PAI ↔ séjour, aménagements ↔ examen, régime ↔ grille repas, consentement ↔ photo publiée) | erreurs évitées avant qu'elles arrivent | M | §4 |
| A16 | **Mode hors ligne mobile pour l'appel et la fiche d'urgence de séjour** | sorties, salles sans réseau | L | appli mobile |
| A17 | **Réinscription en 3 gestes** (tout pré-rempli, seules les différences demandées) | taux de réponse élevé, moins de relances | S | INS-07 |
| A18 | **Imports « sans surprise »** : prévisualisation, différences, rejets ligne à ligne, annulation d'un lot | les imports SIECLE sont une source d'erreurs récurrentes [H] | M | IOP-01 |
| A19 | **Vote électronique des délégués** avec secret garanti | peu couvert par les ENT [H] | M | D25 |
| A20 | **Explication des effets** : avant une action (« changer le régime entraînera : avoir de 42 €, grille repas fermée… ») | l'utilisateur voit la toile avant de la déclencher | M | §4, simulation à blanc |

---

## 6. Ordre de réalisation (lots livrables)

Principes : on part de l'existant (back large, Leptos étroit) ; chaque lot livre **back manquant + Leptos + tests + doc** pour ses actions ; les lots 0 et 1 sont des prérequis ; volume en semaines de développement assisté [H], pour un agent principal + revues.

| Lot | Périmètre (actions) | Dépend de | Critère de fin | Tests attendus | Volume [H] |
|---|---|---|---|---|---|
| **S2-0 Toile** | outbox (`publish_tx`, `seq`, corrélation), consommateurs idempotents, file d'échecs, `ent-types::evenements` (lot 1 : D2, D3, D5), consommateur Notifications (mail d'abord), clôture d'appel transactionnelle ; PLT-01→06 | socle 1 C3 (`Tenant`) | appel clos ⇒ absences + notification famille, même après panne simulée ; rejeu sans double envoi | tests d'intégration Postgres : panne entre fait et événement, double livraison, ordre par agrégat, séparation inter-établissements ; tests de schéma JSON | 3–4 sem. ; ~5 tables, 0 écran (+1 écran admin des échecs) |
| **S2-1 Dossier élève & personnes** | `ent_personne` (V8), liens d'autorité (B7), contacts d'urgence, payeurs, fiche 360° Leptos, chronologie ; DOS-01→16, PLT-07 | S2-0 | une famille avec parent sans compte, parents séparés, élève majeur, gérée de bout en bout sans `ui.rs` | migration additive avec reprise des liens ; matrice droits × périmètres ; fusion de doublons réversible | 4 sem. ; ~6 tables, ~25 routes, 6 écrans |
| **S2-2 Référentiel complet** | sites, niveaux/MEF, périodes rapatriées, calendrier, grille horaire, séances datées, `classe_id` dans les 9 tables texte (V1) ; REF-01→15, EDT-03, EDT-06 | S2-0 | plus aucune lecture métier sur `classe` texte ; bascule d'année testée | migration de rattachement idempotente, comparaison avant/après sur données de labo, clôture d'année | 3–4 sem. ; ~6 tables, ~20 routes, 5 écrans |
| **S2-3 Administratif : admissions et inscriptions** | INS-01→15, D-014 (préinscription vérifiée, 2ᵉ parent, IBAN masqué), IOP-01/04 (import SIECLE) | S2-1, S2-2 | `inscription.validee` déclenche affectation, invitation, documents, message ; une campagne de réinscription menée en Leptos | parcours famille de bout en bout (anti-énumération de comptes), import avec rejets, réactions §4.3 | 4–5 sem. ; ~6 tables, ~30 routes, 10 écrans |
| **S2-4 Vie scolaire complète** | absences par séance/demi-journée, déclarations accueil/famille, vérification d'appel, punitions ≠ sanctions, procédure disciplinaire, signalement, appel mobile ; VS-01→25, A1, A2, A5 | S2-2, S2-0 | une journée type CPE/accueil/prof sans ancienne UI ; décompte demi-journées exact | cas limites d'horaires, rapprochement déclaration/constat, effacement planifié des sanctions, séparation | 4–5 sem. ; ~6 tables, ~25 routes, 8 écrans |
| **S2-5 EDT admin & remplacements** | EDT-01→08, RH-01→05, ADM-04 délégations | S2-2 | absence de prof ⇒ séances listées ⇒ remplacement ⇒ familles prévenues | conflits, A/B, vacances, annulation en masse, droits temporaires expirés | 3 sem. |
| **S2-6 Cahier de textes et travail** | CDT-01→10 | S2-2 (séances) | un prof tient son cahier et reçoit des copies ; élève/famille voient l'agenda | pièces privées, échéances, visibilité par groupe | 2–3 sem. |
| **S2-7 Évaluations, bulletins, conseils, LSU** | EVA-01→14, ORI-01→04, D23 exports LSU/LSL au format officiel | S2-2, S2-4 | une période complète : saisie grille → conseil → bulletin publié → export | calculs de moyennes, verrouillage de période, version de bulletin, conformité de format (fichiers de référence) | 5–6 sem. |
| **S2-8 Communication complète** | COM-01→09, campagnes avec confirmation, SMS OVH (D-004), push réel, D21 rendez-vous et réservations, D24 formulaires et consentements | S2-0 | rencontres parents-profs organisées ; consentement droit à l'image appliqué au trombinoscope | préférences, regroupement, plafond SMS, anti-abus, droits de ciblage | 4 sem. |
| **S2-9 Finances reliées** | FIN-01→17, génération annuelle, facturation par événements, multi-payeurs, paiement en ligne (si Q5) | S2-3, S2-0 | une rentrée facturée sans ressaisie ; un rejet SEPA traité ; export comptable | idempotence par source, avoirs, prorata, arrondis, rejeu sans double facture | 5–6 sem. |
| **S2-10 Restauration** | RES-01→09 | S2-9, S2-4 | réservation + effectifs cuisine + volume facturé | prévision vs passages, délais de prévenance | 2 sem. |
| **S2-11 Santé et accompagnement** | SAN-01→08, ACC-01→05 | S2-1, socle 1 D3 (chiffrement) | PAI → contrainte publiée → alertes cantine/séjour sans fuite médicale | tests de minimisation (aucune donnée santé dans charges utiles et listes), journal de consultation | 3 sem. |
| **S2-12 Internat, sorties, stages, séjours** | INT-01→10, SEJ-01→10 | S2-9, S2-4, S2-11 | un séjour mené de la validation à la clôture avec facturation et absences automatiques | effets croisés §4.3, désistement, fiche d'urgence | 4–5 sem. |
| **S2-13 Examens, orientation, activités, instances** | EXA-01→07, ORI-05/06, PER-01→05, INSTA-01→05 | S2-5, S2-11 | un brevet blanc organisé ; élection des délégués | placements avec aménagements, secret du vote | 4 sem. |
| **S2-14 Pilotage** | PIL-01→08, indicateurs documentés | S2-4, S2-7, S2-9 | tableau de bord direction sur projections rejouables | cohérence projections/sources, droits sur agrégats | 2–3 sem. |
| **S2-15 Interopérabilité publique** | IOP-02, 03, 06, 07 (STS, ONDE, ÉduConnect, GAR) | Q1 | selon décision Q1 | conformité aux spécifications officielles | 4–6 sem. [H forte] |
| **Transverse mobile** | appel, absences, notes, messagerie, agenda, à faire (A3), fiche d'urgence | S2-4, S2-7, S2-8 | une appli par plateforme, tous rôles (D-003) | parité des droits API/mobile | en parallèle dès S2-4 |
| **Sortie de socle** | retrait de `ui.rs` domaine par domaine, préparation pilote | tous | plus aucune route de l'ancienne UI utilisée | régression navigateur | — |

Total indicatif [H] : 60–75 semaines-agent hors mobile ; parallélisable à partir de S2-2 (D6/D7/D8/D11/D13 n'ont pas de tables communes, cf. cartographie §3.4). À rapprocher de l'objectif de sortie fin 2027 (D-007) : **la parallélisation est nécessaire**.

### 6.1 S2-0 — réalisé (2026-11-05, branche `ent-socle2-outbox`)

**Niveau : implémenté + testé en local** (Postgres jetable). Non validé pilote, non fusionné. Première tranche de S2-0 : la fondation (outbox, distribution, idempotence) et trois réactions ; le reste du lot est listé en « reste à faire ».

**Conception livrée [F]**

| Élément | Où | Détail |
|---|---|---|
| Migration additive | `ent-rust/migrations/102_outbox.sql` (`MIGRATION_TAG = "102_outbox"`) | `ent_outbox` (état de distribution, une ligne par événement : `seq` global, `seq_agregat` par agrégat, `etablissement_id` obligatoire et non nul, `correlation_id`, `causation_id`, `profondeur` ≤ 5, `schema_version`, statut `a_distribuer` / `distribue` / `echec`, tentatives, prochaine tentative, dernière erreur) ; `ent_outbox_agregat` (compteur d'ordre par agrégat) ; `ent_event_traitement` (marque d'idempotence par consommateur, clé `(consommateur, event_id)`) ; `ent_notification` (notifications internes D0, clé unique `(source_event_id, destinataire, kind)`). Aucune table existante modifiée. |
| Publication en transaction | `ent-db/src/outbox.rs`, trait `Publier` | `tx.publier(draft)` / `tx.publier_avec(draft, causalite)` sur toute `Transaction<Postgres>` (et sur la transaction d'un consommateur) : écrit le journal `ent_metier_event` **et** l'outbox dans la transaction du fait. Le compteur d'agrégat reste verrouillé jusqu'au commit : l'ordre `seq_agregat` est l'ordre de validation. |
| Distribution | `Distributeur` (même fichier), lancé par `ent-api/src/main.rs` (désactivable : `ENT_OUTBOX_DISTRIBUTEUR=0`) | Tâche tokio ; lots `FOR NO KEY UPDATE SKIP LOCKED` (plusieurs distributeurs possibles) ; un événement n'est pris que si tous ses prédécesseurs du même agrégat sont distribués ; reprise avec délai doublé (2 s → 15 min, réglable), plafond de tentatives (8 par défaut) puis file des échecs (`file_des_echecs`, `rejouer_echec`) qui bloque seulement son agrégat ; rejeu d'un consommateur (`rejouer_consommateur`, refusé pour un consommateur non rejouable) ; métriques en mémoire (`metriques()`) et volumes par statut (`etat()`). |
| Consommateurs | trait `Consommateur` ; registre par type d'événement ; `PgEnt::distributeur()` installe les consommateurs standard | Chaque traitement : une transaction = marque `ent_event_traitement` + effet ; une seconde livraison est un doublon ignoré. Transaction bornée à l'établissement de l'événement (`set_config('ent.etablissement_id', …, true)`, prête pour une RLS) ; le consommateur ne reçoit que cet établissement. |
| Lecture | `PgEnt::mes_notifications`, route `GET /notifications` (session obligatoire) | Notifications de l'utilisateur dans l'établissement de sa session. |

**Fonctions migrées vers l'outbox (fait + événement dans une seule transaction)** : `clore_appel` (clôture + absences + retards + `appel.closed` + un `eleve.absence_declared` `source = appel` par absent + `vs.retard` ; absences et retards ont `causation_id` = `appel.closed`) ; `accepter_justificatif` et `refuser_justificatif` (`eleve.absence_justified` / `_refused`). L'application de `BrainAction::EnsureAbsence` après `appel.closed` est supprimée du service : l'absence est écrite en transaction dans D5 (règle §4.1-1, point 9 du §4.4). Le journal en mémoire du Brain reste alimenté après commit (`diffuser_locaux`).

**Réactions livrées (consommateur `d0.notifications`, `ent-db/src/reactions.rs`)**

| Déclencheur | Réaction | Règles |
|---|---|---|
| `eleve.absence_declared`, source appel | notification « Absence de … » aux responsables | absence déclarée par la famille ou l'accueil : rien (§4.3) |
| `eleve.absence_justified` / `eleve.absence_refused` | notification de la décision | la note de décision (texte libre) n'est jamais recopiée |
| `vs.retard` | notification « Retard de … » | — |

Destinataires relus **au traitement** : lien `ent_eleve_lien` de l'établissement avec `peut_voir_vs`, compte membre de rôle `parent` ; élève relu dans l'établissement de l'événement (sinon rien). Rejouable (aucun envoi externe).

**Tests (`ent-db/src/tests_outbox.rs`, feature `labo`)** : transaction annulée → aucun événement ; panne simulée de l'outbox au milieu de `clore_appel` (déclencheur SQL) → appel encore ouvert, aucune absence, aucun événement, puis clôture réussie après réparation ; double distribution et deux distributeurs concurrents → un seul effet ; ordre par agrégat avec un échec intercalé ; échec → reprise planifiée avec délai → file des échecs → correction → rejeu dans l'ordre, consommateur déjà passé non rejoué ; rejeu d'une projection sans rejouer l'envoi externe ; isolation (événement forgé dans un établissement citant l'élève d'un autre → aucune notification, transaction bornée, distributeur restreint) ; réactions de bout en bout (absence constatée, droits `peut_voir_vs` respectés, chaîne de causalité, justificatif accepté et refusé, droit retiré avant distribution → rien, retard à l'appel, absence famille non notifiée) ; boucle tokio.

**Résultats (2026-11-05, Postgres 16 local jetable)** : `tests_outbox` 13/13 ; série ciblée `-p ent-events -p ent-db -p ent-api --features ent-api/labo` : 236 réussis, 0 échec (ent-db 101, ent-api 133, ent-events 2) ; suite complète du workspace (`--features ent-api/labo,ent-lab/labo`) : 492 réussis, 1 ignoré, **1 échec préexistant** hors périmètre (`ent-types` `session_aplatie_et_defi_passkey`, divergence `MeDto` déjà signalée, échoue aussi sur le commit de départ `5761dc8`) ; `cargo build --locked -p ent-api` réussi. Aucune dépendance modifiée (`Cargo.lock` inchangés). CI GitHub non vérifiée.

**Écarts par rapport au §4.4 [signalés]**
- Table `ent_outbox` séparée plutôt que colonnes ajoutées à `ent_metier_event` : le journal reste inchangé (aucune réécriture d'une table volumineuse), l'outbox porte l'état de distribution et pointe le journal (`ON DELETE CASCADE` : la purge PLT-09 du journal emporte l'outbox).
- Marque de traitement **par événement** (`ent_event_traitement`) au lieu d'un curseur `derniere_seq` par consommateur : un curseur sur `seq` saute les transactions validées dans le désordre ; l'état par événement permet `SKIP LOCKED`, reprise et file des échecs. La table `ent_event_consommateur` (actif, rejouable) n'est pas créée : le registre est dans le code (`Consommateur::rejouable`).
- `pg_notify` non branché : sondage toutes les 500 ms (réveil à brancher si la latence l'exige).
- Notifications : canal interne seulement ; le canal mail du §6 S2-0 (« mail d'abord ») n'est pas fait.

**Reste à faire dans S2-0 (plan de migration progressive)**
1. `ent-types::evenements` (événements typés D2, D3, D5, exemples JSON figés) ; les consommateurs lisent encore du JSON.
2. Seuil d'absences (`apres_absence_ecrite` : signal, fil d'alerte, mail) encore exécuté **après** le commit de `clore_appel` : à transformer en consommateur D5 (signal) + consommateur notifications (mail, non rejouable).
3. Consommateur d'envoi (mail via `ent_mail_outbox`, puis SMS OVH D-004 et push), préférences, regroupement, heures calmes.
4. Écran d'administration de la file des échecs (API `file_des_echecs` / `rejouer_echec` prête) et exposition des métriques dans `/metrics`.
5. Migration des ~160 autres `self.events.publish(…)` hors transaction, domaine par domaine, en commençant par ceux qui ont un consommateur : publication du bulletin (`note.bulletin_publie`, dépend de `deposer_document` en transaction), sanctions, inscriptions (`inscription.validee` à créer en S2-3). Règle : un événement n'entre dans la toile (outbox) que publié en transaction ; l'ancien `publish` n'alimente que le journal.
6. Les charges utiles existantes contenant du texte libre (`decision_note`, `motif`) sont conservées pour compatibilité : à minimiser lors du typage (règle §4.1-3).

### 6.2 S2-1 — réalisé (2026-11-05, branche `ent-socle2-dossier`, fusionne `ent-socle2-outbox`)

**Niveau : implémenté + testé en local** (Postgres jetable). Non validé pilote, non fusionné. Couvre le cœur du lot (personne distincte du compte, liens d'autorité, contacts, payeurs, fusion réversible, fiche 360° Leptos, chronologie) ; le reste est listé plus bas.

**Données (migrations additives `120`→`123`, `MIGRATION_TAG = "123_personne_fusion"`) [F]**

| Migration | Contenu |
|---|---|
| `120_personne.sql` | `ent_personne` (D-020) : responsable, contact ou élève majeur, **propre à l'établissement** (D-013/D-014), compte `user_id` facultatif (unique par établissement), `eleve_id` (fiche de l'élève lui-même), `fusionnee_dans`, `source` (`saisie`, `reprise`, `compte`). `ent_eleve_lien.personne_id` **obligatoire** ; `user_id` devient facultatif et la clé étrangère passe en `ON DELETE SET NULL` (supprimer un compte ne supprime plus l'autorité). **Reprise idempotente** : une personne par (établissement, compte) déjà lié, nom affiché du compte repris tel quel (aucun découpage prénom/nom deviné). Déclencheurs : un lien créé par compte (code existant, `link_parent`, seed) reçoit sa personne ; un lien créé par personne reçoit le compte de la personne ; personne d'un autre établissement refusée ; le compte de la personne est recopié sur ses liens. |
| `121_lien_autorite.sql` | Lien : `autorite_parentale` (présumée `true` pour les liens repris, posée une fois), `garde` (`exclusive`/`alternee`/`aucune`), `date_debut`/`date_fin`, suspension (`suspendu_le`, `suspendu_par`, `suspension_motif` obligatoire, `suspension_reference`). **Vue `ent_eleve_lien_actif`** : non suspendu, dans ses dates (UTC), personne non fusionnée. Droits fins existants conservés. |
| `122_contact_urgence.sql` | `ent_eleve_contact_urgence` (DOS-08) : personne, relation, ordre, `peut_recuperer`, statut `valide`/`propose`. |
| `123_personne_fusion.sql` | `ent_personne_fusion` : instantané JSON de la personne absorbée (liens et contacts déplacés ou retirés, compte transféré), motif, annulation. |

**Point unique des contrôles famille** : `PgFoyerRepo::lien_parent_eleve` et `liens_for_parent` (donc `PgEnt::lien_parent_eleve`, `eleves_lies`, `mes_enfants_for_parent`, `get_eleve_for_actor` et les ~60 contrôles famille de `service.rs`) lisent `ent_eleve_lien_actif` ; idem e-mails / téléphones des responsables, appareils push (`list_liens_eleve_actifs`) et le consommateur `d0.notifications`. Aucune lecture directe restante pour un accès famille ; les lectures brutes restantes (`list_liens_eleve`, `get_lien`, `list_liens_dossier`, `list_liens_foyer`, `first_eleve_id_foyer`) servent à l'administration et à la facturation et ne retournent que les liens avec compte (structures existantes à `user_id` non nul). Invariant des parts payeurs : calculé sur **tous** les liens (avec ou sans compte).

**Service (`ent-db/src/dossier_personnes.rs`) et API (`ent-api/src/dossier.rs`, 21 routes `/dossier/*`, types `ent-types::dossier`)**

| Action | Route | Droit (catalogue D-017) |
|---|---|---|
| DOS-03 créer / modifier une personne (sans e-mail possible), rechercher (homonymes) | `POST/GET /dossier/personnes`, `GET/PUT /dossier/personnes/{id}` | `personnes.gerer` (nouveau ; modèles : admin, direction, administratif) |
| Rattacher un compte responsable de l'établissement | `PUT /dossier/personnes/{id}/compte` | `personnes.gerer` |
| DOS-05 lier / modifier un lien (autorité, garde, dates, droits fins, foyer) | `POST /dossier/eleves/{id}/responsables`, `PUT /dossier/liens/{id}` | `personnes.gerer` |
| DOS-06 ★ suspendre / rétablir | `POST /dossier/liens/{id}/suspendre|retablir` | `liens.suspendre` (nouveau ; admin, direction) |
| DOS-07 payeurs (parts = 100 %, payeur rattaché à un foyer) | `PUT /dossier/eleves/{id}/payeurs` | `payeurs.definir` (nouveau ; + comptabilité) |
| DOS-08 contacts : ajout (personnel → validé ; responsable actif → proposé), validation, retrait | `POST /dossier/eleves/{id}/contacts`, `POST /dossier/contacts/{id}/valider`, `DELETE /dossier/contacts/{id}` | `personnes.gerer` / lien actif |
| DOS-14 (partiel) fiche « personne » de l'élève majeur (payeur, lien `eleve_majeur` sans autorité ni droits famille) | `POST /dossier/eleves/{id}/personne-eleve` | `personnes.gerer` |
| DOS-15 ★ (personnes) fusion / défusion ≤ 30 jours | `GET/POST /dossier/fusions`, `POST /dossier/fusions/{id}/annuler` | `personnes.fusionner` (nouveau ; admin, direction) |
| DOS-13 fiche 360° | `GET /dossier/eleves/{id}/fiche` | droits existants du dossier (sections), lien actif |
| PLT-07 chronologie | `GET /dossier/eleves/{id}/chronologie` | idem |

Règles : les quatre nouvelles actions n'acceptent que la portée « établissement » ; familles minimisées (autres responsables sans coordonnées ni parts, liens inactifs et motifs de suspension masqués) ; un responsable ne parcourt jamais les personnes de l'établissement ; une référence d'un autre établissement répond 404 ; deux personnes avec des comptes différents ne sont jamais fusionnées (D-013). Chronologie : liste blanche de types d'événements → titre **neutre** fixe (aucun texte de charge utile recopié) + entrées du journal (mêmes règles que la route journal) ; familles : scolarité, vie scolaire, notes selon leurs droits ; santé et psy visibles des seuls rôles infirmerie / psy, titre neutre. Alertes de fiche (personnel) : parents séparés sans e-mail (D-014), payeur sans foyer ou suspendu, aucune autorité parentale active.

**Événements (outbox S2-0, en transaction avec le fait et le journal)** : `personne.creee`, `personne.coordonnees_modifiees` (noms des champs seulement), `personne.compte_rattache`, `personne.fusionnee`, `personne.defusionnee` (agrégat `personne`, ajouté à `AggregateKind`) ; `eleve.lien_updated` (existant, désormais émis), `lien.suspendu`, `lien.retabli`, `eleve.contacts_urgence_changes`, `foyer.payeur_defini` (agrégat élève). Le motif de suspension n'est ni dans l'événement ni dans le journal. Aucun consommateur nouveau : l'effet « accès coupé » est obtenu par la relecture des liens actifs à chaque requête et par le consommateur de notifications existant.

**Frontend Leptos** : `/web/eleves/{id}` (`ent-web-app/src/modules/dossier/`) : en-tête identité/scolarité (minimisée par le serveur), alertes, responsables (état, autorité, garde, droits, payeur, coordonnées), contacts d'urgence, sections existantes accessibles avec leur contenu résumé et lien vers l'écran Leptos existant, actions proposées par le serveur, chronologie filtrable par domaine. Actions : ajouter un responsable (avec ou sans e-mail), définir les payeurs, ajouter / proposer / valider / retirer un contact, suspendre (motif obligatoire) / rétablir un lien. Accès depuis la liste d'élèves d'une classe. Logique d'écran pure testée (`modele.rs`).

**Tests** : `ent-db/src/tests_dossier_personnes.rs` (8 scénarios Postgres : reprise idempotente rejouée et code existant, parent sans compte + parents séparés + contacts + payeurs, élève majeur, suspension → accès coupé partout (12 points d'accès, notifications, motif absent des événements) puis rétabli, lien hors dates, matrice 8 profils × 5 actions + rôle paramétré, isolation entre établissements (y compris SQL direct), fusion / défusion à l'identique + délai de 30 jours + compte transféré, chronologie filtrée) ; 5 tests unitaires (chronologie, alertes, saisie) ; `ent-api/src/tests_dossier.rs` (HTTP : codes 201/400/403/404/409, minimisation, suspension sur routes existantes et nouvelles, payeurs, fusion, isolation) ; `ent-types` (aller-retour JSON) ; `ent-web-app` (5 tests du modèle) ; `ent-web-server/tests/dossier.rs` (rendu serveur, parcours complet contre le vrai backend).

**Écarts par rapport à la conception [signalés]**
- Pas de table `ent_lien_responsabilite` séparée : `ent_eleve_lien` **est** le lien de responsabilité (une seule source de vérité, code existant inchangé) ; `ent_personne_contact` et `ent_foyer_membre` non créés (coordonnées sur la personne ; foyer porté par le lien comme avant).
- DOS-15 livré pour les **personnes** ; la fusion de deux **dossiers élèves** (plus de 100 tables portant `eleve_id`) reste à faire.
- DOS-06 ★ / DOS-15 ★ : la ré-authentification des actions sensibles n'est pas branchée (droit réservé à la direction + confirmation dans l'écran seulement).
- Révocation des sessions du responsable (§4.3 `lien.suspendu` → D15) non faite : inutile pour l'accès (relu à chaque requête), à reconsidérer si des données sont mises en cache côté client.

**Reste à faire dans S2-1**
1. DOS-10 (le parent met à jour ses coordonnées, proposition « aussi chez B »), DOS-16 (séparer les fiches d'un foyer en un geste), DOS-09 invitation à partir d'une personne sans compte (D-014 second parent), DOS-14 passage automatique à la majorité et règle d'accès des parents (**Q10 non tranchée** : accès maintenu par défaut).
2. Écrans Leptos : recherche de personnes et fusion/défusion (API prête), modification d'un lien (dates, droits) et des coordonnées, recherche d'élève (accès à la fiche hors liste de classe), fiche côté portail famille (lien depuis « Absences »).
3. `ent-types::evenements` pour les nouveaux événements ; consommateurs éventuels (notification à l'établissement de `personne.coordonnees_modifiees`).
4. Migrer les écrans de l'ancienne interface qui affichent les liens (`list_liens_dossier`, comptes seulement) vers les personnes.

---

### 6.3 S2-2 — réalisé (2026-11-05, branche `ent-socle2-referentiel`)

**Niveau : implémenté + testé en local** (Postgres jetable). Non validé pilote, non fusionné. La branche contient la fusion de `ent-socle2-outbox` (S2-0) ; les faits du référentiel publient via `tx.publier` (même transaction que l'écriture). Écrans Leptos : **faits** dans la branche `ent-socle2-ecrans-ref-vs` (voir « Écrans réalisés » ci-dessous).

**Livré [F]**

| Élément | Où | Détail |
|---|---|---|
| Structure | `migrations/110_referentiel_structure.sql` | `ent_site`, `ent_niveau` (mnémoniques 6EME…TERM, cycle), `ent_mef` (code MEF 11 car., MEFSTAT11, formation/diplôme BCN, source SIECLE/BCN), `ent_periode`, `ent_jour_calendrier`, `ent_grille_horaire` (M1…M4 / S1…S4), `ent_edt_seance`, `ent_classe.{site_id,niveau_id,mef_id,archivee_le}`, `ent_edt_creneau.service_id` (EDT-03), cycle de vie de l'année (`statut` préparation/courante/clôturée, dérivé par trigger) |
| `classe_id` | `111_classe_id.sql` | colonne + trigger (double écriture par la base) dans les **9 tables** : `ent_eleve`, `ent_eleve_periode` (kind classe), `ent_eleve_scolarite_annee`, `ent_appel`, `ent_edt_creneau`, `ent_devoir`, `ent_cahier_texte`, `ent_conseil_classe`, `ent_annonce` (`cible_classe`). Texte seul → id résolu (classe/année créées au besoin) ; id seul → texte aligné ; classe d'un autre établissement refusée |
| Rattachement | `112_classe_id_rattachement.sql` | `ent_rattacher_classes()` idempotente (n'agit que si `classe_id` est nul, ne modifie aucun texte, année déduite de la date de la ligne) ; rapatriement de `ent_note_periode` → `ent_periode` (`periode_id`, trigger sur insertion) |
| Droits | `ent-auth/src/actions.rs` | actions `referentiel.lire` / `referentiel.gerer` (portée établissement), modèles par défaut équivalents à l'existant ; toutes les routes passent par le moteur (`droits_repo`) |
| Domaine | `ent-db/src/referentiel_{structure,annee,seances,classe_id}.rs`, `ref_commun.rs` | sites, niveaux (nomenclature nationale), import MEF idempotent avec rejets, périodes (couverture sans trou ni chevauchement, REF-04), calendrier unifié (jours fermés + `ent_periode_vacances` de `vacances_repo`), grille, désignation du PP (REF-13), archivage de classe, EDT-03, séances (EDT-06) |
| Année / bascule | `referentiel_annee.rs` | créer (non chevauchante, REF-01), préparer l'année suivante + reconduction idempotente des structures (classes, groupes, périodes — continuité conservée en année bissextile —, services sans PP), affectation en lot avec aperçu et montée (REF-11), ouvrir (REF-02 : l'ancienne courante est clôturée, la classe courante des élèves bascule), clôturer (REF-03 : bloquée si une période de notes est ouverte, sauf `forcer` tracé). Année clôturée = lecture seule |
| Séances | `referentiel_seances.rs` | générées depuis créneaux × grille × A/B × exceptions × calendrier ; vacances/fermetures exclues ; régénération idempotente ; séances saisies à la main jamais écrasées ; créneaux A/B sans calendrier ignorés et comptés |
| API | `ent-api/src/referentiel_s2.rs` | ~30 routes sous `/referentiel/*` |
| Événements | `referentiel.*` (agrégat `Referentiel`), `edt.seances_generees`, `edt.creneau_modifie` | publiés en transaction |

**Tests** (`ent-db/src/tests_referentiel_s2.rs` + unitaires, feature `labo`) : rattachement idempotent avec comparaison avant/après sur données de labo (9 tables) ; cohérence texte ↔ `classe_id` ; séances (vacances, jour fermé, exception, séance manuelle, idempotence) ; bascule d'année complète ; périodes de notes rapatriées ; isolation entre établissements, droits (profil de base et rôle paramétré D-017), import MEF.

**Écarts / limites [signalés]**
- `docs` annonçait « 9 tables texte » : confirmé (liste ci-dessus, `ent_eleve_periode` incluse).
- Les anciennes fonctions `referentiel_*` gardent leur garde par rôle ; l'API passe un marqueur après autorisation par le moteur (passerelle R2, `ROLE_AUTORISE_MOTEUR_*`).
- Les anciens chemins d'écriture (affectation unitaire, création de classe/groupe/service, amorçage) ne publient pas encore d'événement : à migrer vers `tx.publier`.
- Réponses des nouvelles routes = types d'`ent-db` sérialisés ; à promouvoir en DTO `ent-types` avec les écrans.
- Suppression d'une séance générée : à transformer en annulation dès que l'appel s'y rattachera (S2-4).
- La reconduction recopie les codes de classe tels quels (pas de changement de niveau automatique) : la montée se fait par correspondances explicites.
- Le trigger `classe_id` résout l'année d'une ligne sans date (`ent_eleve`, `ent_edt_creneau`) sur la date du jour.

**Plan de coupure de la lecture du texte `classe` (Q14)**
1. Mesurer : `GET /referentiel/classe-id/comparaison` doit rester à zéro (sans `classe_id`, incohérence) en pilote.
2. Basculer les lectures par module (appel, EDT, devoirs, cahier, conseil, annonces, droits `enseigne_a`) vers `classe_id`, un lot à la fois, avec test de non-régression.
3. Un lot de sécurité vérifie qu'aucune lecture métier ne dépend plus du texte (grep des requêtes).
4. Rendre `classe_id` obligatoire (NOT NULL) puis retirer les colonnes texte et les triggers dans une migration dédiée.

**Écrans réalisés (Leptos, `/web/referentiel/*`, 2026-11-05, branche `ent-socle2-ecrans-ref-vs`)** : années et bascule (étapes, préparation avec reconduction, montée avec aperçu serveur puis confirmation, ouverture et clôture confirmées, « forcer » tracé après refus), sites / niveaux / MEF (import collé), classes (rattachement, archivage, professeur principal) et groupes / ateliers (création, responsables), périodes (couverture contrôlée avant envoi) et calendrier (vacances, jours fermés), grille horaire (grille type, chevauchements), génération et liste des séances. Navigation selon les droits effectifs du moteur (`/roles/mes-droits`). Détail : `docs/FRONTEND_CIBLE.md` §10.

**Écarts / reste à faire (écrans)** : route ajoutée `GET /referentiel/groupes-admin` (liste des groupes et ateliers avec responsables) ; la montée propose des classes d'arrivée par le code de classe (« 6A » → « 5A ») à vérifier, l'aperçu et l'application sont ceux du serveur ; pas d'aperçu serveur pour l'ouverture d'année (texte d'effets uniquement) ; pas d'affectation en lot hors montée ; pas de filtre par professeur sur les séances ; périodes par niveau (`niveau_id`) non éditées ; séances d'une classe archivée non traitées ; comparaison / rattachement `classe_id` non exposés à l'écran ; accessibilité vérifiée par tests de rendu seulement (axe-core, clavier, lecteur d'écran, 375 px au navigateur : non faits).

---

### 6.4 S2-4 — réalisé (2026-11-05, branche `ent-socle2-vie-scolaire`)

**Niveau : implémenté + testé en local** (PostgreSQL jetable). Non validé pilote, non fusionné. Construit sur `ent-socle2-outbox` (fusionnée dans la branche). `origin/ent-socle2-referentiel` **n'existait pas** au moment du lot : la « séance » est donc provisoire (voir ci-dessous) et à rebrancher sur les séances datées de S2-2.

**Règles réglementaires : non vérifiées.** Les textes officiels (Code de l'éducation L131-8, R511-12 et suivants, circulaire sur l'assiduité) n'ont pas pu être consultés depuis l'environnement du lot (domaines Légifrance et education.gouv.fr non résolus). Tous les seuils, délais et durées ci-dessous sont des **hypothèses de travail [H] paramétrables**, à vérifier avant pilote (D-019).

**Conception livrée [F]**

| Élément | Où | Détail |
|---|---|---|
| Absences à la séance (D-020) | migration `130_vs_absence_seance.sql`, `ent-db/src/assiduite.rs`, `vie_scolaire.rs` | `ent_absence` reste l'unité de justification ; `ent_absence_seance` détaille les séances manquées (horaires figés, source, « prévenue »). `clore_appel` écrit l'absence **et** sa ligne de séance dans la même transaction que l'événement. |
| Demi-journées dérivées | `assiduite::deriver_demi_journees` (pure) | Jamais saisies. Journée coupée à `heure_coupure` (défaut 12:00) ; une séance à cheval est ventilée à la minute ; union des intervalles (pas de double compte) ; une demi-journée « compte » à `seuil_minutes` (défaut 30) ; « non justifiée » quand les minutes non justifiées atteignent le seuil ; absence annulée (remise présent) ignorée. Règles par établissement : `ent_vs_regle_assiduite`. |
| Reprise de l'existant | fin de la migration 130 + repli de lecture | Chaque absence à la journée existante devient deux demi-journées par jour ouvré (lundi à vendredi, plafond 365 jours). Idempotente. Les absences créées plus tard par l'ancien chemin (sans ligne de séance) sont lues par un repli équivalent : pas de double compte. |
| Séance provisoire | `PgEnt::seances_du_jour`, colonne `seance_source` | Créneau EDT + jour, filtré semaine A/B et annulations, remplaçant substitué. **Seul point à remplacer** par les séances datées de S2-2. |
| Déclarations (VS-06, VS-07) | migration 131, `declarer_absence_accueil` / `_famille` | Accueil : action `absences.declarer`, identité de l'appelant obligatoire. Famille : responsable avec `peut_justifier_absence`, ou élève **majeur** ; identité tirée du compte. Période avec heures facultatives, avant ou après les faits ; motif contrôlé ; pièce = document de l'élève vérifié. Annulation par le déclarant ou le personnel habilité. |
| Rapprochement (VS-08, A1) | `rapprocher_seance_tx`, `rapprocher_declaration_tx` | Dans les deux sens, dans la transaction : une déclaration active qui **couvre entièrement** la séance (bornes incluses) marque l'absence « prévenue ». `GET /appels/{id}/attendus-absents` pré-remplit la prévision à l'appel. La notification « absence constatée » n'est plus envoyée à la famille pour une absence prévenue. |
| Vérification de l'appel (VS-04, A2) | migration 132, `verifier_appels_for_actor`, `relancer_appel` | Calculée à la demande : séance sans appel à H + délai (défaut 15 min, paramétrable), appel ouvert après la fin de séance, incohérences (absent puis présent ; présent malgré une absence annoncée). Relance tracée + événement `vs.appel_relance` + notification de l'enseignant (consommateur `d5.relance_appel`, destinataire relu). Heure locale : `heure_paris`. |
| Punitions ≠ sanctions (VS-15) | migration 133, `discipline_vs.rs` | Tables et droits distincts (`punitions.donner` : professeur sur ses élèves, vie scolaire sur l'établissement). Aucune ligne dans `ent_sanction`. Types usuels amorcés [H]. |
| Sanctions : catégories et effacement (VS-18) | migration 134 | Catégorie de l'échelle [H] sur le type de sanction ; règles d'effacement par catégorie (fin d'année scolaire, N mois, jamais) ; effacement idempotent qui vide le texte libre, date l'opération, conserve la trace d'existence ; la liste du dossier masque les sanctions effacées. Défauts [H] : avertissement/blâme/mesure de responsabilisation à la fin de l'année scolaire, exclusions temporaires 12 mois, exclusion définitive jamais. |
| Procédure disciplinaire (VS-17) | migration 134 | Deux instances : chef d'établissement (observations de l'élève obligatoires avant décision) et conseil de discipline (convocation avec délai minimal, défaut 8 jours [H] ; décision à partir du jour du conseil). Sanction liée = même élève. Classement sans suite. |
| Signalement d'assiduité (VS-21, A5) | migration 134, `proposer_signalements` | Le CPE propose (élèves au seuil du mois, défaut 4 demi-journées non justifiées [H]), la direction valide/refuse puis émet. Dossier JSON détaillé. |
| Droits | `ent-auth/src/actions.rs` | Cinq actions : `absences.declarer`, `appel.verifier`, `punitions.donner`, `discipline.gerer`, `assiduite.signaler`. Modèles par défaut calés sur les profils déjà habilités ; test d'un rôle paramétré (prof + `absences.declarer`). |
| Événements | `ent-db/src/vie_scolaire.rs` (`types_vs`) | `vs.absence_declaree`, `vs.declaration_annulee`, `vs.absence_rapprochee`, `vs.appel_relance`, `vs.punition_donnee`, `vs.procedure_engagee`, `vs.procedure_decidee`, `vs.conseil_discipline_decide`, `vs.sanction_effacee`, `vs.signalement_assiduite_emis` : tous publiés dans la transaction du fait (outbox S2-0). |
| API | `ent-api/src/vie_scolaire.rs` | 27 chemins (29 méthodes HTTP). Les droits sont ceux d'`ent-db`. |
| Écrans Leptos | `/web/journee`, dialogue de déclaration ; `/web/discipline` ; `/web/signalements` | Journée vie scolaire/accueil (absents prévenus ou non, **demi-journées dérivées par le serveur**, retards, absences annoncées, appels à vérifier + relance, incohérences, saisie d'une absence annoncée — ouverte aux rôles paramétrés `absences.declarer` / `appel.verifier`) ; famille : « Déclarer une absence » dans `/web/absences` ; **punitions ≠ sanctions** (deux sections), procédure disciplinaire (observations, convocation, décision, classement), effacement des sanctions ; signalements d'assiduité (décompte, proposer, valider / refuser / émettre). Branche `ent-socle2-ecrans-ref-vs`. |

**Écarts par rapport au §3 / aux actions VS [signalés]**
- Agrégats d'événements **réutilisés** (`Absence`, `Appel`, `Sanction`) au lieu de variantes dédiées, pour ne pas toucher à l'enum partagée entre lots en parallèle ; types d'événements déclarés dans `ent-db` et non dans `ent_events::types`.
- VS-08 n'émet pas `eleve.absence_justif_soumis` (une absence prévenue n'est pas un justificatif soumis) mais `vs.absence_rapprochee`.
- Le rapprochement n'accepte qu'une **couverture entière** de la séance ; une déclaration partielle (ex. « jusqu'à 9 h 30 » pour une séance 9 h–10 h) ne rend pas l'absence prévenue (visible comme incohérence potentielle côté vérification).
- La prononciation d'une sanction (VS-16) garde son contrôle historique par profil (`can_saisir_sanctions`) : non migrée vers le moteur de droits (reste de R2).
- Le menu « Journée » est affiché selon le profil (même ensemble que « trancher un justificatif ») et non selon les droits effectifs du moteur : un rôle paramétré qui reçoit `appel.verifier` sans ce profil doit saisir l'adresse (le serveur reste l'autorité).

**Non livré / reste à faire**
- **Écrans : reste à faire** — édition des règles d'assiduité et des règles / catégories d'effacement (routes prêtes) ; prononcer une sanction depuis l'écran discipline (route historique `POST /eleves/{id}/sanctions`, liste en lecture seule seulement) ; rattacher la sanction à la décision (`sanction_id`) ; affichage du pré-remplissage A1 dans la feuille d'appel (route prête ; feuille d'appel inchangée) ; liste et annulation des déclarations côté famille (route prête) ; dossier de signalement détaillé (JSON `dossier` non affiché). Livrés dans la branche d'écrans : punitions, sanctions (lecture), procédure disciplinaire, effacement, signalements, demi-journées de la journée.
- VS-03 (consulter un appel sans l'ouvrir, GET), VS-05 (corriger un appel clos), `appel.non_fait_detecte` / tâche périodique de détection et relance push automatique : la vérification est calculée à la demande, la relance est manuelle.
- Dossier de signalement en PDF (VS-21) ; lettre de convocation (D9).
- Jours fériés et vacances non exclus du repli de reprise (lundi à vendredi seulement) ; samedi matin non géré.
- Compteurs historiques en jours (`apres_absence_ecrite`, signal de seuil annuel) inchangés : ils comptent encore des absences, pas des demi-journées. Le seuil mensuel de signalement utilise, lui, les demi-journées dérivées.
- Séance du référentiel (S2-2) : à brancher sur `seances_du_jour` ; migrer `seance_ref` vers l'identifiant de la séance datée.
- Appli mobile : non commencée dans ce lot.
- Vérification des textes officiels (voir ci-dessus).

**Tests** : voir la ligne « Tests S2-4 » de `docs/PROJECT_STATUS.md` pour les résultats exacts.

### 6.5 S2-11 — santé et accompagnement : réalisé (2026-11-06, branche `ent-socle2-sante`)

**Niveau : implémenté + testé en local** (PostgreSQL 16 jetable). Non validé pilote, non fusionné. **Q7 (hébergement certifié HDS) reste à faire trancher par le DPO** : d'ici là, minimisation stricte et chiffrement (voir « Point HDS à soumettre » ci-dessous). Construit sur `ent-integration-lot4` (S2-0 outbox, S2-1 personnes et liens, RLS pilote, chiffrement D3). Ancienne UI (`ui.rs`) non lue ni étendue.

**Conception livrée [F]**

| Élément | Où | Détail |
|---|---|---|
| PAI complet (SAN-05) | migration `210`, `ent-db/src/sante_acc.rs` | Version incrémentée à chaque écriture ; **signatures par partie et par version** (famille, médecin, chef d'établissement, infirmier, collectivité) : toute modification invalide les signatures (`ent_pai_signature`, unique par PAI × version × partie). Quatre parties requises (famille, médecin, chef, infirmier) ; « complet » quand elles ont toutes signé la version courante. Révision annuelle (`date_revision`, un an après le début par défaut, liste « PAI à réviser » à 30 jours), trousse d'urgence (présence, lieu de rangement non médical, péremption). La famille signe depuis son compte (lien actif) ; médecin et collectivité sont consignés par l'infirmerie (nom obligatoire) ; seule la direction signe pour le chef d'établissement. Ancienne route de signature famille conservée (délègue au nouveau parcours). |
| Contraintes publiées (SAN-06) | migration `211`, `sante_acc.rs` | **Catalogue fermé** (17 codes : « repas sans arachide », « trousse d'urgence », « ne pas courir »…) ; la base ne reçoit **aucun texte libre** ; audiences restauration / enseignants / sorties, chaque code n'admettant que ses audiences. Publication manuelle par l'infirmerie ; la trousse du PAI se publie et se retire d'elle-même ; un PAI inactif retire ce qu'il a dérivé. Lecture : personnel de santé (tout, avec la source), restauration (`repas.gerer/scanner`), sorties (`sorties.gerer/controler`), enseignants (`contraintes.lire`, portée « mes classes »). La **source** (PAI) n'est jamais communiquée hors santé. |
| Passages (SAN-02/03/04) | migration `213`, `sante_acc.rs` | Heures, motif, soins (chiffrés), issue (retour en classe, repos, famille appelée, renvoi au domicile, secours). **Information de la famille** (SAN-03) : règle par défaut [H] — accident, famille appelée, renvoi au domicile, secours → automatique ; sinon à la demande ; notification interne **sans motif ni soins** (`infirmerie_passage`), à tous les responsables au lien actif, idempotente. **Renvoi au domicile** (SAN-04) : exige une personne habilitée (responsable au lien actif ou contact d'urgence validé « peut récupérer »), consignée avec son nom. |
| Fiche d'urgence | `sante_acc.rs`, `/web/infirmerie/eleves/{id}/urgence` | Imprimable (styles `@media print`). Niveau **complet** (`sante.lire`) : protocole du PAI, médecin, téléphone d'urgence. Niveau **minimal** (`sante.urgence_lire`, professeur sur ses élèves, CPE, direction…) : responsables, contacts, contraintes publiées, trousse, consigne générale — ni diagnostic, ni traitement, ni protocole. Chaque consultation est journalisée. |
| Registre des protocoles (SAN-08) | migration `214` | Textes génériques de l'établissement (jamais liés à un élève), versionnés, titre unique. |
| Accompagnement (ACC-01→05) | migration `212`, `ent-db/src/accompagnement.rs` | `ent_eleve_dispositif` étendu (PPRE, ULIS, statut, révision) ; **aménagements** du catalogue fermé (14 codes : tiers-temps, police adaptée…) avec portée cours / examen ; AESH (nom, quotité, dates) ; réunions d'équipe éducative / de suivi (compte rendu **chiffré**, réservé à la gestion). Clôture d'un dispositif : aménagements liés retirés. Un PAP peut être ouvert par l'infirmerie (`sante.saisir`), les autres types par `accompagnement.gerer`. |
| Droits | `ent-auth/src/actions.rs` | 4 actions : `accompagnement.gerer` (admin, direction), `amenagements.lire` (professeur : mes classes ; admin, direction, administratif, CPE, infirmerie, psy), `contraintes.lire` (professeur : mes classes ; admin, direction, administratif, CPE, infirmerie), `sante.urgence_lire` (professeur : mes classes ; admin, direction, administratif, CPE, infirmerie) → **51 actions**. |
| Chiffrement et RLS | `champs_chiffres.rs`, migration `215` | **Étendus** : `ent_pai` (motif, mesures, protocole, note interne), `ent_infirmerie_passage` (note, soins), `ent_equipe_reunion` (compte rendu) en plus de la fiche santé et du dossier psy ; `TenantTx` partout ; RLS forcée sur les 6 nouvelles tables. |
| Événements | `types_sante`, `types_acc` | Tous publiés dans la transaction du fait (outbox S2-0). Charges utiles : identifiants, états, dates techniques — **jamais** de motif, de soins, de code de contrainte ni de nom d'aménagement. |
| API | `ent-api/src/sante.rs` | 29 méthodes sur 24 chemins. Droits décidés par `ent-db`. |
| Écrans Leptos | `/web/infirmerie` (+ `eleves/{id}`, `eleves/{id}/urgence`), `/web/accompagnement` (+ `eleves/{id}`), `/web/amenagements`, `/web/sante-enfant` | Infirmerie : tableau du jour, saisie et clôture d'un passage (renvoi contrôlé), PAI à réviser, fiche santé (PAI, signatures, contraintes). Accompagnement : dispositifs, aménagements, AESH, réunions. Professeur : ses élèves aménagés et les contraintes de son public, sans diagnostic. Famille : PAI à lire et signer, passages visibles (sans note), aménagements. |

**Minimisation : fuites corrigées dans l'existant [F]** (les charges utiles d'événements portaient des données de santé) : `infirmerie.fiche_sante` (allergies, traitements), `infirmerie.pai` et `infirmerie.pai_signe` (motif du PAI = diagnostic), `infirmerie.pai_piece` (nom du fichier), `infirmerie.passage` (motif), `infirmerie.fiche_sante_pdf` (drapeau PAI), `psy.dossier` (axes de suivi). Ces événements ne portent plus que des identifiants, états et dates.

**Écarts par rapport au §3 [signalés]**
- **PP (professeur principal)** : pas de notion de PP dans le moteur de droits ; ACC-01/02 « DIR, PP » → direction par défaut, et un établissement peut attribuer `accompagnement.gerer` (portée « mes classes ») à un rôle de professeur principal. Pas de rôle « médecin ».
- **SAN-04** : le renvoi au domicile est consigné (personne habilitée, événement) mais **l'absence n'est pas créée** ni la sortie VS tracée (couplage avec DOS-08 / sorties non fait).
- **SAN-03** : la règle d'information de la famille est une valeur par défaut [H], non paramétrable par l'établissement.
- **ACC-04** : planification et compte rendu ; les **convocations** (notification aux participants) ne sont pas envoyées. ACC-03 : AESH saisi par son nom (pas de compte AESH).
- **Fiche d'urgence de séjour (SEJ-08)** : la route et l'écran existent (niveau minimal) ; l'accès temporaire limité à la durée du séjour, la copie hors ligne et l'effacement après retour relèvent de S2-12.
- **Restauration** : elle lit la liste des contraintes (`GET /sante/contraintes?audience=restauration`) ; aucun consommateur d'événement n'alerte la cantine (le domaine D11 n'en a pas encore).
- Aménagements dans les **convocations d'examen** (EXA) et placements : S2-13.
- Libellé libre d'un dispositif (120 caractères) : non chiffré et visible du suivi ; la consigne « pas de diagnostic » est une règle d'usage, non vérifiable par le système.
- Pièces du PAI (ordonnances) : stockées dans le magasin d'objets, **non chiffrées au repos** par ce lot.

**Point HDS à soumettre au DPO (Q7) — questions, pas des faits**
1. Le PAI, les notes de soins, la fiche santé et les notes du psychologue relèvent-ils de l'hébergement de données de santé certifié (HDS), pour un éditeur qui héberge pour le compte d'établissements ? Le cas échéant, quel hébergeur certifié [H : à vérifier pour Scaleway] pour la base et le magasin d'objets ?
2. Quelles données de santé sont réellement nécessaires à l'ENT ? Proposition [R] : ne garder dans l'ENT que la contrainte publiée et la fiche d'urgence minimale, le reste (motif, protocole, ordonnances) restant au dossier de l'infirmerie.
3. Durées de conservation et effacement (passages, PAI échus, comptes rendus d'équipe) ; droit d'accès de la famille aux passages ; AIPD.
4. Habilitations : qui, hors infirmerie, peut recevoir la fiche d'urgence minimale (professeur, encadrant de séjour) et pour quelle durée.
5. Chiffrement actuel : champs applicatifs AES-256-GCM (clé `ENT_SECRETS_KEY` dérivée) ; les pièces et sauvegardes ne le sont pas dans ce lot.

**Tests** : voir la ligne « Tests S2-11 » de `docs/PROJECT_STATUS.md` pour les résultats exacts.

## 7. Décisions à soumettre à Florian (questions fermées)

| # | Question | Options | Recommandation |
|---|---|---|---|
| **Q1** | **Cible du socle 2 : privé sous contrat d'abord, ou public et privé en même temps ?** | (a) privé d'abord, public dans S2-15 ; (b) les deux dès maintenant | **(a)** : le public exige ÉduConnect, GAR, SDET, interface Op@le (lourd, à vérifier) ; le privé valorise directement facturation et inscriptions déjà fortes dans le code |
| **Q2** | **Contrat de la toile : tout effet inter-domaines passe obligatoirement par un événement publié en transaction (outbox), et l'ancienne `publish` hors transaction est retirée progressivement ?** | oui / non | **oui** : sans cela, aucune garantie (perte d'événements constatée en risque) ; lot S2-0 en premier |
| **Q3** | **Rôles : ajouter Accueil, Surveillant/AED, Restauration, Documentaliste (et des délégations datées) ?** | oui / non (tout par capacités) | **oui**, adossés au modèle de capacités B6 ; les remplacements et organisateurs passent par délégation datée, pas par rôle |
| Q4 | Personne distincte du compte (`ent_personne`) pour les responsables (V8) ? | oui / non | **oui** (indispensable à D-014) |
| Q5 | Paiement en ligne par carte dans le socle 2 ? | oui (prestataire européen à choisir) / non (SEPA seul) | **oui, en fin de S2-9**, prestataire soumis à votre validation (aucun par défaut) |
| Q6 | Granularité des absences : passer à la séance avec dérivation en demi-journées ? | oui / non | **oui** (additif, nécessaire au décompte réglementaire) |
| Q7 | Données de santé : hébergement certifié HDS exigé ? | à faire trancher par le DPO | **consulter le DPO avant S2-11** ; d'ici là, minimisation stricte et chiffrement (socle 1 D3) |
| Q8 | Premier degré (écoles des ensembles scolaires, ONDE) dans le socle 2 ? | oui / non | **non**, sauf client pilote qui l'exige |
| Q9 | Journal d'accès visible par la famille (A11) : par service (« vie scolaire ») ou nominatif ? | service / nominatif / non | **par service** (transparence sans exposer les personnels) |
| Q10 | Élève majeur : les parents gardent-ils l'accès par défaut ? | oui / non / au choix de l'élève | **au choix de l'élève majeur, accès maintenu par défaut pour le payeur aux factures** (à valider DPO) |
| Q11 | Un impayé peut-il bloquer la réinscription ? | oui (paramètre) / non | **paramètre par établissement, désactivé par défaut**, toujours avec décision humaine |
| Q12 | Sanctions : échelle réglementaire imposée, ou paramétrable ? | imposée / paramétrable | **référentiel réglementaire par défaut, extensible pour le privé**, effacements automatiques |
| Q13 | Ensemble scolaire (plusieurs établissements, une facturation fratrie) ? | oui / non | **non dans le socle 2** : contredit l'absence de visibilité entre établissements (D-013) ; à étudier comme « groupe » explicite et consenti plus tard |
| Q14 | Date de retrait des colonnes texte `classe` (après S2-2) ? | fin S2-2 / fin socle 2 | **fin S2-2 + un lot de sécurité** (lecture texte coupée, colonnes supprimées ensuite) |
| Q15 | Ordre des lots S2-0 → S2-14 ci-dessus ? | valider / réordonner | **valider**, avec parallélisation après S2-2 |

---

## 8. Faits, hypothèses, questions (récapitulatif)

**Faits [F]** : voir §0 ; toutes les mentions « existant » des §2–4 renvoient à des tables (`ent-rust/migrations/`), modules (`domain-eleve/src/`), repos (`ent-db/src/*_repo.rs`) ou constantes (`ent-events/src/lib.rs`) relevés par `grep`. Le comportement réel des routes n'a pas été exécuté.

**Hypothèses [H] principales** : couverture fonctionnelle des concurrents (§1.1, École Futée très incertaine) ; règles réglementaires citées de mémoire (assiduité et seuil de 4 demi-journées, échelle et effacement des sanctions, formats LSU/LSL/Affelnet, ÉduConnect/GAR/SDET, Op@le, pré-notification SEPA, HDS, RGAA, vote du CA pour les voyages) : **à vérifier sur les textes officiels avant chaque lot** ; volumes du §6.

**Contradictions et points à signaler** :
- `CARTOGRAPHIE_ENT.md` indique que le Brain réagit à `eleve.created` : non retrouvé dans `ent-brain/src/react.rs` (seul `appel.closed`).
- `CARTOGRAPHIE_ENT.md` §0 « aucune table `ent_etablissement` » est dépassé (migration 096).
- `ent-metier-checklist.md` donne plusieurs éléments « déjà » (ex. LSU, Better-Auth pour les rôles) qui relèvent de l'ancienne génération ou d'exports CSV : ne pas le lire comme un état du code Rust actuel.
- Migration `094` absente de la séquence : à expliquer (branche socle 1 en cours ?).

**Questions ouvertes hors §7** : périmètre exact du pilote (déjà posé dans `PROJECT_STATUS.md`) ; choix du fournisseur d'e-mail ; visio (service externe à décider, non par défaut).
