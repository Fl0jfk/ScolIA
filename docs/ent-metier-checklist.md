# Checklist métier — ENT complet (référence Charlemagne)

> **But :** définir ce dont on a besoin pour un **ENT à nous** (Rust + PostgreSQL + API Rust — **partout**).  
> **Vision produit / cerveau :** voir [`ent-vision-rust.md`](./ent-vision-rust.md) (socle élève au centre, Brain, IA branchée, pas de touch prod).  
> **Pas le but :** cloner Charlemagne, Charlemagne Host, import 1:1, recopier l’UI WinDev.  
> Charlemagne sert de **référentiel de couverture métier** : « qu’est-ce qu’un ENT scolaire privé français doit savoir faire ? »

---

## 1. Sources d’évidence (REA + clichés)

| Source | Rôle |
|---|---|
| REA `open_binary` / inventaire ZIP | Confirme archives HFSQL lisibles, empreintes SHA-256 |
| Clichés 12/10/2026 `H:\Alcuin\Sauvegardes\LA PROVIDENCE\` | Admin / Notes / Vie scolaire — **même catalogue 659 tables** |
| Registre `HKCU\SOFTWARE\Aplim\*\Fen_*` | Capacités UI par module (Administratif, VS, Notes…) |
| Clients `C:\Program Files (x86)\alcuin\*.exe` | Découpage produit (pas le modèle de données) |
| Catalogue projet | `docs/charlemagne-reference/` (listes + JSON) |

**Constats clés**

- Un seul pot HyperFile partagé : sauvegarder « Notes » ou « Vie scolaire » ne change pas la liste des tables.
- Préfixes métier observés : `COM_`, `VS_`, `NO_`, `LSU_`, `INS_`, `FAC_`, `ADM_`, `PA_`, `TAB_`, `REC_`, `ENT2_`, `INF_`, …
- Compta OGEC / Paie = sauvegardes **séparées** sur `H:\Alcuin` (hors cœur ENT familles).

---

## 2. Principes de reconstruction

1. **Concepts d’abord** (élève, foyer, absence, devoir…) — noms Postgres/Rust **à nous**.
2. **Prioriser le geste utilisateur** (intranet personnel + apps familles/élèves/profs).
3. **Tranches** : socle → VS → notes → inscriptions → factu → le reste.
4. **Keep / drop explicite** : tout ce qui est dans Charlemagne n’a pas vocation à être dans l’ENT.
5. Multi-tenant : toute table métier = `etablissement_id`.

---

## 3. Checklist métier ENT (par domaine)

Légende priorité :

| Code | Signification |
|---|---|
| **P0** | Indispensable pour un ENT « vivant » |
| **P1** | Fortement attendu (année scolaire complète) |
| **P2** | Utile / différenciant |
| **P3** | Optionnel / métier annexe / OGEC |
| **DROP** | Hors scope ENT (référence seulement) |

Statut ScolIA : **déjà** = partiellement ou largement couvert dans le repo actuel ; **à faire** = à concevoir/renforcer côté ENT cible (Rust).

### 3.1 Établissement & année scolaire — P0

| Besoin métier | Réf. Charlemagne (indicatif) | Priorité | ScolIA |
|---|---|---|---|
| Établissement (multi-tenant) | `COM_ETABLISSEMENT` | P0 | déjà |
| Année scolaire / calendrier | `VS_LIBELLE_PERIODES`, calendriers | P0 | déjà |
| Sites / locaux (si multi-sites) | `TAB_SITE`, `VS_LOCALISATION` | P2 | déjà (UI `/app/etab` salles + site ; sorties multi-site EDT) |
| Paramètres établissement | `VS_PARAMETRES_ETAB`, prefs `COM_*` | P1 | déjà (UI `/app/etab` params) |
| Utilisateurs, rôles, profils | `ADM_PROFIL`, `CC_PROFIL_*`, `VS_PROFIL` | P0 | déjà (Better-Auth) |
| Authentification forte (2FA) | écrans sécurité | P0 | déjà (Rust : **passkeys WebAuthn uniquement**, pas de TOTP — `ent_passkey_credential` + 2ᵉ facteur non contournable, connexion sans mot de passe, récupération Direction/Admin, UI `/app/securite`) |
| SSO établissement (délégation d'identité) | fédération | P1 | déjà (Rust : **OIDC** par établissement — découverte validée à la configuration, PKCE S256 + state + nonce, `id_token` RS256 vérifié (JWKS), state à usage unique, rattachement par `sub`/e-mail, **aucun provisioning implicite**, UI bouton SSO + `/app/securite`) |
| Supervision & sauvegardes (production) | exploitation | P0 | déjà (Rust : `/metrics` Prometheus derrière jeton, `/health` + `/ready` avec `migration_tag`, journaux JSON optionnels, **inventaire d'intégrité** qui détecte la dérive donc prouve une restauration, scripts `pg_dump`/restauration/exercice PRA gardés loopback — `docs/ent-rust-observabilite-pra.md`) |

### 3.2 Référentiels élèves & familles — P0

| Besoin métier | Réf. Charlemagne | Priorité | ScolIA |
|---|---|---|---|
| Élève (identité, INE, scolarité) | `COM_ELEVES` | P0 | déjà |
| Classes / niveaux / divisions | `COM_CLASSES`, `COM_NIVEAU` | P0 | déjà |
| Foyer / famille | `COM_FOYER` | P0 | déjà |
| Responsables + lien élève | `COM_RESPONSABLES`, `COM_LIENER` | P0 | déjà |
| Régime, options, MEF / formation | `FAC_REGIME`, `COM_OPTIONS_EL`, `COM_MEF_*` | P0–P1 | déjà (régime/options/continuum + MEF sur filières) |
| Groupes pédagogiques | `COM_GROUPE*` | P1 | déjà (p1) |
| Nomenclatures (civilité, CSP, villes…) | `TAB_*` | P1 | déjà (`ref_nomenclature`) |
| Infos complémentaires paramétrables | `ADM_IC*` | P2 | déjà (UI `/app/etab` + lecture famille natives) |
| Photos / trombi | `ADM_TROMBIS`, photos sur partage | P2 | déjà (UI trombi `/app/eleves` + pose photo dossier + API) |
| Journal des modifications | `COM_JOURNAL_MODIF` | P1 | déjà (Rust : `ent_journal_modif` + API élève) |

**Agrégats Rust suggérés :** `Etablissement`, `AnneeScolaire`, `Eleve`, `Classe`, `Foyer`, `Responsable`, `LienFamilial`, `GroupePedagogique`, `Nomenclature`.

### 3.3 Inscriptions & dossiers — P0/P1

| Besoin métier | Réf. Charlemagne | Priorité | ScolIA |
|---|---|---|---|
| Préinscription / candidature | `INS_PREINSCRIPTION` | P0 | déjà (rdv + dossiers) |
| Filières / MEF / vœux | `INS_FILIERES`, `INS_MEF_*`, `INS_VOEUX` | P1 | déjà (UI `/app/etab` + natives vœux) |
| Pièces dossier + réception | `INS_PIECES_*`, `INS_PIECE_*` | P0 | déjà |
| Documents à signer | `INS_DOC_A_SIGNER`, `INS_SIGNATURE_DOC` | P1 | déjà (publication UI `/app/docs-signer` + signature parent natives) |
| Workflow étapes / motifs | écrans `Fen_Ins_Suivi_*`, `Fen_Choix_Motif_*` | P0 | déjà (UI `/app/inscriptions` + API suivi / file par étape) |
| Homonymes / dedup | `INS_LOGS_HOMONYMES_*` | P1 | déjà (`POST /eleves/homonymes`, score nom+prénom+naissance) |
| Passage N → N+1 | `COM_OPTIONS_PLUS1/MOINS1`, Passage | P1 | déjà (Rust : continuum + montée) |

**Agrégats :** `DossierInscription`, `PieceDossier`, `Voeu`, `EtapeWorkflow`.

### 3.4 Vie scolaire — P0

| Besoin métier | Réf. Charlemagne | Priorité | ScolIA |
|---|---|---|---|
| Appel de classe | `VS_APPEL`, `VS_APPEL_ELEVE` | P0 | déjà |
| Absences / retards élèves | `VS_ABSENCE*`, motifs, prévenues | P0 | déjà (retards = fait **canonique** `ent_retard`, persistant et distinct des absences : `POST/GET /eleves/{id}/retards`, compteur, vue famille) |
| Absences / appels profs | `VS_ABSENCE_PROF`, `VS_APPEL_PROF` | P1 | déjà (`/rh/absences` + validation ; appels élève déjà) |
| Sanctions / incidents | `VS_SANCTION*`, `VS_INCIDENT*` | P0 | déjà |
| Dispenses | `VS_DISPENSES` | P1 | déjà (Rust : EPS/piscine/médical + couverture appel) |
| Autorisations de sortie | `VS_AUTOR_SORTIE_*` | P1 | déjà (Rust : permanent + exception + badge) |
| Carnet / observations / suivi | `VS_SUIVI*`, `VS_OBSERVATION_*`, `VS_REMARQUE_*` | P0 | déjà (carnet p5) |
| Alertes CPE | `VS_ALERTE*` | P2 | déjà (UI `/app/alertes` + visible famille natives + clôture) |
| Internat (chambres, suivis) | `VS_INTERNAT_*` | P1 | déjà (UI `/app/internat` + récap famille natives) |
| Permis à points | `VS_PERMIS_*` | P2 | déjà (solde via commission / sanctions) |
| SMS / notifications familles | `VS_SMS_*`, notifs | P1 | déjà (outbox `/sms` + `/mails` + push devices) |
| Éditions / documents VS | `VS_EDITION*` | P2 | déjà (`/editions-vs` modèles + rendu placeholders) |

**Agrégats :** `Appel`, `LigneAppel`, `Absence`, `Sanction`, `Incident`, `AutorisationSortie`, `EntreeCarnet`, `ChambreInternat`, `DispenseVs`.

### 3.5 Emploi du temps — P0/P1

| Besoin métier | Réf. Charlemagne | Priorité | ScolIA |
|---|---|---|---|
| Créneaux / cours | `VS_EDT_COURS`, `VS_EDT_TYPE_*` | P0 | déjà (`edt_creneau` + `GET /eleves/{id}/edt` famille) |
| Affectation profs / salles | `VS_EDT_PROF`, `VS_EDT_SALLE`, `VS_SALLES` | P0 | déjà (`POST /edt/creneaux/{id}/affectation` + conflits + salles) |
| Événements / exceptions | `VS_EDT_EVENEMENT_*` | P1 | déjà (Rust : `edt_exception`) |
| Alternances semaines | `VS_EDT_ALTERNANCES` | P1 | déjà (Rust : calendrier A/B) |
| Contraintes (prof, salle, matière…) | `VS_CONTRAINTES_*` | P2 | déjà (`/edt/contraintes` soft ; solveur CP-SAT DROP) |
| Remplacements | `VS_REMPLACEMENT_*`, Pacte | P2 | déjà (via `edt_exception` kind remplacement) |
| Vacances / grilles horaires | `VS_HORAIRE*`, import vacances | P1 | déjà (UI `/app/etab` vacances + jour-ferme ; grilles EDT A-B) |

> Un ENT a besoin de **consulter et consommer** un EDT ; un **solveur** type Charlemagne peut rester hors scope (P2/DROP).

### 3.6 Notes, bulletins, compétences — P0/P1

| Besoin métier | Réf. Charlemagne | Priorité | ScolIA |
|---|---|---|---|
| Matières / sous-matières / services | `TAB_MATIERE`, `NO_SOUS_MATIERE`, `NO_TAB_SERVICE_*` | P0 | déjà |
| Périodes (trim/semestre) | `NO_PER_CLASSE_*` | P0 | déjà |
| Devoirs / évaluations | `NO_DEVOIR`, `NO_TYPE_DEVOIR` | P0 | déjà |
| Notes élève | `NO_NOTE` | P0 | déjà |
| Moyennes (élève / classe / rang) | `NO_MOY_*` | P0 | déjà |
| Appréciations | `NO_APPRECIATION*` | P0 | déjà (saisie + période partielle_appreciations) |
| Compétences / socle / livret | `NO_COMP*`, `NO_LIVRET*`, `LSU_*` | P1 | déjà (référentiel socle par établissement `ent_competence_item` + niveau de maîtrise saisi par `(élève, item, période)`, export LSU sur évaluations réelles, **livret PDF notifié**, écran `/app/competences` + section famille) |
| LSL / LSUN / exports EN | `NO_LSL_*`, `NO_LSUN_*` | P1 | déjà (`/exports/lsl` + `/exports/lsun` CSV ; dépôt national hors scope) |
| Mentions / grilles / simulation conseil | `NO_TABLE_MENTIONS`, `NO_GRILLE*`, `NO_SIMULATION` | P2 | déjà (Rust : mentions conseil) |
| Parcoursup / APB / Notanet | `NO_APB_*`, `NO_NOTANET_*` | P2 | déjà (`/exports/notanet` + `/exports/parcoursup` CSV ; dépôt national hors scope) |
| Conseil de classe (planif) | écrans Notes | P2 | déjà (UI `/app/conseils` + lecture famille natives) |

**Agrégats :** `Matiere`, `Periode`, `Devoir`, `Note`, `Moyenne`, `Appreciation`, `Competence`, `LivretScolaire`.

### 3.7 Communication établissement ↔ familles — P0/P1

| Besoin métier | Réf. Charlemagne | Priorité | ScolIA |
|---|---|---|---|
| Messagerie / messages prédéfinis | `COM_MESSAGES_*`, `COM_MODELS_MAIL` | P0 | déjà |
| Historique mails | `COM_HISTORIQUE_MAILS` | P1 | déjà (`GET /mails` outbox staff) |
| Documents famille (GED légère) | `COM_DOCUMENT`, pièces | P0 | déjà |
| Annonces établissement (actualités diffusées) | `COM_*` (diffusion) | P1 | déjà (Rust : `ent_annonce` + diffusion mail/push familles ciblées + UI `/app/annonces`) |
| Portail famille (absences, notes, carnet) | ÉcoleDirecte côté Aplim | P0 | `/famille` |
| Notifications push / SMS | VS SMS, notifs | P1 | déjà (devices push + outbox SMS/mail) |

### 3.8 Facturation familles — P1

| Besoin métier | Réf. Charlemagne | Priorité | ScolIA |
|---|---|---|---|
| Tarifs / articles / formules | `FAC_ARTICLE*`, `FAC_FORMULE*`, `FAC_GRILLE_*` | P1 | déjà (p1) |
| Factures foyer / échéances | `FAC_GESTION_*`, `FAC_LIGNE`, `FAC_ECHEANCE_*` | P1 | déjà (API + UI web + natives factures/quittances) |
| Encaissements / mandats / IBAN | `ADM_ENCAISSEMENT*`, `FAC_HISTO_IBAN` | P1 | déjà (SEPA + `/foyers/{id}/iban-histo`) |
| Bourses / quotients | `FAC_BOURSE_*`, `FAC_QUOTIENT` | P2 | déjà (bourses + quotient foyer `/foyers/{id}/quotient`) |
| TVA / catégories | `FAC_TAUX_*`, `FAC_CATEGORIE_TVA` | P2 | déjà (`/taux-tva` + calcul HT→TTC) |
| Lien compta OGEC | `FAC_COMPTA_*`, `FAC_TIERS_*` | P3 | DROP ENT |

### 3.9 Cantine / passages — P2

| Besoin métier | Réf. Charlemagne | Priorité | ScolIA |
|---|---|---|---|
| Passages / badge | `PA_PASSAGES`, `PA_CARTE`, `COM_BADGE` | P2 | déjà (scan + jour + hist élève `GET /eleves/{id}/repas/passages` famille/staff + facture badge) |
| Menus / formules / tarifs conso | `PA_MENU`, `PA_*TARIF*` | P2 | déjà (publication UI `/app/cantine` + lecture famille + tarifs `/tarifs-conso`) |
| Porte-monnaie | `PA_PORTE_MONNAIE*` | P2 | DROP (paiement via facturation / SEPA / badge) |
| Absences repas | `PA_ABSENCE_REPAS` | P2 | déjà (via dispenses repas) |

### 3.10 Ressources humaines (personnel) — P1 (léger ENT)

| Besoin métier | Réf. Charlemagne | Priorité | ScolIA |
|---|---|---|---|
| Fiche personnel / fonctions | `COM_PERSONNELS`, `ADM_FONCTION_*` | P1 | déjà |
| Absences RH / validation | VS prof + module RH | P1 | déjà |
| Planning RH complet | `RHPlanning` (zip Paie) | P3 | DROP |
| Paie / salaires | `Sauvegardes Paie` | DROP | DROP |
| Compta générale OGEC | `Sauvegardes Comptabilite` | DROP | DROP |

### 3.11 Pont Éducation nationale — P1

| Besoin métier | Réf. Charlemagne | Priorité | ScolIA |
|---|---|---|---|
| Import / export Siècle, STS-WEB | `REC_*`, module `EducationNationale` | P1 | déjà (nomenclatures) |
| ONDE / BE1D / 1D | `REC_BE1D`, `REC_ENVOI_ONDE` | P2 | déjà (`GET /exports/onde` CSV effectif ; envoi national hors scope) |
| LSU / LSL nationaux | `LSU_*`, exports LSL | P1 | déjà (CSV locaux `/exports/lsl` + `/exports/lsun` ; dépôt national hors scope) |
| Identifiants nationaux (INE…) | `REC_IDSIECLE`, `REC_IDPRIVE_*` | P0 | déjà (validation `normalize_ine` à la création / update) |

### 3.12 Stages / entreprises / CFA — P2 ou DROP

| Besoin métier | Réf. Charlemagne | Priorité | ScolIA |
|---|---|---|---|
| Entreprises, tuteurs, conventions | `ENT2_*` | P2 | déjà (`/entreprises` + `/conventions-stage` + lecture famille) |
| CFA / contrats / OPCO | `ENT2_CFA_*` | P3 | DROP sauf besoin |
| Taxe d’apprentissage | `ENT2_TAXE*` | DROP | DROP |

### 3.13 Infirmerie — P3

| Besoin métier | Réf. Charlemagne | Priorité | Note |
|---|---|---|---|
| Fiches santé, vaccins, visites | `INF_*` | P3 | déjà (passages, fiche, PAI, PDF + natives fiche/signature PAI) |

### 3.14 Technique / hors métier ENT — DROP

| Élément | Note |
|---|---|
| `CC_*` licences, journal technique | Socle Aplim |
| `DASH_*` | Dashboard éditeur |
| `DATALAKE_EXPORT` | Pipeline éditeur |
| UI WinDev / `Fen_*` | Inspiration flux seulement |

---

## 4. Socle ENT minimal (ordre de construction)

```
P0-1  Établissement + auth + rôles
P0-2  Élève / classe / foyer / responsables
P0-3  Année scolaire + calendrier
P0-4  Vie scolaire : appel, absences, sanctions, carnet
P0-5  EDT consultable (créneaux)
P0-6  Notes : matières, périodes, devoirs, notes, moyennes
P0-7  Portail famille (lecture + justifs)
P1-1  Inscriptions / pièces / workflow
P1-2  Facturation familles
P1-3  Pont EN (nomenclatures + exports)
P1-4  Communication / messagerie
P2    Cantine, internat avancé, LSU complet, stages
P3+   Infirmerie, RH planning, CFA…
DROP  Paie, compta OGEC, licences Aplim
```

---

## 5. Cible technique (rappel)

| Couche | Choix |
|---|---|
| Domaine + API | **Rust** |
| Persistance | **PostgreSQL** |
| Noms | Vocabulaire métier clair FR/EN code — **pas** les noms `COM_*`/`VS_*` |
| UI intranet | Web (stack actuelle ScolIA / future) |
| Familles / élèves / profs | Apps natives + web dédié |

Les tables Charlemagne = **checklist de couverture**, pas schéma cible.

---

## 6. Fichiers associés dans le repo

| Fichier | Contenu |
|---|---|
| `docs/ent-metier-checklist.md` | Ce document |
| `docs/charlemagne-reference/tables-catalog.json` | 659 tables groupées par préfixe |
| `docs/charlemagne-reference/tables-*-2026-2027.txt` | Listes brutes (Admin = Notes = VS) |

**Ne pas committer** les ZIP HFSQL ni données élèves.

---

## 7. Décisions produit (tranchées côté cible Rust)

- [x] Cantine / badge : **in** ENT (scan + menus + tarifs)
- [x] Infirmerie : **in** module santé (passages, PAI, PDF)
- [x] Factu familles : **in** ENT + export OGEC / SEPA
- [x] Solveur EDT : consultation + greedy ; CP-SAT = hors scope
- [x] Stages : **in** Rust (entreprises + conventions) ; CFA Charlemagne DROP
- [x] Permis à points : **keep** (solde via commission)
- [x] 2FA : **passkeys WebAuthn uniquement** — pas de TOTP ni d'application d'authentification (décision Florian)
- [x] Observabilité / PRA : **in** — `/metrics` (Prometheus, jeton ou 404), journaux JSON, `migration_tag` sur `/health`, **inventaire d'intégrité** par établissement (preuve de fidélité d'une restauration, dérive détectée ligne à ligne), sauvegardes `pg_dump` gardées par loopback. Détail : `docs/ent-rust-observabilite-pra.md`

---

## 8. Prochaine itération recommandée

1. ~~Valider la section 7~~ — tranché (voir §7).
2. ~~Socle domaine Rust P0–P2~~ — livré dans `ent-rust` (lab vert).
3. Brancher les **lives** optionnels : SMTP / SMS / push / banque PSD2 / MinIO (`ENT_*_LIVE`).
4. Apps natives famille : élargir (stages, photo trombi, exports) au fil des besoins démo.
5. CP-SAT EDT : **hors scope** (greedy + contraintes soft suffisent pour consultation).
