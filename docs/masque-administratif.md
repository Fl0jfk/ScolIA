# Masque administratif

Décision Florian, 22 sept 2026. Tout est prévu **dans les faits**, avant les écrans. Un oubli acceptable plus tard = un **réglage** (motif, barème, délai, modèle, qui a le droit). Pas un dossier, pas une année, pas une présence, pas une facture.

Les **objectifs par bloc** (ce qu’il faut pour cocher, à la mesure de Charlemagne / Pronote / École Directe) : [Objectifs par bloc](objectifs-blocs.md). Un geste livré ne coche pas le bloc.

La comptabilité se fait **dans l’ENT**. Elle couvre ce qu’un établissement scolaire tient lui-même. Elle ne va pas jusqu’au travail d’un cabinet (bilan, liasse, plan comptable complet).

Un fichier pour l’expert-comptable peut sortir **après**. Ce n’est pas la comptabilité. La vue `export_comptable_famille` est cette sortie, rien de plus.

---

## Comptabilité — dans l’établissement

Partie **simple** : une recette ou une dépense. Pas de débit / crédit. Pas de plan de comptes.

| On fait dans l’ENT | On ne fait pas |
|--------------------|----------------|
| Tarifs (scolarité, cantine, internat, voyage, fournitures) | Plan comptable, écritures au débit/crédit |
| Factures familles, lignes, avoirs | Bilan, compte de résultat, liasse fiscale |
| Échéances (`facture_echeance`) | Déclarations de TVA |
| Encaissements, rattachement à la facture | Immobilisations, amortissements |
| Impayé = facture moins encaissements | Lettrage général de cabinet |
| Dépenses de l’établissement (`depense` : cantine, voyage, internat…) | DSN, cotisations, bulletin de paie |
| Caisse et banque (`tresorerie_compte`) | Rapprochement bancaire façon cabinet |
| Mouvements entrée / sortie (`mouvement_tresorerie`) | |
| Mandat, IBAN, RUM, quotient (déjà sur le foyer) | |

La paie établissement est **une autre poche** : période + éléments (absence, heure, prime, retenue), branchés sur les absences du personnel. Elle ne mélange pas les factures des familles.

---

## Personnes

| Fait | Où | Rôle |
|------|----|------|
| Élève, INE, identité, photo, statut | `eleve` | Stable d’une année à l’autre |
| Scolarité datée | `eleve_scolarite` | `en_cours`, `prevue`, `terminee`. Une seule en cours |
| Régime daté | `eleve_regime_periode` | Interne / DP / externe, pour la facture |
| Foyer | `foyer` | Adresse, fratrie |
| Responsable | `foyer_responsable` | **Quatre** cases, pas une : autorité, payeur, urgence, **peut récupérer l’enfant** |
| Lien élève ↔ foyer | `eleve_foyer_link` | Garde, foyer principal, autre foyer |
| Pièces | `eleve_document` | Livret, jugement, PAI. Le secret reste le document |

Réglages, pas de nouvelle table : qui voit les notes, les absences, les factures ; élève majeur (ça se déduit de la date de naissance) ; homonymes (l’INE tranche).

La classe encore copiée sur `eleve.classe` est une dette. La vérité de l’année est `eleve_scolarite`.

---

## Années, inscriptions

| Fait | Où |
|------|----|
| Année passée, en cours, à venir | `annee_scolaire` + scolarité `terminee` / `en_cours` / `prevue` |
| Bascule | `annee_bascule` — qui valide, année source, année cible. **Rien n’est effacé** |
| Préinscription | `preinscription` (+ RDV déjà en place) |
| Réinscription | une scolarité `prevue` sur l’année suivante. Pas une troisième table |
| Orientation / vœux | fiches de dialogue déjà là |

Le rituel de bascule (backend, plus tard) : l’année prévue devient l’année en cours, l’ancienne passe `terminee`. Les notes, absences, factures, cahiers de l’année finie restent.

---

## Scolarité, notes, académie

| Fait | Où |
|------|----|
| Matières, périodes, devoirs, notes, moyennes | `note_*` |
| Compétences (LSU) | `note_competence_*` — **une** saisie |
| Envoi académique | `export_academique` (LSU, LSL, Siècle, STS). Trace d’envoi, **pas** une copie des notes |
| Bulletin | `bulletin` + `bulletin_ligne`. Au verrouillage, moyenne et appréciation sont **figées**. Publier ne réécrit pas la note vivante |
| Conseil | `conseil_seance` + `conseil_avis` (décision, mention, appréciation). Le texte de la décision est libre : la liste des mentions est un réglage |
| EDT | `edt_creneau` |
| Cahier de textes | `cahier_texte` — leçon et travail à rendre. Distinct du devoir noté |
| Groupes (LV, options) | `groupe_pedagogique` |

Réglages : barème, coefficients, compétences ou notes, visibilité famille avant le conseil, semaines A/B.

---

## Vie scolaire et « où est l’élève »

Quatre présences, jamais fusionnées.

| Présence | Fait | Ce que ce n’est pas |
|----------|------|---------------------|
| Appel de cours | `vs_appel` | Pas le passage à la porte |
| Absence bulletin | `vs_absence_eleve` | Une sortie scolaire n’en écrit pas |
| Passage | `passage` — sens entrée/sortie, lieu `portail` / `self` / `internat` / `infirmerie` | Le repas **pris** = un passage `self`. Le régime = le droit, pas le pris |
| Internat du soir | `internat_appel` | Distinct de l’appel de la journée |
| Infirmerie | `infirmerie_passage` | L’heure compte pour « où est X ». Le motif court n’est pas le dossier. `signal_vie_scolaire` dit seulement « il est à l’infirmerie » |

Autour : sanctions `vs_sanction`, carnet `vs_carnet`, stages (convention déjà là), sorties scolaires (voyages déjà là), sortie d’internat `internat_sortie` (week-end, correspondant — pas un voyage).

Réglages vie scolaire : motifs d’absence, barème des sanctions, délai de justificatif, forfait cantine ou facturation au passage.

---

## Bloc infirmerie

Le passage (`/sante/passages`, `infirmerie_passage`) est **un geste** : qui est là maintenant. La vie scolaire lit le signal, pas le dossier. **Le bloc n’est pas coché.**

| Volet | Rôle | État |
|-------|------|------|
| Passage | Arrivée, sortie, signal « à l’infirmerie » | Fait |
| Soin du passage | Suite (repos, retour cours, renvoi famille, urgence) + notes — obligatoire à la clôture | Fait (geste) |
| Fiche infirmerie | Antécédents utiles à l’établissement, personnes à prévenir — `/sante/fiches` | Fait (geste) |
| PAI | Protocole, traitements autorisés, document. L’infirmerie valide | À faire |
| Extrait diffusé | Cantine, EPS, voyage, internat, périscolaire — `/sante/extraits`, soft-désactivation | Fait (geste) |
| Médicaments | Journal des prises — `/sante/medicaments` (ordonnance = document) | Fait (geste) |
| Accidents | Registre — `/sante/accidents` (conservation longue, pas de DELETE) | Fait (geste) |
| Inaptitude EPS | Certificat daté → extrait EPS | À faire |
| Nuit internat | Même infirmerie, autre horaire | À faire |
| Secret | Motif = infirmerie. Signal = vie scolaire. Dashboard = volumes, pas le dossier | Règle déjà posée, à tenir partout |
| Documents | PAI, PAP, PPS, ordonnances dans le tiroir santé | Tiroir déjà là |

Hors bloc établissement : le dossier du médecin scolaire, les vaccinations nationales. On ne refait pas un logiciel hospitalier.

Réglages : motifs de passage, qui voit le motif, durée de conservation des fiches.

---

## Internat

Tables prêtes à recevoir, **sans recopier ni effacer** le JSON encore servi par l’écran actuel : bâtiment, chambre, affectation datée (une ouverte par élève), appel du soir, sortie d’internat.

Le branchement de l’écran sur ces tables vient après. Le JSON n’est pas la source d’un wipe.

---

## Factures familles et paie

Déjà là : tarif, foyer de facturation, facture, lignes, encaissement, avoir (`facture.nature`).

Dans l’ENT : échéances, dépenses, compte de caisse ou de banque, mouvements. L’impayé se calcule. La quittance est l’encaissement, pas une table de plus.

Paie : `paie_periode`, `paie_element` lié au personnel et, si besoin, à une absence RH. Pas de bulletin de salaire, pas de cotisations.

---

## Déjà en base, on n’y ajoute pas un double

Dossier élève, foyer, préinscription, RDV, années, appel, absences, sanctions, carnet, notes, compétences, EDT, factures, stages, voyages, messagerie staff, fiches de dialogue.

---

## Volontairement pas dans ce masque

Générateur d’emploi du temps, CDI / prêts, GAR, vidéoprotection, PPMS comme module, bilan et liasse, DSN. Si un jour on les veut, ce sont d’autres produits ou des réglages — pas un trou dans le dossier, l’année, la présence ou la facture.
