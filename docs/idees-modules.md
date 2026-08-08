# Idées de modules Scola

Backlog produit — outils **supplémentaires** qui ne font pas ce qu’ÉcoleDirecte / Pronote font déjà.

> **ÉcoleDirecte informe et gère la scolarité. Scola mobilise, organise et active — une démarche à la fois.**

Dernière mise à jour : 8 août 2026

---

## Deux niveaux de produit

| Niveau | Quoi | Exemples |
|--------|------|----------|
| **Killers** | Gros modules — valeur quotidienne ou stratégique, argument de vente principal | OCR documents, sorties, réservation salles |
| **Boîte à outils saisonnière** | Petites briques activables à la demande — sympas, utiles, pas le cœur du pitch | QR code… (PO / Rentrée / Santa pilotés depuis **Établissement → Événements**) |

Les killers font vendre Scola. La boîte à outils crée de la **fidélité** et du **bouche-à-oreille** (« ils ont même un truc pour les portes ouvertes »).

---

## Charte (rappel)

| On fait | On ne fait pas |
|--------|----------------|
| Workflows ciblés (proposer → modérer → clôturer) | Messagerie parents ↔ profs |
| Parents / élèves **ponctuels** (token, code, rôle limité) | Portail parent ou élève généraliste |
| Orchestration + purge | Coffre-fort / second registre élève |
| Outils privés sous contrôle de l’établissement | Notes, bulletins, devoirs, EDT officiel |

**Cible actuelle :** établissements privés — parents souvent bien placés professionnellement (gérants, cadres, réseaux d’entreprises).

---

## Légende des statuts

| Statut | Signification |
|--------|---------------|
| `produit` | Déjà en production |
| `a-produire` | Idée validée, à développer |
| `reflexion` | Intéressant, spec / risques à trancher |
| `plus-tard` | Pas prioritaire pour l’instant |
| `abandonne` | Écarté volontairement |

---

## Killers — déjà en production

| Module | Pourquoi c’est un killer |
|--------|-------------------------|
| **Documents + OCR IA → OneDrive** | Personne ne range les dossiers élèves comme ça. Gain de temps massif admin. |
| **Sorties scolaires** | Workflow complet de A à Z + IA. Les ENT ne font pas ça. |
| **Réservation de salles & transversaux** | Fini les tableurs ; usage quotidien profs. |
| **Demandes & corbeilles** | Tout l’établissement dépose et suit — routing par service. |
| **RH & personnel** | Dossiers salariés centralisés (à garder léger côté RGPD). |
| **Internat** | Vertical métier complet + tokens parents. |
| **Covoiturage** | Outil famille unique, zéro concurrence ENT. |

---

## Killers — candidats (gros impact)

| Idée | Public | Statut | Pourquoi killer |
|------|--------|--------|-----------------|
| **Événements établissement** (moteur sous-jacent) — créneaux, formulaire public, confirmations mail, export tableur/PDF, fichier `.ics` calendrier | Admin, communication, familles externes | `a-produire` | En **privé**, portes ouvertes = **recrutement** = enjeu business. Remplace Eventbrite + Excel + mails manuels. Premier cas : **portes ouvertes**. |
| **Bourse aux stages / PFMP** — parents proposent, élèves candidatent, établissement modère | Parents, élèves, direction | `a-produire` | Réseau des familles en privé = avantage compétitif impossible pour un ENT générique. |
| **Bot bien-être / signalement** — harcèlement, mal-être ; canal d’écoute et orientation | Élèves | `reflexion` | Différenciant fort en privé. Spec RGPD avant dev. Pas une messagerie profs. |
| **Rentrée digitale** (bundle) — simulateurs tarifs/fournitures + inscriptions + docs + checklists | Familles, admin | `reflexion` | Déjà des pages publiques ; à packager en killer saisonnier récurrent (chaque août-sept). |
| **IA de routage universel** — tout dépôt (mail, formulaire, doc) classé et envoyé au bon service | Admin | `plus-tard` | Extension naturelle de l’OCR + demandes. Vision « plus personne ne se demande à qui envoyer quoi ». |

---

## Boîte à outils saisonnière

Petits modules **activables / désactivables** par l’admin (par saison ou à la demande). Regroupés dans une tuile **« Boîte à outils »** sur le dashboard.

| Outil | Saison / moment | Statut | Notes |
|-------|-----------------|--------|-------|
| **Portes ouvertes** | Sept–janv. (pic nov.) | `a-produire` | Formulaire en ligne, choix de créneau, mail de confirmation, **ajout calendrier (.ics)**, export tableur ou PDF par créneau. Alimenté par le moteur **Événements établissement**. |
| **Secret Santa** | Décembre | `a-produire` | Tirage au sort anonyme (staff ou classes). Petite bricole sympa, zéro données sensibles. |
| **QR Code** | Toute l’année | `produit` | Déjà en prod — à **déplacer dans la boîte à outils** (tuile groupée, pas module isolé au milieu des killers). |
| **Concours inter-classes** | Toute l’année (1×/mois) | `reflexion` | ~10 min, classes d’un niveau, points, champion fin d’année. Gamification, pas notes ED. |
| **Tombola / kermesse** | Printemps | `plus-tard` | Numéros, tirage, liste participants — même moteur événements |
| **Cartes de vœux équipe** | Décembre | `plus-tard` | Génération collective sympa |
| **Inscription photo scolaire** | Sept. | `plus-tard` | Créneaux + rappel — moteur événements |

**Principe technique :** un seul moteur (créneaux + formulaire + export + mail + `.ics`), des **templates saisonniers** par-dessus.

---

## Tes idées — backlog élargi

Idées citées en discussion — classées honnêtement par ambition.

| Idée | Niveau | Statut | Notes |
|------|--------|--------|-------|
| Portes ouvertes (formulaire, tableur, .ics) | Boîte à outils → alimente killer **Événements** | `a-produire` | Pas une niche : stratégique en privé. |
| Secret Santa | Boîte à outils | `a-produire` | Simple, viral, fidélisation. |
| Bourse PFMP / stages parents | Killer | `a-produire` | |
| Bot bien-être / harcèlement | Killer | `reflexion` | |
| Concours inter-classes | Boîte à outils | `reflexion` | |
| Inscriptions parascolaires (chorale, théâtre…) | Killer léger ou boîte à outils | `reflexion` | Utile mais moins « waouh » que sorties/OCR — peut attendre après événements. |
| Bac à sable SNT | Killer léger | `reflexion` | Cool pour profs SNT, pas un argument de vente principal. |
| Diplôme activités artistiques | Niche (sauf filière arts) | `plus-tard` | Demande La Providence — garder si pilote local, pas priorité produit nationale. |

---

## Sorties scolaires (Travels) — liste élèves & communication

Socle livré : sélection nominative, flux bus, com’ parents optionnelle.

### 1. Liste d’élèves (socle)

| Idée | Statut | Notes |
|------|--------|-------|
| **Sélection depuis `eleves.json`** | `produit` | **Pas d’Excel en V1.** Choisir d’abord une ou plusieurs **classes**, puis cocher les élèves (tout sélectionner / sélection fine). Les mails parents viennent du JSON si disponibles. Onglet hub **Élèves**. |
| **Rappel droit à l’image** | `produit` | Au moment de composer la liste : rappel que le droit à l’image est géré **en interne** par l’établissement. Case « OK » **cochée par défaut** (responsabilité établissement). Ajustable élève par élève. |

### 2. Transporteur — **obligation** (si bus)

| Idée | Statut | Notes |
|------|--------|-------|
| **Rappel J−3 / J−4** | `produit` | Type `bus_liste_j3` + `remindersSent` : mail au créateur « Confirmez la liste » → `/travels/{id}?tab=eleves`. |
| **Confirmation → envoi auto au transporteur** | `produit` | Liste confirmée → CSV (nom, prénom, classe) au transporteur retenu (`selectedBusQuote`). |

### 3. Communication parents — **option**, pas obligation

| Idée | Statut | Notes |
|------|--------|-------|
| **Onglet / flux Communication** | `produit` | Message + photos compressées → mails parents (BCC). Multi-envois ; journal `parentComLogs` (métadonnées). Sens unique. |
| **Mail du jour J (proposition)** | `produit` | Rappel `com_parents_j0` au créateur le jour du départ → `?tab=communication`. Jamais bloquant. |

### Parcours cible (résumé)

1. Sélection classes → élèves (`eleves.json`) + rappel droit à l’image (défaut OK).  
2. J−3/4 (bus) → rappel « confirmez la liste ».  
3. Liste confirmée → **part auto au transporteur**.  
4. Jour J → proposition « communiquer aux parents » (optionnel, multi-envois).

### Pourquoi c’est intéressant
- Liste élèves = besoin métier réel (bus) + base pour la com’.
- WhatsApp / réseaux : numéros exposés, pas universel ; le mail reste le canal le plus égalitaire.
- Séparation claire : **transporteur = obligatoire** · **com’ parents = possibilité**.

### Points d’attention
- RGPD photos + mails : rétention courte, pas d’album long terme dans ScolIA ; compression / PJ plutôt que stockage.
- Droit à l’image : rappel + responsabilité établissement.
- Qui confirme / envoie : owner + direction (mêmes droits d’édition effectif).
- Ne pas confondre avec une messagerie parents ↔ profs (hors charte).

---

## Socle technique réutilisable

| Brique | Statut | Sert pour |
|--------|--------|-----------|
| **Moteur événements** (créneaux, inscription, mail, export, `.ics`) | `a-produire` | Portes ouvertes, kermesse, photo scolaire, rentrée… |
| **Moteur d’offres** (proposer → modérer → candidater → purger) | `reflexion` | PFMP, interventions parents |
| **Tokens & liens signés** | `produit` (partiel) | Internat, événements publics sans compte |
| **Templates saisonniers** (activer/désactiver) | `a-produire` | Boîte à outils |

---

## Réserve — idées niche (basse priorité)

À garder en réserve, pas pour le pitch commercial principal :

<details>
<summary>Cliquer pour voir la réserve</summary>

- Diplôme / portfolio activités artistiques
- Bac à sable SNT
- Inscriptions parascolaires seules (sans moteur événements)
- Permanences & surveillances
- Prêt matériel, costumes, registre visiteurs…
- Mentorat parent, chambre des métiers, expo virtuelle, etc.

</details>

---

## Déjà en production (liste complète)

| Module | Niveau |
|--------|--------|
| Documents + OCR IA → OneDrive | Killer |
| Sorties scolaires | Killer |
| Réservation de salles & transversaux | Killer |
| Absences personnel | Killer |
| Demandes & corbeilles | Killer |
| RH & personnel | Killer |
| Internat | Killer |
| Covoiturage | Killer |
| Photocopies, HSE, assistance | Utilitaire |
| Salons, annuaire établissement | Utilitaire |
| **Événements** (hub PO / Rentrée / Santa) | Établissement |
| **QR Code** | Boîte à outils |
| Planning RH (profs A/B + OGEC) | Killer léger (RH) |
| Brain AI / chatbot | Killer léger |
| Échéances académiques, feuille de semaine | Widget dashboard |
| Paramètres, membres, plateforme | Admin |
| Pages publiques rentrée / simulateurs | Killer léger (à renforcer) |

**Liens externes :** ÉcoleDirecte, ZeenDoc, Arena Ac-Normandie.

---

## Points d’attention (RGPD & ENT)

- **Événements publics** (portes ouvertes) : données prospects — consentement, durée conservation courte, export puis purge.
- **Secret Santa** : pas de données élève si version staff ; version classe = minimiser.
- **Bot harcèlement** : anonymat, référent, rétention courte — pas dossier disciplinaire ED.
- **PFMP** : modération, pas d’annuaire familles ouvert, purge fin d’année.
- **`eleves.json`** : à réduire — pas de second registre.
- **Travels** : liste via `eleves.json` (classes → élèves) ; rappel droit à l’image (défaut OK, géré en établissement) ; liste bus **obligatoire** (rappel J−3/4 → confirm → envoi transporteur) ; com’ parents **optionnelle** (mail J + multi-envois message/photos).

---

## Journal des ajouts

| Date | Ajout |
|------|-------|
| 2026-08-08 | **Établissement** : Annuaire (ex-organigramme) + hub Événements (PO/Rentrée/Santa) + Identité ; **Planning RH** profs A/B + OGEC. |
| 2026-07-30 | **Travels** : liste élèves + com’ parents **livrés** (`produit`) — onglets Élèves / Communication, confirm → CSV transporteur, rappels `bus_liste_j3` / `com_parents_j0`. |
| 2026-07-30 | **Travels** : sélection classes/élèves (`eleves.json`) ; rappel droit à l’image ; flux bus ; com’ parents optionnelle — backlog puis livraison. |
| 2026-06-13 | Création du document. |
| 2026-06-13 | Inscriptions, SNT, diplôme artistique, idées diverses. |
| 2026-06-13 | **Refonte** : séparation Killers / Boîte à outils saisonnière. Ajout portes ouvertes, Secret Santa, moteur événements. QR code → boîte à outils. Idées niche → réserve. |

---

## Prochaines étapes suggérées

1. **Moteur événements** — premier template : **portes ouvertes** (créneaux, formulaire, mail, `.ics`, export tableur/PDF).
2. **Boîte à outils** — tuile dashboard qui regroupe QR code + modules saisonniers activables.
3. **Secret Santa** — premier petit module saisonnier (rapide à livrer, effet « sympa »).
4. **Bourse PFMP** — prochain killer métier.
5. Regrouper **QR code** visuellement dans la boîte à outils (sans casser les routes).
