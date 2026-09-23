# Objectifs par bloc

Décision Florian, 22 sept 2026. La peur à éviter : entendre « c’est terminé » alors qu’on a livré un geste.

**Mesure.** Charlemagne, Pronote, École Directe, Index, Skolengo. D’abord à leur niveau. Ensuite mieux, et lié. Pas l’inverse.

**Vocabulaire.**

| Mot | Sens |
|-----|------|
| **Geste** | Un écran ou un flux livré (ex. passage infirmerie). |
| **Bloc ouvert** | L’objectif est noté ici. On n’a pas le droit de dire que c’est fini. |
| **Bloc coché** | Chaque volet du bloc est tenu, utilisable, pas une coque. |

Un Value Gate sur un geste ne coche pas le bloc. La jauge peut bouger d’un cran. La case du bloc, non.

---

## Dossier élève — ouvert

Identité, INE, photo, statut, scolarité par année, régime daté, pièces, santé en tiroir, fratrie. Une fiche qu’on ouvre et qui tient l’année en cours et les années passées.

Gestes déjà là : fiche, documents, OCR. Pas coché : la classe encore copiée à plat, le lien propre vers VS / notes / infirmerie / facture depuis la même fiche.

## Dossier foyer — ouvert

Autorité parentale, payeur, urgence, personne qui récupère l’enfant. Plusieurs foyers, garde, droits inégaux (notes, absences, factures).

Gestes déjà là : tables foyer. Pas coché : l’écran qui distingue les quatre rôles et les droits.

## Inscriptions et années — ouvert

Préinscription, réinscription, année passée, année en cours, année à venir. Bascule qui ne efface rien. Orientation (fiches de dialogue).

Gestes déjà là : préinscription, RDV, scolarité `prevue`, table `annee_bascule`. Pas coché : le rituel de bascule et les dossiers des années antérieures consultables comme chez eux.

## Scolarité, EDT, cahier de textes — ouvert

Classes, groupes, matières, emploi du temps, leçon, travail à faire. Semaines A/B en réglage.

Gestes déjà là : EDT en base, lecture famille labo, table cahier de textes. Pas coché : saisie cahier, devoirs à rendre, EDT staff et familles officiels.

## Notes, bulletins, conseils, académie — ouvert

Devoirs, notes, moyennes, compétences, bulletin figé, conseil (avis, mention, passage), envoi LSU / LSL sans seconde saisie.

Gestes déjà là : saisie note et lecture famille labo, tables bulletin et conseil. Pas coché : bulletins publiés, conseil, compétences remontées.

## Vie scolaire — ouvert

Appel, absences, retards, justificatifs, sanctions, carnet, exclusions, punitions. « Où est l’élève » lit sortie, stage, infirmerie, portail. Une sortie n’est pas une absence bulletin.

Gestes déjà là : appel, justifs famille, carnet (signature), feuille du jour. Pas coché : sanctions encore masquées, retards et exclusions comme chez eux, messagerie du carnet.

## Infirmerie — coché

Gestes : passage + suite ; contexte journée / nuit internat ; extraits ; fiches ; médicaments ; accidents ; PAI validé ; **inaptitude EPS → extrait**. Secret : motif à l’infirmerie, signal à la vie scolaire. **Bloc coché** pour ces volets (22 sept 2026) — pas un inventaire concurrent Charlemagne ligne à ligne.

Détail : [Masque administratif](masque-administratif.md), section Bloc infirmerie.

## Cantine et passage — coché

Droit (régime daté) et pris (passage au self). Porte de l’établissement (entrée / sortie). Facturation au forfait ou au réel = réglage.

Gestes déjà là : régime, grille repas, occupancy lit une sortie portail ; **saisie portail** (`/passages/portail`) ; **self repas pris** (`/passages/self`, alerte extrait cantine) ; **prévision** (`/passages/prevision`, droit vs pris) ; **facturation** (`/passages/facturation`, forfait / réel). **Bloc coché** pour ces volets (22 sept 2026).

## Internat — coché

Chambres, lits datés, appel du soir, sorties week-end, infirmerie de nuit, repas du soir. L’écran JSON `/gestion-internat` reste en parallèle (pas d’effacement).

Gestes : **chambres + lit daté** ; **appel du soir Postgres** ; **sorties week-end** (exclus de l’appel) ; **repas du soir** (`/gestion-internat/repas-soir`) ; nuit = geste santé. **Bloc coché** pour ces volets (23 sept 2026).

## Stages — ouvert

Convention, période, élève hors cours, compétences, lien présence. Déjà un module voyages-voisin en prod. Pas coché tant que la présence et le bulletin parlent le même fait.

## Sorties et voyages — ouvert

Le workflow voyages existe (direction, compta, listes, blog). Le bloc est coché quand une sortie vide les cours, la cantine et l’internat, emporte l’extrait PAI, et ne crée pas une absence bulletin. Le moteur a commencé. Pas coché.

## Facturation familles — coché

Tarifs, factures, avoirs, échéances, encaissements, prélèvement, impayés, quittances. Lié au régime et au foyer payeur.

Gestes : tuile + hub ; facture auto ; émettre → parent ; encaissement ; avoirs ; SEPA ; échéancier multi ; board impayés ; quittance (= encaissement). **Bloc coché** pour ces volets (23 sept 2026). Limite : PDF facture S3 parfois absent en local.

## Comptabilité d’établissement — coché

Dans l’ENT, partie simple : recettes familles, dépenses (cantine, voyage, internat), caisse et banque. Pas un logiciel de cabinet (pas de bilan, pas de plan comptable).

Gestes : **écrans `/compta`** ; **encaissements familles → entrées livre** ; **synthèse période** ; **export cabinet CSV**. **Bloc coché** pour ces volets (23 sept 2026).

## Paie établissement — ouvert

Période et éléments liés aux absences du personnel. Pas un bulletin de salaire, pas la DSN. Isolée des factures familles.

Gestes déjà là : tables ; **écrans `/paie`** (période, éléments, figer). Pas coché : lien absences RH → élément ; trou EDT.

## Communication — ouvert

Messagerie familles ↔ établissement, informations, notifications. Le carnet est un canal, pas toute la messagerie. Staff : déjà là. Familles : pas coché.

## RH personnels — ouvert

Dossier, absences, validation direction, remplacement, lien EDT. Déjà un intranet RH utilisable. Le bloc est coché quand l’absence prof tient le trou de cours et la paie légère. Pas coché.

## Échanges académiques — ouvert

Siècle (élèves, responsables), LSU, LSL, STS. Une trace d’envoi, pas une copie des notes. Import Siècle déjà là. Envois du quotidien : pas coché.

## Quotidien familles et profs — ouvert

L’équivalent École Directe : notes, absences, EDT, cahier, messagerie, cantine, factures. Apps natives (Swift, Kotlin) et un web à part. `/famille` est un labo. Pas coché.

---

## Ce qui n’est pas un bloc ScolIA

Générateur d’emploi du temps, CDI, GAR, vidéoprotection, bilan et liasse, DSN, dossier du médecin scolaire. Si on les veut un jour, on le dit. Ce n’est pas un trou dans les blocs ci-dessus.
