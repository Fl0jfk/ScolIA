/** Tutoriels « première visite » par gros module intranet. */

export type ModuleTourExcelColumn = {
  letter: string;
  header: string;
  optional?: boolean;
  sampleValues: string[];
};

export type ModuleTourStep = {
  title: string;
  body: string;
  /** Valeur de l'attribut data-tour sur la page (optionnel). */
  target?: string;
  /** Puces complémentaires. */
  bullets?: string[];
  /** Aperçu type grille Excel (colonnes + exemples). */
  excelPreview?: {
    columns: ModuleTourExcelColumn[];
  };
  /** Aperçu visuel module Sorties scolaires. */
  travelsPreview?:
    | "type-choice"
    | "simple-form"
    | "complex-no-bus"
    | "complex-with-bus"
    | "workflow"
    | "dossier-hub";
  /** Actions à l'entrée / sortie de l'étape (événements module-tour-actions). */
  onEnter?: string;
  onLeave?: string;
  /** Modale plus large (ex. aperçu Excel / voyages). */
  wide?: boolean;
  /** Position du panneau tutoriel (défaut : centre). */
  panelAnchor?: "center" | "bottom";
};

export type ModuleTourDefinition = {
  moduleId: string;
  title: string;
  /** Si défini, le tuto ne s'affiche que pour ces rôles intranet. */
  audienceRoles?: string[];
  steps: ModuleTourStep[];
};

const MODULE_TOURS: ModuleTourDefinition[] = [
  {
    moduleId: "agent-ia-ocr",
    title: "Ajout de documents IA",
    audienceRoles: ["administratif", "direction_ecole", "direction_college", "direction_lycee", "admin"],
    steps: [
      {
        title: "Bienvenue",
        body: "Ce module numérise vos PDF et les range automatiquement dans le dossier OneDrive de chaque élève. La première fois, suivez l'ordre : connexion OneDrive → liste élèves → table MEF → création des dossiers → dépôt des documents.",
      },
      {
        target: "onedrive-connect",
        title: "Connexion OneDrive",
        body: "Connectez-vous avec le compte Microsoft de votre secteur (collège, lycée ou école). Sans cette étape, le dépôt et le classement sont impossibles.",
      },
      {
        target: "eleves-import",
        title: "Liste des élèves (Excel)",
        wide: true,
        body: "Ouvrez ce volet « Configuration », choisissez Pronote ou École Directe, puis importez votre fichier Excel (.xlsx). Sur École Directe, la liste est modulable : placez les colonnes dans cet ordre (A → G). La 1re ligne doit être la ligne d'en-tête.",
        bullets: [
          "OBLIGATOIRE : la ligne 1 = en-têtes. Sans elle, l'import échoue.",
          "Pronote : Scolarité → Exports → liste des élèves (Excel).",
          "École Directe : export liste élèves — réorganisez les colonnes pour suivre l'ordre ci-dessous.",
          "Fusion : le fichier s'ajoute à eleves.json (mises à jour + nouveaux). Les absents du fichier restent.",
          "Remplacer : la liste Excel devient la vérité. Les élèves partis (plus dans le fichier) sortent du référentiel et du pilotage.",
        ],
        excelPreview: {
          columns: [
            {
              letter: "A",
              header: "Nom",
              sampleValues: ["DUPONT", "MARTIN", "BERNARD", "LEROY", "PETIT"],
            },
            {
              letter: "B",
              header: "Prénom",
              sampleValues: ["Marie", "Lucas", "Emma", "Thomas", "Chloé"],
            },
            {
              letter: "C",
              header: "Classe",
              sampleValues: ["3ème A", "3ème B", "4ème A", "3ème A", "4ème B"],
            },
            {
              letter: "D",
              header: "INE",
              sampleValues: [
                "150123456AB",
                "160234567CD",
                "170345678EF",
                "180456789GH",
                "190567890IJ",
              ],
            },
            {
              letter: "E",
              header: "Code MEF",
              sampleValues: [
                "14110003312",
                "14110003312",
                "14110003313",
                "14110003312",
                "14110003313",
              ],
            },
            {
              letter: "F",
              header: "E-mail élève",
              optional: true,
              sampleValues: [
                "marie.dupont@…",
                "lucas.martin@…",
                "—",
                "thomas.leroy@…",
                "—",
              ],
            },
            {
              letter: "G",
              header: "E-mail parent",
              optional: true,
              sampleValues: [
                "parent.dupont@…",
                "parent.martin@…",
                "parent.bernard@…",
                "parent.leroy@…",
                "parent.petit@…",
              ],
            },
          ],
        },
      },
      {
        target: "mef-table",
        title: "Table des formations (MEF)",
        body: "Le code MEF (colonne E de votre Excel) indique la formation de l'élève (ex. 3EME, 2NDE…). La table MEF fait le lien entre ces codes et le secteur : Lycée, Collège ou École.",
        bullets: [
          "À quoi ça sert : chaque secrétariat ne traite que les élèves de son secteur. Sans table MEF, certains élèves peuvent être ignorés à la création des dossiers.",
          "Configuration : une fois par an (ou quand les formations changent) dans Paramètres → Formations MEF — listez les codes ou libellés par secteur.",
          "Bouton « Importer table MEF (JSON) » : raccourci pour charger un fichier JSON déjà préparé (même contenu que Paramètres).",
          "Les codes de la colonne E de l'Excel doivent correspondre à une entrée de cette table.",
        ],
      },
      {
        target: "sync-onedrive",
        title: "Créer les dossiers OneDrive",
        body: "Ce bouton crée les dossiers élèves manquants dans votre arborescence OneDrive (un dossier par élève : NOM Prenom).",
        bullets: [
          "Quand l'utiliser : après l'import de la liste élèves (et idéalement après configuration de la table MEF).",
          "Sans risque : les dossiers déjà présents ne sont jamais supprimés ni renommés — l'outil ajoute uniquement les manquants.",
          "Il est normal d'avoir plus de dossiers sur OneDrive que d'élèves dans la liste : les anciens élèves partis restent archivés.",
          "Après l'action, un bilan détaillé liste les dossiers créés, ceux déjà là et les archives laissées intactes.",
          "Le rapport indique « Dossiers créés » et « Déjà existants ». Les élèves des autres secteurs (lycée / collège / école) ne sont pas touchés.",
        ],
      },
      {
        target: "drop-standard",
        title: "Dépôt des PDF",
        body: "Une seule zone pour tout : un document par élève (bulletin, courrier…) ou un export de classe entière (PDF multi-pages depuis Pronote). L'outil détecte automatiquement le cas, découpe si nécessaire et range chaque document chez le bon élève. Vous pouvez déposer plusieurs fichiers à la fois.",
      },
      {
        title: "Vérification",
        body: "Consultez les résultats en bas de page. En cas d'échec, le fichier reste dans Temp sur OneDrive — vous pouvez le classer manuellement.",
      },
    ],
  },
  {
    moduleId: "stages",
    title: "Stages & conventions",
    steps: [
      {
        title: "Vue d'ensemble",
        body: "Gérez les offres de stage, les conventions déposées par les élèves (PDF), les signatures prof référent + direction, et l'envoi vers OneDrive.",
      },
      {
        target: "stages-board",
        title: "Tableau de bord",
        body: "Résumé des dossiers en attente : dépôts à valider, conventions à signer, file OneDrive.",
      },
      {
        target: "stages-classe",
        title: "Suivi classe",
        body: "Le professeur principal voit tous les élèves de sa classe et l'état de leur stage (sans stage, en cours, validé).",
      },
      {
        target: "stages-conventions",
        title: "Conventions",
        body: "Liste des dossiers par élève. Ouvrez une ligne pour voir le PDF, valider un dépôt ou lancer les signatures.",
      },
      {
        target: "stages-deposer-link",
        title: "Page publique élèves",
        body: "Communiquez le lien /stages/deposer aux élèves : ils déposent leur convention signée (élève, parents, entreprise) en un clic.",
      },
    ],
  },
  {
    moduleId: "travels",
    title: "Sorties scolaires",
    steps: [
      {
        title: "Module Voyage",
        body: "Ce module couvre tout le cycle d'une sortie : demande du professeur, validations direction et finances, bus, cuisine, documents parents et suivi jusqu'à la sortie.",
        bullets: [
          "Deux types de dossiers : sortie de proximité (sans bus) ou voyage / sortie bus (logistique complète).",
          "Chaque dossier a un fil de statuts — vous suivez l'avancement en un coup d'œil.",
        ],
      },
      {
        target: "travels-list",
        title: "Liste des dossiers",
        body: "Toutes vos sorties apparaissent ici sous forme de cartes. Filtrez par établissement (École, Collège, Lycée, Groupe scolaire). Cliquez sur une carte pour ouvrir le dossier complet.",
        bullets: [
          "Les sorties passées apparaissent grisées.",
          "Le bandeau coloré en haut de chaque carte indique l'établissement.",
        ],
      },
      {
        target: "travels-direction",
        title: "Pilotage direction",
        body: "Si vous êtes direction, ce bandeau résume les dossiers à signer, les devis bus en attente et les validations pédagogiques de votre périmètre.",
      },
      {
        target: "travels-create",
        title: "Nouvelle demande",
        body: "Point de départ de toute sortie : ce bouton ouvre le choix du type de déplacement. À l'étape suivante, la fenêtre s'ouvre automatiquement pour vous montrer les deux options.",
      },
      {
        target: "travels-type-modal",
        onEnter: "travels:open-create-modal",
        onLeave: "travels:close-create-modal",
        panelAnchor: "bottom",
        title: "Choisir le type de sortie",
        body: "Deux parcours distincts — choisissez selon le transport et la complexité logistique.",
        bullets: [
          "🍦 Sortie de proximité : cinéma, musée, parc… sans bus ni nuitée. Formulaire allégé, validation direction puis documents.",
          "🚌 Voyage / sortie bus : transport, budget détaillé, éventuellement cuisine et nuitées. Parcours complet avec devis transporteurs.",
        ],
        travelsPreview: "type-choice",
        wide: true,
      },
      {
        title: "Sortie de proximité (sans bus)",
        body: "Après « Sortie de proximité », vous remplissez un formulaire unique : destination, dates, effectifs, coût, pièces jointes. Option pique-nique / repas cuisine si besoin.",
        bullets: [
          "Récurrence possible : même sortie chaque semaine sur une période (jours fériés exclus).",
          "Envoi → statut « Validation pédagogique » : la direction valide ou demande des modifications.",
          "Pas d'onglet Transport dans le dossier — logistique bus désactivée.",
        ],
        travelsPreview: "simple-form",
        wide: true,
      },
      {
        title: "Voyage sans bus",
        body: "Dans « Voyage / sortie bus », vous pouvez décocher « Besoin d'un bus » : le dossier reste de type voyage (budget famille / établissement, objectifs pédagogiques) mais sans demande aux transporteurs.",
        bullets: [
          "Utile pour un séjour en train, covoiturage organisé ou hébergement sans autocar.",
          "L'onglet Transport n'apparaît pas dans le dossier.",
        ],
        travelsPreview: "complex-no-bus",
        wide: true,
      },
      {
        title: "Voyage avec bus",
        body: "Cochez « Besoin d'un bus » : précisez le point de prise en charge, si le bus reste sur place, joignez le programme PDF/Excel. Un récapitulatif s'affiche avant envoi.",
        bullets: [
          "À la création : e-mail automatique aux transporteurs pour demander des devis.",
          "Les devis reçus (e-mail ou saisie manuelle) apparaissent dans l'onglet Transport.",
          "La direction choisit un devis, le signe numériquement, puis le dossier passe en validation finances.",
          "Si l'effectif ou les dates changent après coup : avenant bus envoyé au transporteur retenu.",
        ],
        travelsPreview: "complex-with-bus",
        wide: true,
      },
      {
        title: "Parcours des statuts",
        body: "Chaque dossier avance dans cet ordre (certaines étapes sont ignorées sans bus ou sans cuisine).",
        travelsPreview: "workflow",
        wide: true,
      },
      {
        title: "Dossier sortie — les onglets",
        body: "En cliquant sur une carte, vous accédez au hub du dossier. Les onglets s'adaptent au contenu (transport seulement si bus, cuisine si commande repas).",
        travelsPreview: "dossier-hub",
        wide: true,
        bullets: [
          "Vue d'ensemble : effectifs, dates, budget, actions rapides.",
          "Documents : circulaire parents, autorisations, pièces jointes.",
          "Messagerie interne : échanges prof / direction / compta.",
          "Actions : valider, refuser, demander modification, annuler la séance.",
        ],
      },
      {
        target: "travels-reminders",
        title: "Rappels automatiques",
        body: "Le compteur de rappels (sous le titre) signale les dossiers qui nécessitent une action (signature oubliée, document manquant…). Cliquez pour voir le détail.",
      },
      {
        title: "C'est parti",
        body: "Vous pouvez relancer ce tutoriel à tout moment via le lien en bas de la page. Pour une nouvelle sortie : + Nouvelle demande → choix du type → formulaire → suivi dans la liste.",
      },
    ],
  },
  {
    moduleId: "internat",
    title: "Internat",
    audienceRoles: ["administratif", "direction_ecole", "direction_college", "direction_lycee", "education", "cpe", "admin"],
    steps: [
      {
        title: "Gestion internat",
        body: "Suivez les élèves internes, leurs sorties, autorisations parents et le registre.",
      },
      {
        target: "internat-roster",
        title: "Liste des internes",
        body: "Consultez et mettez à jour la liste des élèves de l'internat.",
      },
      {
        target: "internat-outings",
        title: "Sorties internat",
        body: "Créez une sortie, envoyez les demandes d'autorisation aux parents et suivez les réponses.",
      },
    ],
  },
  {
    moduleId: "rh",
    title: "Module RH",
    audienceRoles: ["administratif", "direction_ecole", "direction_college", "direction_lycee", "comptabilite", "admin"],
    steps: [
      {
        title: "Dossiers personnel",
        body: "Centralisez les documents RH par salarié : contrats, visites médicales, formations…",
      },
      {
        target: "rh-list",
        title: "Annuaire RH",
        body: "Recherchez un membre du personnel et ouvrez son dossier numérique.",
      },
      {
        target: "rh-upload",
        title: "Dépôt de documents",
        body: "Déposez un Excel, PDF ou Word directement sur le dossier d'un salarié via glisser-déposer.",
      },
    ],
  },
  {
    moduleId: "documents",
    title: "Cloud personnel",
    steps: [
      {
        title: "Bienvenue dans votre cloud",
        body: "Ce module, ce n'est pas le classement automatique des dossiers élèves sur OneDrive (ça, c'est « Ajout de documents IA »). Ici, chaque membre du personnel dispose de son propre espace de fichiers sur le cloud Scola — comme un petit disque dur en ligne, accessible depuis n'importe quel poste connecté à l'intranet.",
        bullets: [
          "Fichiers personnels : cours, supports, modèles, scans… ce que vous voulez garder sous la main.",
          "Partages avec des collègues : dossiers communs ou envoi ciblé d'un fichier.",
          "Hébergement sur AWS S3 (région France) — vos données restent dans l'écosystème Scola, pas sur votre PC.",
        ],
      },
      {
        target: "documents-quota",
        title: "Vos 2 Go par personne",
        body: "Chaque compte dispose de 2 Go de stockage personnel. La jauge en haut de page se met à jour après chaque envoi ou suppression. Quand vous approchez de la limite, la barre passe au rouge : il faudra faire du ménage ou archiver ailleurs avant d'importer de gros fichiers.",
        bullets: [
          "2 Go ≈ quelques centaines de PDF ou une dizaine de gros PowerPoint — largement suffisant pour le quotidien pédagogique.",
          "Le quota est individuel : le contenu d'un dossier partagé compte dans l'espace du propriétaire du dossier, pas chez chaque membre.",
          "Pas de quota « illimité » : si vous dépassez, l'upload est refusé avec un message clair.",
        ],
      },
      {
        target: "documents-scope",
        title: "La barre latérale — trois espaces",
        body: "À gauche, trois zones distinctes. Les dossiers partagés y sont listés : c'est souvent par là qu'on les ouvre, et c'est aussi là qu'on peut vérifier qui y a accès.",
        bullets: [
          "Mon cloud : votre espace privé. Personne d'autre ne le voit tant que vous ne partagez pas.",
          "Fichiers partagés : raccourcis vers des fichiers qu'un collègue vous a envoyés individuellement (badge indigo si vous en avez en attente).",
          "Dossiers partagés : espaces collaboratifs. 👑 = vous êtes propriétaire ; 👥 = vous êtes invité.",
          "Clic droit sur un dossier partagé ici : Ouvrir, Voir qui a accès (fenêtre avec la liste du personnel), et Modifier les accès si vous êtes propriétaire.",
        ],
      },
      {
        target: "documents-upload",
        title: "Les boutons d'action",
        body: "Barre d'outils en haut à droite. Quand un dossier partagé est ouvert, deux boutons utiles apparaissent en plus des actions habituelles.",
        bullets: [
          "+ Dossier / + Ajouter un fichier : dans Mon cloud ou dans un dossier partagé où vous avez les droits.",
          "+ Dossier partagé : crée un espace commun et choisissez les collègues (personnel de l'établissement).",
          "Voir qui a accès (bouton indigo) : visible dès qu'un dossier partagé est ouvert — propriétaire 👑, membres et rôles, pour tout le monde qui a accès (pas seulement le propriétaire).",
          "Modifier les accès (bouton sombre, propriétaire seulement) : ajouter ou retirer des membres — remplace l'ancien « Gérer le partage ».",
          "Supprimer le dossier partagé (propriétaire, racine du dossier) : efface tout le contenu et retire le dossier pour tous les membres.",
          "Supprimer ce dossier / Quitter le dossier : selon que vous êtes propriétaire ou invité.",
        ],
      },
      {
        target: "documents-dropzone",
        title: "Glisser-déposer",
        body: "La zone en pointillés accepte un ou plusieurs fichiers, ou un dossier entier depuis votre bureau. L'arborescence des sous-dossiers est conservée à l'import.",
        bullets: [
          "Survolez avec des fichiers : le cadre devient bleu — lâchez pour envoyer.",
          "Pendant l'envoi, la zone est grisée : patientez avant de relancer un dépôt.",
          "Impossible de déposer dans « Fichiers partagés » (ce sont des raccourcis, pas un vrai dossier d'upload).",
        ],
      },
      {
        target: "documents-breadcrumb",
        title: "Fil d'Ariane",
        body: "Le chemin au-dessus de la grille rappelle où vous êtes : Mon cloud, un dossier partagé, ou un sous-dossier. Cliquez sur un segment pour remonter sans repasser par la barre latérale.",
      },
      {
        target: "documents-grid",
        title: "La grille de fichiers",
        body: "Vue type bureau : icônes par type de fichier (PDF, Excel, Word…), nom en dessous, puis « Fichier · taille » ou « Dossier ». Un clic gauche ouvre le fichier (nouvel onglet) ou entre dans le dossier.",
        bullets: [
          "Survol : léger liseré coloré selon le type — repère visuel sans surcharge.",
          "Les fichiers reçus par partage ciblé portent la mention « Partagé » sous le nom.",
          "Double-clic non requis : un simple clic suffit.",
        ],
      },
      {
        title: "Voir qui a accès — trois chemins",
        body: "Avant de déposer un document sensible dans un dossier partagé, vous pouvez toujours vérifier qui le verra. La même fenêtre s'ouvre depuis trois endroits.",
        bullets: [
          "Colonne de gauche : clic droit sur le dossier partagé → Voir qui a accès.",
          "Haut de page : bouton indigo Voir qui a accès (quand le dossier est ouvert).",
          "Grille de fichiers : clic droit sur un élément partagé → Voir qui a accès (dossier partagé ou fichier partagé ciblé).",
          "La fenêtre liste le propriétaire 👑, chaque membre avec son rôle intranet, et (vous) si c'est votre compte.",
          "Modifier les accès reste réservé au propriétaire — consulter la liste, c'est pour tout le monde.",
        ],
      },
      {
        title: "Clic droit — la grille de fichiers",
        body: "Dans la zone centrale, le clic droit ouvre un menu léger : pas de boutons au survol, mais les actions essentielles à portée de main.",
        bullets: [
          "Ouvrir : même effet que le clic gauche.",
          "Voir qui a accès : uniquement sur les éléments partagés — ouvre la fenêtre décrite à l'étape précédente.",
          "Partager (Mon cloud uniquement) : envoyer ce fichier à des collègues — ils le retrouveront dans « Fichiers partagés ».",
          "Déplacer, Supprimer, Retirer du partage : selon vos droits sur l'élément.",
        ],
      },
      {
        title: "Dossier partagé vs partage de fichier",
        body: "Deux mécanismes complémentaires — et dans les deux cas, Voir qui a accès vous indique exactement qui est dans la boucle.",
        bullets: [
          "Dossier partagé : projet d'équipe (rentrée, BTS, coordination pôle…). Même arborescence pour tous ; vérifiez la liste des membres avant d'y déposer des documents confidentiels.",
          "Partage de fichier : envoi ponctuel (convocation, Excel, modèle) — les destinataires le voient dans « Fichiers partagés » ; clic droit sur le fichier pour voir qui d'autre y a accès.",
          "Quitter un dossier partagé : vous disparaissez de la liste des membres ; le propriétaire peut vous réinviter via Modifier les accès.",
        ],
      },
      {
        title: "Qui voit quoi ?",
        body: "Seuls les utilisateurs du personnel inscrits sur l'intranet de votre établissement apparaissent dans les listes de partage et dans la fenêtre Voir qui a accès.",
        bullets: [
          "Mon cloud : strictement privé — pas d'entrée Voir qui a accès.",
          "Dossier partagé : propriétaire + membres désignés, visibles dans la fenêtre.",
          "Fichier partagé ciblé : expéditeur + destinataires choisis uniquement.",
        ],
      },
      {
        title: "Bonnes pratiques",
        body: "Quelques réflexes pour que le cloud reste fluide pour tout le monde.",
        bullets: [
          "Nommez vos dossiers de façon explicite (« Rentrée 2025 — 3ème » plutôt que « Nouveau dossier »).",
          "Évitez les doublons de gros PDF : préférez un lien dans un dossier partagé plutôt que d'envoyer le même fichier à dix personnes.",
          "Avant de déposer dans un dossier partagé : Voir qui a accès — au cas où une personne en trop serait encore membre.",
          "Avant une grosse importation (photos de sortie, archive ZIP), jetez un œil à la jauge des 2 Go.",
          "Ce module ne remplace pas OneDrive pour les dossiers élèves : utilisez « Ajout de documents IA » pour ça.",
        ],
      },
      {
        title: "C'est parti",
        body: "Vous pouvez relancer ce tutoriel à tout moment via le lien en bas de la page. Pour tester : ouvrez un dossier partagé (ou créez-en un), cliquez Voir qui a accès ou faites un clic droit sur son nom dans la colonne de gauche.",
      },
    ],
  },
  {
    moduleId: "requests-staff",
    title: "Demandes",
    steps: [
      {
        title: "Demandes internes",
        body: "Soumettez et traitez les demandes de l'établissement (matériel, locaux, impressions…).",
      },
      {
        target: "requests-inbox",
        title: "Boîte de réception",
        body: "Les demandes qui vous sont assignées ou celles de votre service.",
      },
      {
        target: "requests-new",
        title: "Nouvelle demande",
        body: "Créez une demande avec pièces jointes ; les responsables sont notifiés par e-mail.",
      },
    ],
  },
  {
    moduleId: "prof-room",
    title: "Réservation de salle",
    steps: [
      {
        title: "Bienvenue",
        body: "Réservez les salles (informatique, labo, CDI…) pour vos cours ou activités. Le planning affiche la semaine en cours : vert = libre, couleur = déjà réservé.",
      },
      {
        target: "prof-room-room-select",
        title: "Choisir une salle",
        body: "Sélectionnez la salle dans la liste déroulante, puis naviguez dans le calendrier (flèches ou date) pour afficher la bonne semaine.",
      },
      {
        target: "prof-room-calendar",
        title: "Planning hebdomadaire",
        body: "Cliquez sur un créneau libre (+ LIBRE) pour pré-remplir le formulaire. Cliquez sur une réservation existante pour la modifier (si c'est la vôtre ou si vous êtes admin). Clic droit : copier / coller un créneau sur un autre jour.",
        bullets: [
          "Survolez une réservation pour voir le détail (matière, classe, commentaire).",
          "Vos réservations sont entourées d'un liseré bleu.",
        ],
      },
      {
        target: "prof-room-form",
        title: "Formulaire de réservation",
        body: "Complétez matière, niveau, classe et commentaire. Vous pouvez réserver plusieurs heures d'affilée et programmer une récurrence (hebdo ou bi-hebdo).",
        bullets: [
          "Bouton « Confirmer la réservation » : enregistre le créneau.",
          "En modification : possibilité d'appliquer les changements à toute la série.",
        ],
      },
      {
        target: "prof-room-upcoming",
        title: "Mes prochaines réservations",
        body: "Raccourci vers vos 5 prochains créneaux — cliquez pour rouvrir le formulaire en mode édition.",
      },
    ],
  },
  {
    moduleId: "absences",
    title: "Absences",
    steps: [
      {
        title: "Vue d'ensemble",
        body: "Déposez une demande d'autorisation d'absence (professeur ou personnel OGEC), joignez un justificatif si demandé, et suivez le traitement par la direction ou le secrétariat.",
      },
      {
        target: "absences-tabs",
        title: "Les onglets",
        body: "« Demande d'autorisation » pour votre propre absence. « Calendrier » pour la vue collective (si vous y avez accès). « À traiter » pour les responsables qui valident ou refusent. « Pour un collègue » pour le secrétariat.",
      },
      {
        target: "absences-declare",
        title: "Demande d'autorisation d'absence",
        body: "Indiquez le type (prof / OGEC), l'établissement, les dates, le motif et éventuellement un justificatif PDF. Une fois validée, l'absence est transmise au secrétariat ou à la comptabilité selon le cas.",
      },
      {
        target: "absences-treat",
        title: "Traiter les demandes",
        body: "Réservé aux responsables : validez, refusez ou demandez un justificatif. Pour les professeurs, précisez le traitement des heures (retenue, remplacement…) avant validation.",
        bullets: [
          "La validation est définitive.",
          "Les absences OGEC validées partent vers la comptabilité.",
        ],
      },
      {
        target: "absences-calendar",
        title: "Calendrier",
        body: "Vue mensuelle des demandes d'autorisation d'absence — utile pour la vie scolaire et les directions.",
      },
    ],
  },
  {
    moduleId: "domain-planning",
    title: "Enseignements transversaux",
    steps: [
      {
        title: "Bienvenue",
        body: "Planifiez les créneaux d'enseignements transversaux (EMI, accompagnement, projets…) par domaine et par classe, sur le même principe que la réservation de salles.",
      },
      {
        target: "domain-planning-domain",
        title: "Choisir un domaine",
        body: "Chaque domaine (EMI, latin, etc.) a sa couleur sur le planning. Les coordinateurs du domaine peuvent réserver pour d'autres collègues.",
      },
      {
        target: "domain-planning-calendar",
        title: "Grille de planning",
        body: "Cliquez sur un créneau libre pour réserver. Clic droit pour copier-coller un créneau. Les coordinateurs voient et modifient les créneaux de leur domaine.",
      },
      {
        target: "domain-planning-form",
        title: "Réserver un créneau",
        body: "Libellé d'activité, niveau, classe, commentaire, récurrence. Le créneau est rattaché au domaine et au professeur concerné.",
      },
      {
        title: "Paramétrage",
        body: "Les admins et coordinateurs accèdent à l'onglet « Paramétrage » pour gérer les domaines, les classes par pôle et les coordinateurs.",
      },
    ],
  },
  {
    moduleId: "channels",
    title: "Salons",
    steps: [
      {
        title: "Messagerie interne",
        body: "Échangez avec vos collègues par salons de discussion (publics ou privés), partagez des fichiers et suivez les conversations en temps réel.",
      },
      {
        target: "channels-list",
        title: "Liste des salons",
        body: "Les salons publics sont accessibles à tous. Les salons privés (cadenas) n'apparaissent que pour les membres invités. Le point rouge signale des messages non lus.",
        bullets: [
          "Bouton ＋ : créer un nouveau salon (public ou privé avec sélection des membres).",
        ],
      },
      {
        target: "channels-messages",
        title: "Fil de discussion",
        body: "Lisez les messages du salon actif. Vous pouvez supprimer vos propres messages ; les créateurs de salon peuvent gérer les membres.",
      },
      {
        target: "channels-compose",
        title: "Envoyer un message",
        body: "Tapez votre message, joignez un fichier (📎), puis envoyez. Le mode « Anonyme » masque votre nom (utile pour des retours discrets).",
      },
    ],
  },
  {
    moduleId: "photocopies-couleur",
    title: "Photocopies couleur",
    steps: [
      {
        title: "Demandes d'impression",
        body: "Les enseignants et le personnel déposent une demande de photocopies couleur. La direction de l'établissement concerné accepte ou refuse avant envoi au service impressions.",
      },
      {
        target: "photocopies-new",
        title: "Nouvelle demande",
        body: "Choisissez l'établissement, le motif, les classes ou la matière, le nombre de copies et joignez le PDF à imprimer (recommandé).",
        bullets: [
          "Un e-mail valide sur votre compte est requis pour recevoir la décision.",
        ],
      },
      {
        target: "photocopies-mine",
        title: "Mes demandes",
        body: "Suivez l'état de vos demandes : en attente, acceptée ou refusée, avec le message éventuel de la direction.",
      },
      {
        target: "photocopies-queue",
        title: "File direction",
        body: "Pour les directions : traitez les demandes de votre pôle (école, collège ou lycée). Acceptez ou refusez — le demandeur est notifié par e-mail.",
      },
    ],
  },
  {
    moduleId: "conformite-rgpd",
    title: "Conformité RGPD",
    audienceRoles: [
      "administratif",
      "direction_ecole",
      "direction_college",
      "direction_lycee",
      "admin",
    ],
    steps: [
      {
        target: "rgpd-module",
        title: "Module Conformité RGPD",
        body: "Ce module vous aide à inventorier les documents RGPD, mesurer votre conformité et gérer les incidents. Ce n'est pas un conseil juridique — validation par le DPD/DPO et la direction.",
      },
      {
        title: "Questionnaire",
        body: "Répondez au questionnaire pour déterminer quels documents sont obligatoires ou recommandés pour votre établissement (école, collège, lycée, sous-traitants, traitements sensibles…).",
        bullets: [
          "7 étapes avec sauvegarde automatique",
          "Synthèse finale : liste des documents applicables",
        ],
      },
      {
        title: "Documents & score",
        body: "Le tableau de bord affiche une note sur 100. Pour chaque document : générez un PDF prérempli, importez le vôtre, ou lancez l'analyse IA pour identifier les manques.",
        bullets: [
          "Registre, mentions d'information, procédures, sous-traitants…",
          "L'IA compare votre document aux critères CNIL attendus",
        ],
      },
      {
        title: "Incidents",
        body: "Déclarez une violation de données ou un incident de sécurité. Décrivez la situation au bot : il préremplit la fiche que vous validez avant enregistrement et export PDF.",
      },
    ],
  },
];

export function getModuleTour(moduleId: string): ModuleTourDefinition | undefined {
  return MODULE_TOURS.find((t) => t.moduleId === moduleId);
}

export function tourVisibleForRoles(tour: ModuleTourDefinition, roles: string[]): boolean {
  if (!tour.audienceRoles?.length) return true;
  if (roles.includes("admin") || roles.includes("master")) return true;
  return tour.audienceRoles.some((r) => roles.includes(r));
}
