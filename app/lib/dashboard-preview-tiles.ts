/** Tuiles du mock tableau de bord — contenu aligné sur les modules réels du produit. */

export type DashboardPreviewTile = {

  id: string;

  label: string;

  title: string;

  summary: string;

  features: string[];

};



export const DASHBOARD_PREVIEW_TILES: DashboardPreviewTile[] = [

  {

    id: "documents",

    label: "Cloud personnel",

    title: "Cloud personnel & partages",

    summary:

      "Chaque prof et membre du personnel dispose de 2 Go de stockage cloud (AWS S3), avec quota affiché en temps réel.",

    features: [

      "Espace personnel : dossiers, upload, glisser-déposer",

      "Dossiers partagés entre collègues avec gestion des membres",

      "Partage de fichiers ciblés avec d'autres utilisateurs",

      "Hébergement France — pas de limite artificielle de modules",

    ],

  },

  {

    id: "docs-eleves",

    label: "Docs élèves",

    title: "Tri documentaire assisté par IA",

    summary:

      "Workflow OCR + Mistral : déposez des bulletins ou dossiers, l'IA segmente, lit et range au bon endroit.",

    features: [

      "OCR multi-pages et découpage automatique par élève",

      "Matching nom / classe / secteur MEF",

      "Renommage et déplacement vers OneDrive ou dossiers élèves",

      "Traitement par lots depuis le tableau de bord",

    ],

  },

  {

    id: "sorties",

    label: "Sorties",

    title: "Sorties scolaires de A à Z",

    summary: "Parcours complet avec validations, finances, transport et documents parents.",

    features: [

      "Workflow : pédagogie → finances → direction → validation",

      "Devis bus, signatures PDF, circulaire parents générée",

      "IA sur devis entrants (OCR, extraction montants, e-mails transporteurs)",

      "Sorties simples ou complexes, séries et rappels automatiques",

    ],

  },

  {

    id: "absences",

    label: "Absences",

    title: "Déclaration & calendrier",

    summary: "Les équipes déclarent, la direction valide — avec vue calendrier partagée.",

    features: [

      "Se déclarer : demi-journée, journée ou période multi-jours",

      "Calendrier visuel avec code couleur par enseignant",

      "Déclaration pour une autre personne (direction / vie scolaire)",

      "Validation, motifs, pièces justificatives PDF",

    ],

  },

  {

    id: "salles",

    label: "Salles",

    title: "Réservation de salles",

    summary: "Grille hebdomadaire claire — fini les tableurs et les conflits de créneaux.",

    features: [

      "Planning par salle, réservation au créneau horaire",

      "Récurrence hebdomadaire ou quinzaine",

      "Matières en couleurs, classes par pôle (école, collège, lycée)",

      "Ajout de salles, horizon de réservation et paramètres admin",

    ],

  },

  {

    id: "rh",

    label: "RH",

    title: "Personnel & dossiers salariés",

    summary: "L'administratif RH centralisé — sans remplacer votre logiciel de paie.",

    features: [

      "Annuaire du personnel, fiches et documents par salarié",

      "Dépôt rapide : l'IA propose le bon dossier destinataire",

      "Absences, HSE, annuaire, espace « Mon dossier »",

      "Signatures et suivi des arrivées / pièces RH",

    ],

  },

  {

    id: "internat",

    label: "Internat",

    title: "Vie de l'internat",

    summary: "Chambres, internes, sorties et appels — dans le même intranet.",

    features: [

      "Gestion des chambres et affectation des internes",

      "Sorties autorisées, activités et planning",

      "Appel du soir, alertes et statistiques",

      "Digest hebdomadaire parents (optionnel)",

    ],

  },

  {

    id: "demandes",

    label: "Demandes",

    title: "Corbeilles & suivi",

    summary: "Les demandes arrivent au bon service et se traitent en tableau Kanban.",

    features: [

      "Dépôt avec pièces jointes, routage IA vers le bon pôle",

      "Tableau : nouvelles, en cours, en attente, terminées",

      "Corbeilles par service, prise en charge et délégation",

      "Suivi côté demandeur + rappels par e-mail",

    ],

  },

  {

    id: "familles",

    label: "Familles",

    title: "Liens & simulateurs parents",

    summary: "Pages publiques dédiées, sans surcharger l'ENT.",

    features: [

      "Liens de préparation de rentrée par niveau",

      "Simulateur de tarifs scolaires pour les familles",

      "Simulateur fournitures et outils saisonniers sur demande",

      "Accès parents prévu dans la feuille de route",

    ],

  },

];

/** Piliers déjà montrés par les animations — exclus du tableau bonus. */
export const DASHBOARD_PILLAR_TILE_IDS = new Set([
  "docs-eleves",
  "sorties",
  "salles",
  "absences",
  "demandes",
]);

export const DASHBOARD_BONUS_TILES = DASHBOARD_PREVIEW_TILES.filter(
  (t) => !DASHBOARD_PILLAR_TILE_IDS.has(t.id),
);

