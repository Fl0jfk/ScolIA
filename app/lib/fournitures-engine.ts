import type {
  FournituresSection,
  FournituresChild,
  FournituresProfileOverrides,
  EcoleNiveau,
  CollegeNiveau,
  LyceeNiveau,
  LyceeTrack,
  LangueSeconde,
  LyceeSpecialite,
  LyceeOption,
} from "./fournitures-types";

export type {FournituresChild, EcoleNiveau, CollegeNiveau, LyceeNiveau, LyceeTrack, LangueSeconde, LyceeSpecialite, LyceeOption} from "./fournitures-types";

export type { FournituresStage as Stage } from "./fournitures-types";

type SupplySection = FournituresSection;

export function getEcoleSuppliesLegacy(niveau: EcoleNiveau): SupplySection[] {
  const je1 = [
    "1 change complet marqué, dans un petit sac qui reste au porte-manteau (à renouveler quand nécessaire).",
    "2 photos d’identité pour le jour de la rentrée.",
    "2 boîtes de mouchoirs en papier (à renouveler de temps en temps).",
    "1 tablier de peinture ou la chemise de papa avec des manches longues.",
    "1 duvet léger et un petit oreiller (marqués) au nom de l’enfant.",
    "1 gros classeur à levier dans lequel vous mettrez 100 pochettes plastique.",
    "1 cahier 96 pages 24x32.",
    "1 pochette Canson de couleurs vives.",
    "1 paire de chaussons.",
    "Pour les enfants qui ont une tétine pour dormir, celle-ci doit être mise dans une boîte uniquement prévue à cet effet pour éviter la propagation des microbes.",
  ];
  const je2 = [
    "2 photos d’identité pour le jour de la rentrée.",
    "1 duvet et un petit oreiller (dans un sac en tissu) pour le temps de repos.",
    "1 change complet dans un sac.",
    "1 paire de chaussons.",
    "1 pochette de 12 feutres pointe large.",
    "1 pochette de 12 feutres pointe fine.",
    "2 feutres Velléda noirs.",
    "1 classeur souple.",
    "1 gros classeur à levier dans lequel vous mettrez 100 pochettes plastique + 5 intercalaires (grand format).",
    "1 lutin de 15 vues.",
    "1 pochette de feuilles canson blanc.",
    "1 pochette canson de couleurs vives.",
    "2 boîtes de mouchoirs en papier.",
    "Chaque objet (même les sacs) devra être marqué au nom de l’enfant.",
    "Pour les petits qui ont une tétine pour dormir, celle-ci doit être mise dans une boîte uniquement prévue à cet effet pour éviter la propagation des microbes.",
  ];
  const je3 = [
    "2 photos d’identité (Pour le jour de la rentrée).",
    "2 boîtes de mouchoirs en papier.",
    "1 gros classeur à levier gris + 100 pochettes plastique épaisses.",
    "6 intercalaires format 24X32.",
    "1 lutin de 30 vues.",
    "3 pochettes de 12 feutres pointes moyennes.",
    "1 ardoise Velleda + effaceur.",
    "MS : prévoir oreiller + duvet pour les enfants qui font la sieste.",
  ];
  const je4 = [
    "2 photos d’identité.",
    "2 boîtes de mouchoirs en papier.",
    "1 gros classeur à levier avec 50 pochettes plastiques.",
    "12 intercalaires format 24X32.",
    "1 lutin de 30 vues.",
    "1 pochette de 12 feutres pointes fines.",
    "1 pochette de 12 feutres pointes larges.",
    "1 ardoise Velleda avec effaceur.",
    "4 feutres Velleda.",
    "5 bâtons de colle.",
    "1 pochette de feuilles canson blanches.",
    "1 pochette de feuilles canson de couleurs vives.",
  ];
  const cpSupplySections: SupplySection[] = [
    { title: "CP — général", items: ["1 agenda une page par jour."] },
    {
      title: "Trousse n°1",
      items: [
        "1 crayon à papier HB.",
        "1 surligneur jaune.",
        "1 stylo bleu effaçable type Frixion.",
        "1 stylo vert effaçable type Frixion.",
        "1 taille-crayons avec réserve.",
        "1 paire de bons ciseaux.",
        "1 tube de colle.",
        "1 double décimètre non flexible et pas en fer.",
      ],
    },
    {
      title: "Trousse n°2",
      items: ["12 feutres.", "12 crayons de couleur."],
    },
    {
      title: "Trousse n°3",
      items: [
        "5 crayons à papier HB.",
        "1 gomme.",
        "4 tubes de colle.",
        "10 feutres ardoise.",
        "1 ardoise Velleda + 1 feutre ardoise + 1 effaceur.",
        "1 lutin (21x29,7) de 100 vues.",
        "2 pochettes à élastiques à rabats (1 bleue, 1 rouge).",
        "1 cahier 24 x 32 grands carreaux 96 pages sans spirale.",
        "3 boîtes de mouchoirs en papier.",
        "1 paire de chaussures de sport.",
      ],
    },
  ];
  const ce1 = [
    "Stylos (1 bleu, 1 vert, 1 rouge).",
    "5 crayons à papier HB.",
    "1 taille-crayons avec réserve.",
    "1 gomme blanche.",
    "5 sticks de colle étiquetés.",
    "1 ardoise Velleda et 4 feutres bleus + 1 chiffon.",
    "3 surligneurs de couleurs différentes.",
    "1 paire de bons ciseaux.",
    "1 boîte de 12 feutres pour colorier (dans une trousse).",
    "1 pochette de 12 crayons de couleurs (dans une trousse).",
    "1 double décimètre non flexible et pas en fer.",
    "1 équerre.",
    "1 agenda une page par jour.",
    "1 pochette rabats à élastiques rouge.",
    "1 lutin (21x29,7) de 100 vues (celui de CP suffit).",
    "1 pochette papier Canson blanc.",
    "1 pochette papier Canson couleurs.",
    "1 pochette de réserve (type sac congélation) contenant crayons à papier, colle, feutre Velleda…",
    "2 boîtes de mouchoirs en papier.",
    "1 paire de chaussures de sport.",
  ];
  const ce2 = [
    "1 crayon à papier HB.",
    "1 taille-crayons.",
    "1 gomme blanche.",
    "4 stylos (rouge, vert, bleu, noir) effaçables.",
    "4 surligneurs de 4 couleurs différentes.",
    "1 paire de ciseaux.",
    "1 stick de colle.",
    "1 ardoise Velleda + 2 feutres + 1 chiffon.",
    "12 feutres et 12 crayons de couleur dans une trousse.",
    "1 règle de 20 cm en plastique solide.",
    "1 équerre plastique.",
    "1 compas dans une boîte avec des mines de rechange.",
    "1 dictionnaire couvert (si achat Larousse junior).",
    "1 agenda scolaire.",
    "1 lutin (40 vues).",
    "1 pochette de feuilles Canson blanc 180 gr (24x32).",
    "1 pochette de feuilles Canson de couleurs vives 160 gr (24x32).",
    "1 trousse de réserve qui se ferme (2 crayons à papier + 9 sticks de colle + 3 feutres Velleda + recharges stylos effaçables).",
    "1 paire de rollers avec protections et casque (utilisation après les vacances de Pâques).",
    "2 boîtes de mouchoirs.",
  ];
  const cm1 = [
    "1 stylo-plume avec 1 effaceur ou stylo effaçable et des cartouches.",
    "1 crayon à papier HB + 1 critérium + les mines.",
    "1 gomme blanche.",
    "1 taille-crayons.",
    "2 surligneurs.",
    "4 stylos à bille (rouge, bleu, vert, noir).",
    "2 bâtonnets de colle de bonne qualité (à renouveler fréquemment).",
    "1 paire de ciseaux.",
    "1 double décimètre non flexible et pas en fer.",
    "1 équerre (partant de 0 sans espace).",
    "1 compas de bonne qualité.",
    "12 feutres et 12 crayons de couleur.",
    "1 bloc de sténo (14,8 x 21) (feuilles blanches).",
    "1 ardoise Velleda et des feutres fins Velleda (4x2).",
    "1 chiffon.",
    "1 dictionnaire couvert (si achat : Larousse illustré).",
    "1 agenda (1 page par jour).",
    "1 protège cahier petit format transparent (avec le nom).",
    "1 pochette de papier Canson de couleur 160 g (24 x 32).",
    "1 pochette de papier Canson blanc 180 g (24 x 32).",
    "1 lutin de 40 vues ou plus (celui de l’an dernier suffit).",
    "2 boîtes de mouchoirs.",
    "1 paire de rollers avec protections et casque.",
  ];
  const cm2 = [
    "1 stylo-plume et des cartouches d’encre bleue, effaçable, avec 1 effaceur.",
    "Ou stylos effaçables et des recharges.",
    "4 stylos-billes (1 rouge, 1 vert, 1 bleu, 1 noir).",
    "1 crayon à papier HB.",
    "1 taille-crayons.",
    "1 gomme blanche.",
    "2 colles en bâtonnet (à renouveler).",
    "3 surligneurs.",
    "1 ardoise + feutres Velleda.",
    "1 paire de ciseaux.",
    "1 double-décimètre plastique.",
    "1 équerre plastique.",
    "1 compas de bonne qualité.",
    "Feutres et crayons de couleur (12).",
    "1 agenda (1 page par jour).",
    "1 dictionnaire couvert (nous conseillons Larousse illustré).",
    "1 pochette de feuilles de Canson blanc 180 gr (24 x 32).",
    "1 pochette de Canson de couleurs vives 160 gr (24 x 32).",
    "des étiquettes.",
    "1 grand classeur.",
    "2 boîtes de mouchoirs en papier.",
  ];
  const sportEncartEcole: SupplySection = {
    title: "Sport",
    items: ["1 sac contenant :", "1 short ou 1 pantalon.", "1 paire de chaussures de sport."],
  };
  if (niveau === "JE1MMEBAYEL") { return [{ title: "À acheter (J.E.1 — Mme BAYEL Christine)", items: je1 }];}
  if (niveau === "JE2MMECARTIER") { return [{ title: "À acheter (J.E.2 — Mme CARTIER Céline)", items: je2 }];}
  if (niveau === "JE3MMEDOUGHTY") { return [{ title: "À acheter (J.E.3 — Mme DOUGHTY Sylvie)", items: je3 }];}
  if (niveau === "JE4MMELOURDEL") { return [{ title: "À acheter (J.E.4 — Mme LOURDEL)", items: je4 }];}
  if (niveau === "CP") { return cpSupplySections;}
  if (niveau === "CE1") { return [{ title: "À acheter (CE1)", items: ce1 }];}
  if (niveau === "CE2") { return [{ title: "À acheter (CE2)", items: ce2 }, sportEncartEcole];}
  if (niveau === "CM1") { return [{ title: "À acheter (CM1)", items: cm1 }, sportEncartEcole];}
  if (niveau === "CM2") { return [{ title: "À acheter (CM2)", items: cm2 }, sportEncartEcole];}
  return [{ title: "Liste indisponible", items: [] }];
}

export function getCollegeSuppliesLegacy(child: Extract<FournituresChild, { stage: "college" }>): SupplySection[] {
  const { niveau, langue, optionBilingueAllemand, optionLatin, optionOse, optionLceAnglais } = child;
  const itemsUSB = ["1 clé USB 16 Go (durant tout le collège)",
    "1 crayon HB",
      "Taille crayon",
      "Colle",
      "Ciseaux",
      "4 stylos (bleu, vert, noir et rouge)",
      "Règle graduée",
      "Agenda",
      "12 crayons de couleur", 
      "surligneurs fluo jaune,",
  ];
  const maths = (() => {
    const maths6e = [
      "4 surligneurs (rose, jaune, vert et bleu)",
      "2 cahiers très grand format (24 x 32) grands carreaux 96 pages",
      "1 lutin 21x 29,7 (100 vues)",
      "1 pochette à rabats avec élastiques (21x 29,7)",
      "1 pochette à rabats avec élastiques (21x 29,7) avec dedans: Copies simples grand format (21x29,7) Copies double grand format (21x29,7) Feuilles blanches format A4 (ramette de papier) Papier calque",
      "Calculatrice Casio collège",
      "Equerre transparente",
      "Compas simple avec mine supplémentaire",
      "Rapporteur gradué en degrés (transparent en forme de demi-cercle, gradué dans les 2 sens)",
      "*** Option EBP : 1 classeur fin et souple (21x 29,7) contenant 30 pochettes plastiques + 1 feutre Velléda",
    ];
    const maths5e = [
      "1 surligneur",
      "2 cahiers très grand format (24 x 32) grands carreaux 96 pages",
      "1 lutin 21x 29,7 (100 vues)",
      "1 pochette à rabats avec élastiques (21x 29,7)",
      "1 pochette à rabats avec élastiques (21x 29,7) avec dedans: Copies simples grand format (21x29,7) Copies double grand format (21x29,7) Feuilles blanches format A4 (ramette de papier) Papier calque",
      "Calculatrice Casio collège",
      "Equerre transparente",
      "Compas simple avec mine supplémentaire",
      "Rapporteur gradué en degrés (transparent en forme de demi-cercle, gradué dans les 2 sens)",
    ];
    const maths4e = [
      "1 surligneur",
      "2 cahiers très grand format (24 x 32) grands carreaux 96 pages",
      "1 lutin 21x 29,7 (100 vues)",
      "1 pochette à rabats avec élastiques (21x 29,7)",
      "1 pochette à rabats avec élastiques (21x 29,7) avec dedans: Copies simples grand format (21x29,7) Copies double grand format (21x29,7) Feuilles blanches format A4 (ramette de papier) Papier calque",
      "Calculatrice Casio collège",
      "Equerre transparente",
      "Compas simple avec mine supplémentaire",
      "Rapporteur gradué en degrés (transparent en forme de demi-cercle, gradué dans les 2 sens)",
    ];
    const maths3e = [
      "4 surligneurs (rose, jaune, vert et bleu)",
      "2 cahiers très grand format (24 x 32) grands carreaux 96 pages",
      "1 lutin 21x 29,7 (100 vues)",
      "1 pochette à rabats avec élastiques (21x 29,7)",
      "1 pochette à rabats avec élastiques (21x 29,7) contenant: Copies simples grand format (21x29,7) Copies double grand format (21x29,7) Feuilles blanches format A4 (ramette de papier)",
      "Calculatrice Casio collège",
      "Equerre transparente",
      "Compas simple avec mine supplémentaire",
      "Rapporteur gradué en degrés (transparent en forme de demi-cercle, gradué dans les 2 sens)",
    ];
    if (niveau === "6e") return maths6e;
    if (niveau === "5e") return maths5e;
    if (niveau === "4e") return maths4e;
    return maths3e;
  })();
  const anglais = [
    "1 cahier 96 pages 24/32 grands carreaux, sans spirales",
  ];
  const espagnol = [
    "1 cahier 100 pages grand format, grands carreaux",
    "1 cahier de brouillon",
    "1 cahier d’exercices fourni par le collège (facturé au 2e trimestre)",
  ];
  const espagnol3e = [
    "1 cahier 100 pages grand format, grands carreaux",
    "1 cahier de brouillon",
  ];
  const allemand = [
    "1 cahier 96 pages 24x32 grands carreaux (sans spirales)",
    "1 carnet format A5 (ou petit cahier) de 100 pages maximum, pour le vocabulaire (à conserver pendant tout le collège)",
    "Avoir toujours quelques copies simples et doublespour les évaluations",
  ];
  const seconde = langue === "Espagnol" ? (niveau === "3e" ? espagnol3e : espagnol) : allemand;
  const secondeEnNiveau6e = optionBilingueAllemand ? allemand : [];
  const artsPlastiques = [
    "Matériel de dessin (carton à dessin A3 ou demi-raisin, papiers calques A4, feuilles Canson A3 + A4)",
    "Dans une boite rigide mettre le matériel : tubes de gouache ou peinture acrylique (couleurs rouge primaire(magenta) + bleu primaire (cyan) + jaune primaire + noir + blanc)",
    "Assortiment de pinceaux souples/brosses.",
    "Quelques feutres moyens et fins, quelques crayons de couleur, règle graduée de 30 cm, crayons noirs (3H – HB – 2B – 4B), gomme, ciseaux, chiffon",
    "1 petit bidon de colle vinylique, 1 scotch papier, feutre fin noir, scotch classique",
    "Matériel marqué au nom de l’élève et de sa classe",
    "(Nouveaux élèves :1 cahier  24x32 sans spirales alternant quadrillé et dessin). Les anciens élèves peuvent continuer sur le même cahier (ou ajouter un nouveau).",
  ];
  const Technologie = [
    "1 classeur grand format (21/29,7)",
    "50 protège-documents plastifiés",
    "10 feuilles blanches papier machine (A4)",
    "10 feuilles simples quadrillées (petits carreaux)",
    "4 surligneurs et des crayons de couleurs",
    "Les cours de 5e et 4e doivent être conservés pour les années suivantes",
  ];
  const vieTerre = [
    "1 cahier 24/32 de 96 pages grands carreaux",
    "Dans une pochette cartonnée à élastiques : feuilles ordinateur (21x29,7) + pochettes plastiques perforées + copies simples et doubles",
    "1 boîte/sachet de gants latex",
  ];
  const physique = (() => {
    switch (niveau) {
      case "6e":
        return ["1 classeur"];
      case "5e":
        return [
          "1 classeur grand format 21/29,7 (50 protège-documents plastifiées par années)",
          "10 feuilles blanches papier machine (A4) + 10 feuilles simples quadrillées (petits carreaux)",
          "Peut servir tout le collège",
        ];
      case "4e":
        return [
          "1 classeur grand format 21/29,7 (50 protège-documents plastifiées par années)",
          "10 feuilles blanches papier machine (A4) + 10 feuilles simples quadrillées (petits carreaux)",
          "Peut servir tout le collège",
        ];
      case "3e":
        return [
          "1 classeur grand format 21/29,7 (50 protège-documents plastifiées par années)",
          "10 feuilles blanches papier machine (A4) + 10 feuilles simples quadrillées (petits carreaux)",
          "Une blouse blanche en coton, prévoir la taille afin qu'elle soit réutilisée au lycée",
        ];
      default:
        return [
          ""
        ];
    }
  })();
  const histoireGeo = (() => {
    if (niveau === "4e") {
      return [
        "2 cahiers très grand format (24/32) de 96 pages grands carreaux",
      ];
    }
    if (niveau === "3e") {
      return [
        "AU CHOIX DE L'ÉLÈVE 2 cahiers grands carreaux 24x32 OU 1 classeur (Copies simples et doubles grands carreaux)",
        "Copies simples et doubles grands carreaux",
      ];
    }
    return [
      "2 cahiers très grand format (24/32) de 96 pages grands carreaux",
      "Livret fourni par le collège (facturé sur le 2e trimestre)",
    ];
  })();
  const catechese = (() => {
    if (niveau === "6e" || niveau === "5e") {
      return ["1 cahier 96 pages très grand format (24/32) grands carreaux"];
    }
    return ["Pas de cahier : lutin 100 vues minimum"];
  })();
  const eps = [
    "Chaussures extérieures + chaussures de salle (obligatoire)",
    "Tenue adaptée au sport (chaussettes, tee-shirt, short au minimum)",
    "Prévoir en fonction de la météo",
    "1 gourde (pas de bouteille d’eau)",
  ];
  const musique = [
    "Cahier de musique et chant (21/29,7) + protège-cahier",
    "Chemise à élastique (21/29,7) pour ranger le matériel de musique",
    "Documents divers + MémoArts (selon consignes du professeur).",
  ];
  const latin = [
    "1 cahier 96 pages grand format (grands carreaux) le cahier peut être réutilisé s'il reste de la place",
    "Des grandes copies simples ou doubles"
  ];
  const ose = [
    "Utiliser le classeur de Technologie pour classer les documents OSE.",
    "30 pochettes transparentes.",
  ];
  const lceAnglais = [
    "1 cahier 24x32 grands carreaux",
  ];
  const francais = (() => {
    const fr6e = [
      "1 cahier grand format 21x29,7 grands carreaux sans spirales",
      "1 cahier de brouillon",
      "Copies simples et doubles grands carreaux pour les évaluations",
      "Dictionnaire (Larousse de préférence)",
    ];
    const fr5e = [
      "1 cahier grand format 21x29,7 grands carreaux sans spirales",
      "Copies simples et doubles grands carreaux pour les évaluations",
    ];
    const fr4e = [
      "1 cahier grand format 21x29,7 grands carreaux sans spirales",
      "Copies simples et doubles grands carreaux pour les évaluations",
    ];
    const fr3e = [
      "1 cahier grand format 21x29,7 grands carreaux sans spirales",
      "Copies simples et doubles grands carreaux pour les évaluations"
    ];
    if (niveau === "6e") return fr6e;
    if (niveau === "5e") return fr5e;
    if (niveau === "4e") return fr4e;
    if (niveau === "3e") return fr3e;
    return [];
  })();
  return [
    { title: "À préparer (commun)", items: itemsUSB },
    { title: "Mathématiques", items: maths },
    { title: "Français", items: francais.filter(Boolean) },
    { title: "Histoire / Géographie", items: histoireGeo },
    { title: "Anglais (LV1) — obligatoire", items: anglais },
    ...(niveau === "6e"
      ? optionBilingueAllemand
        ? [{ title: "Option bilingue : Allemand", items: secondeEnNiveau6e }]
        : []
      : [{ title: `Langue (LV2) : ${langue}`, items: seconde }]),
    { title: "Arts plastiques", items: artsPlastiques },
    ...(niveau === "6e" ? [] : [{ title: "Technologie", items: Technologie }]),
    { title: "Sciences (Vie & Terre)", items: vieTerre },
    { title: "Sciences (Physiques)", items: physique },
    { title: "EPS", items: eps },
    { title: "Musique", items: musique },
    { title: "Catéchèse", items: catechese },
    ...(niveau !== "6e" && optionLatin ? [{ title: "Latin (option)", items: latin }] : []),
    ...((niveau === "5e" || niveau === "4e" || niveau === "3e") && optionLceAnglais ? [{ title: "LCE Anglais (option)", items: lceAnglais }] : []),
    ...(niveau === "3e" && optionOse ? [{ title: "Option OSE", items: ose }] : []),
  ];
}

/** Référence manuel lycée pour achat libre : ISBN — éditeur titre — édition — classe */
function lyceeManuel(isbn: string, editeur: string, titre: string, edition: string, classe: string): string {
  return `${isbn} — ${editeur} ${titre} — ${edition} — ${classe}`;
}

const LYCEE_MANUELS = {
  latin: lyceeManuel(
    "9782012814080",
    "HACHETTE",
    "DICTIONNAIRE LATIN - FRANÇAIS (GAFFIOT POCHE TOP)",
    "2008",
    "MULTIPLE",
  ),
  conjugaisonEspagnole: lyceeManuel(
    "9782011252036",
    "HACHETTE",
    "LA CONJUGAISON ESPAGNOLE",
    "1999",
    "MULTIPLE",
  ),
  sectionEuroAnglais: lyceeManuel(
    "9782091520117",
    "NATHAN",
    "ROBERT & NATHAN VOCABULAIRE DE L'ANGLAIS",
    "2018",
    "MULTIPLE",
  ),
} as const;

export function getLyceeSuppliesLegacy(child: Extract<FournituresChild, { stage: "lycee" }>): SupplySection[] {
  const {
    niveau,
    track,
    langue,
    optionSectionEuropeenne = false,
    specialites,
    latin,
    options = [],
  } = child;

  const withLyceePreamble = (sections: SupplySection[]): SupplySection[] => [
    {
      title: "Fournitures — au choix de l'élève",
      items: [
        "Aucune fourniture imposée (cahiers, classeurs, trousses, etc.) : chaque lycéen organise son matériel comme il le souhaite.",
        "Seuls les manuels scolaires listés ci-dessous sont à prévoir.",
      ],
    },
    ...sections,
    ...(optionSectionEuropeenne
      ? [{ title: "Section européenne", items: lyceeSectionEuropeenneManuals(niveau) }]
      : []),
  ];

  /* ── 2nde ── */
  if (niveau === "2nde") {
    const sections: SupplySection[] = [
      {
        title: "Matières obligatoires",
        items: [
          lyceeManuel("9782013954723", "HACHETTE", "PHYSIQUE - CHIMIE", "2019", "2°"),
          lyceeManuel("9782401085886", "HATIER", "SCIENCES ECONOMIQUES ET SOCIALES", "2022", "2°"),
          lyceeManuel("9782017149507", "HACHETTE", "SKYWALKS", "2025", "2°"),
          lyceeManuel("9782210111653", "MAGNARD", "MATHS", "2019", "2°"),
          lyceeManuel("9782013954730", "HACHETTE", "SCIENCE DE LA VIE ET DE LA TERRE", "2019", "2°"),
          lyceeManuel("9782401046023", "HATIER", "HISTOIRE - GEOGRAPHIE", "2019", "2°"),
        ],
      },
    ];
    if (langue === "Allemand") {
      sections.push({
        title: "LV2 — Allemand",
        items: [lyceeManuel("9782356858269", "LA MAISON DES LANGUE", "FANTASTICH", "2025", "2°")],
      });
    } else {
      sections.push({
        title: "LV2 — Espagnol",
        items: [LYCEE_MANUELS.conjugaisonEspagnole],
      });
    }
    if (latin) {
      sections.push({ title: "Option — Latin", items: [LYCEE_MANUELS.latin] });
    }
    return withLyceePreamble(sections);
  }

  /* ── 1re Général ── */
  if (niveau === "1re" && track === "General") {
    const sections: SupplySection[] = [
      {
        title: "Matières obligatoires",
        items: [
          lyceeManuel("9782356855497", "LA MAISON DES LANGUE", "BLOCKBUSTER", "2019", "1°"),
          lyceeManuel("9782401053793", "HATIER", "HISTOIRE - GEOGRAPHIE", "2019", "1°"),
          lyceeManuel("9782017227236", "HACHETTE", "ENSEIGNEMENT SCIENTIFIQUE", "2024", "1°"),
        ],
      },
    ];
    if (!specialites.includes("Maths")) {
      sections.push({
        title: "Mathématiques (hors spécialité)",
        items: [lyceeManuel("9782017866367", "HACHETTE", "DECLIC", "2023", "1°")],
      });
    }
    if (langue === "Allemand") {
      sections.push({
        title: "LV2 — Allemand",
        items: [lyceeManuel("9782047409558", "BORDAS", "WANDERLUST NEU", "2026", "1°")],
      });
    } else {
      sections.push({
        title: "LV2 — Espagnol",
        items: [LYCEE_MANUELS.conjugaisonEspagnole],
      });
    }
    sections.push(...lyceeGeneralSpecialiteSections(specialites, "1re"));
    if (latin) {
      sections.push({ title: "Option — Latin", items: [LYCEE_MANUELS.latin] });
    }
    return withLyceePreamble(sections);
  }

  /* ── Terminale Général ── */
  if (niveau === "Terminale" && track === "General") {
    const sections: SupplySection[] = [
      {
        title: "Matières obligatoires",
        items: [
          lyceeManuel("9782210114173", "MAGNARD", "PHILOSOPHIE", "2020", "TERMINALE"),
          lyceeManuel("9782017866138", "HACHETTE", "ENSEIGNEMENT SCIENTIFIQUE", "2020", "TERMINALE"),
          lyceeManuel("9782401062962", "HATIER", "HISTOIRE", "2020", "TERMINALE"),
          lyceeManuel("9782401062900", "HATIER", "GEOGRAPHIE", "2020", "TERMINALE"),
          lyceeManuel("9782356856418", "LA MAISON DES LANGUE", "BLOCKBUSTER", "2020", "TERMINALE"),
        ],
      },
    ];
    if (langue === "Allemand") {
      sections.push({
        title: "LV2 — Allemand",
        items: [lyceeManuel("9782047409572", "BORDAS", "WANDERLUST NEU", "2026", "TERMINALE")],
      });
    } else {
      sections.push({
        title: "LV2 — Espagnol",
        items: [
          lyceeManuel("9782401062993", "HATIER", "VIA LIBRE", "2020", "TERMINALE"),
          LYCEE_MANUELS.conjugaisonEspagnole,
        ],
      });
    }
    sections.push(...lyceeGeneralSpecialiteSections(specialites, "Terminale"));
    if (options.includes("Maths Complémentaires")) {
      sections.push({
        title: "Option — Maths complémentaires",
        items: [
          lyceeManuel("9782091728957", "NATHAN", "HYPERBOLE - MATHS COMPLEMENTAIRES", "2020", "TER OPTION"),
        ],
      });
    }
    if (options.includes("Maths Expertes")) {
      sections.push({
        title: "Option — Maths expertes",
        items: [
          lyceeManuel("9782017866213", "HACHETTE", "MATHEMATIQUES - BARBAZO EXPERTES", "2020", "TER OPTION"),
        ],
      });
    }
    if (latin) {
      sections.push({ title: "Option — Latin", items: [LYCEE_MANUELS.latin] });
    }
    return withLyceePreamble(sections);
  }

  /* ── 1re ST2S ── */
  if (niveau === "1re" && track === "ST2S") {
    const sections: SupplySection[] = [
      {
        title: "Matières obligatoires",
        items: [
          lyceeManuel(
            "9782091653747",
            "NATHAN",
            "I-MANUEL PHYSIQUE CHIMIE POUR LA SANTE (LIVRE + LICENCE IMANUEL)",
            "2019",
            "1°ST2S",
          ),
          lyceeManuel("9782356855497", "LA MAISON DES LANGUE", "BLOCKBUSTER", "2019", "1°"),
          lyceeManuel(
            "9782206103457",
            "DELAGRAVE",
            "BIOLOGIE ET PHYSIOPATHOLOGIE HUMAINES",
            "2019",
            "1°ST2S",
          ),
          lyceeManuel("9782017041887", "HACHETTE", "HISTOIRE - GEOGRAPHIE - EMC", "2019", "1° TECH"),
          lyceeManuel(
            "9782017317005",
            "HACHETTE",
            "LES FONDAMENTAUX DU LYCEE - MATHS (Enseignement commun)",
            "2025",
            "1°STMG-STHR-ST",
          ),
        ],
      },
    ];
    if (langue === "Allemand") {
      sections.push({
        title: "LV2 — Allemand",
        items: [lyceeManuel("9782047409558", "BORDAS", "WANDERLUST NEU", "2026", "1°")],
      });
    } else {
      sections.push({
        title: "LV2 — Espagnol",
        items: [
          lyceeManuel("9782401054097", "HATIER", "VIA LIBRE", "2019", "1°"),
          LYCEE_MANUELS.conjugaisonEspagnole,
        ],
      });
    }
    return withLyceePreamble(sections);
  }

  /* ── Terminale ST2S ── */
  if (niveau === "Terminale" && track === "ST2S") {
    const sections: SupplySection[] = [
      {
        title: "Matières obligatoires",
        items: [
          lyceeManuel("9782356856418", "LA MAISON DES LANGUE", "BLOCKBUSTER", "2020", "TERMINALE"),
          lyceeManuel("9782210110922", "MAGNARD", "PHILOSOPHIE", "2020", "TER TECH"),
          lyceeManuel(
            "9782091670829",
            "NATHAN",
            "I-MANUEL HISTOIRE - GEOGRAPHIE - EMC (LIVRE + LICENCE IMANUEL)",
            "2020",
            "TER TECH",
          ),
        ],
      },
    ];
    if (langue === "Allemand") {
      sections.push({
        title: "LV2 — Allemand",
        items: [lyceeManuel("9782047409572", "BORDAS", "WANDERLUST NEU", "2026", "TERMINALE")],
      });
    } else {
      sections.push({
        title: "LV2 — Espagnol",
        items: [
          lyceeManuel("9782401062993", "HATIER", "VIA LIBRE", "2020", "TERMINALE"),
          LYCEE_MANUELS.conjugaisonEspagnole,
        ],
      });
    }
    sections.push({
      title: "Spécialité — Biologie & Physiopathologie",
      items: [
        lyceeManuel(
          "9782206104485",
          "DELAGRAVE",
          "BIOLOGIE ET PHYSIOPATHOLOGIE HUMAINES",
          "2020",
          "TER ST2S",
        ),
      ],
    });
    sections.push({
      title: "Spécialité — Chimie",
      items: [
        lyceeManuel(
          "9782091670881",
          "NATHAN",
          "I-MANUEL CHIMIE (LIVRE + LICENCE IMANUEL)",
          "2020",
          "TER ST2S",
        ),
      ],
    });
    return withLyceePreamble(sections);
  }

  return withLyceePreamble([{ title: "À compléter", items: ["Liste non disponible pour ce cas."] }]);
}

/** Manuels dédiés à la section européenne (anglais section Europe). */
function lyceeSectionEuropeenneManuals(niveau: import("./fournitures-types").LyceeNiveau): string[] {
  if (niveau === "2nde") return [LYCEE_MANUELS.sectionEuroAnglais];
  return ["Pas de livre requis pour la section européenne"];
}

function lyceeGeneralSpecialiteSections(
  specialites: import("./fournitures-types").LyceeSpecialite[],
  niveau: "1re" | "Terminale",
): SupplySection[] {
  const sections: SupplySection[] = [];
  const is1re = niveau === "1re";

  if (specialites.includes("Maths")) {
    sections.push({
      title: "Spécialité — Maths",
      items: [
        is1re
          ? lyceeManuel("9782013954891", "HACHETTE", "DECLIC", "2019", "1° SPECIALITE")
          : lyceeManuel("9782091728919", "NATHAN", "HYPERBOLE - MATHS SPECIALITE", "2020", "TER SPECIALITE"),
      ],
    });
  }
  if (specialites.includes("Physique-Chimie")) {
    sections.push({
      title: "Spécialité — Physique-Chimie",
      items: [
        is1re
          ? lyceeManuel("9782401053977", "HATIER", "PHYSIQUE CHIMIE", "2019", "1° SPECIALITE")
          : lyceeManuel("9782017866091", "HACHETTE", "PHYSIQUE CHIMIE", "2020", "TER SPECIALITE"),
      ],
    });
  }
  if (specialites.includes("SVT")) {
    sections.push({
      title: "Spécialité — SVT",
      items: [
        is1re
          ? lyceeManuel(
              "9782047336335",
              "BORDAS",
              "SCIENCES DE LA VIE ET DE LA TERRE",
              "2019",
              "1° SPECIALITE",
            )
          : lyceeManuel(
              "9782047337622",
              "BORDAS",
              "SCIENCES DE LA VIE ET DE LA TERRE",
              "2020",
              "TER SPECIALITE",
            ),
      ],
    });
  }
  if (specialites.includes("SES")) {
    sections.push({
      title: "Spécialité — SES",
      items: [
        is1re
          ? lyceeManuel(
              "9782401097117",
              "HATIER",
              "SCIENCES ECONOMIQUES ET SOCIALES",
              "2023",
              "1° SPECIALITE",
            )
          : lyceeManuel(
              "9782017257066",
              "HACHETTE",
              "SCIENCES ECONOMIQUES ET SOCIALES",
              "2024",
              "TER SPECIALITE",
            ),
      ],
    });
  }
  if (specialites.includes("HG-GEO-GEOPOL")) {
    sections.push({
      title: "Spécialité — Histoire-Géographie-Géopolitique",
      items: [
        is1re
          ? lyceeManuel(
              "9782091728766",
              "NATHAN",
              "HISTOIRE - GEOGRAPHIE - GEOPOLITIQUE - SCIENCES POLITIQUES",
              "2019",
              "1° SPECIALITE",
            )
          : lyceeManuel(
              "9782401062917",
              "HATIER",
              "HISTOIRE - GEOGRAPHIE - GEOPOLITIQUE - SCIENCES POLITIQUES",
              "2020",
              "TER SPECIALITE",
            ),
      ],
    });
  }
  if (is1re && specialites.includes("HLP")) {
    sections.push({
      title: "Spécialité — Humanités, littérature et philosophie",
      items: ["Pas de livre requis pour cette spécialité"],
    });
  }
  if (is1re && specialites.includes("LLCE")) {
    sections.push({
      title: "Spécialité — Langue, littérature et culture étrangère",
      items: ["Pas de livre requis pour cette spécialité"],
    });
  }
  if (specialites.includes("Sciences-de-l-Ingenieur")) {
    sections.push({
      title: "Spécialité — Sciences physiques / Sciences de l'ingénieur",
      items: [
        is1re
          ? "Pas de livre requis pour cette spécialité"
          : lyceeManuel("9782017866244", "HACHETTE", "SCIENCES PHYSIQUES", "2020", "TER SPE SI"),
      ],
    });
  }
  return sections;
}

function applyProfileOverride(
  profileId: string,
  legacy: FournituresSection[],
  overrides?: FournituresProfileOverrides,
): FournituresSection[] {
  const legacyProfileId = profileId === "ecole:JE4MMELOURDEL" ? "ecole:JE4" : null;
  const custom = overrides?.[profileId] ?? (legacyProfileId ? overrides?.[legacyProfileId] : undefined);
  if (custom && custom.length > 0) return custom.map((s) => ({ ...s, items: [...s.items] }));
  return legacy.map((s) => ({ ...s, items: [...s.items] }));
}

export function ecoleProfileId(niveau: import("./fournitures-types").EcoleNiveau): string {
  return `ecole:${niveau}`;
}

export function collegeProfileId(niveau: import("./fournitures-types").CollegeNiveau): string {
  return `college:${niveau}`;
}

export function lyceeProfileId(child: Extract<FournituresChild, { stage: "lycee" }>): string {
  const { niveau, track } = child;
  if (niveau === "2nde") return "lycee:2nde";
  if (track === "ST2S") return niveau === "1re" ? "lycee:1re-st2s" : "lycee:terminale-st2s";
  return niveau === "1re" ? "lycee:1re-general" : "lycee:terminale-general";
}

function getEcoleSupplies(
  niveau: import("./fournitures-types").EcoleNiveau,
  overrides?: FournituresProfileOverrides,
): FournituresSection[] {
  return applyProfileOverride(ecoleProfileId(niveau), getEcoleSuppliesLegacy(niveau), overrides);
}

function getCollegeSupplies(
  child: Extract<FournituresChild, { stage: "college" }>,
  overrides?: FournituresProfileOverrides,
): FournituresSection[] {
  const legacy = getCollegeSuppliesLegacy(child);
  const baseId = collegeProfileId(child.niveau);
  if (overrides?.[baseId]?.length) {
    const baseTitles = new Set(overrides[baseId].map((s) => s.title));
    const extras = legacy.filter((s) => !baseTitles.has(s.title));
    return [...applyProfileOverride(baseId, [], overrides), ...extras];
  }
  return legacy.map((section) => {
    const sectionId = `${baseId}:${section.title}`;
    const custom = overrides?.[sectionId];
    if (custom?.[0]) return { ...custom[0], items: [...custom[0].items] };
    return { ...section, items: [...section.items] };
  });
}

function getLyceeSupplies(
  child: Extract<FournituresChild, { stage: "lycee" }>,
  overrides?: FournituresProfileOverrides,
): FournituresSection[] {
  const legacy = getLyceeSuppliesLegacy(child);
  const profileId = lyceeProfileId(child);
  if (overrides?.[profileId]?.length) {
    const baseTitles = new Set(overrides[profileId].map((s) => s.title));
    const extras = legacy.filter((s) => !baseTitles.has(s.title));
    return [...applyProfileOverride(profileId, [], overrides), ...extras];
  }
  return legacy.map((section) => {
    const sectionId = `${profileId}:${section.title}`;
    const custom = overrides?.[sectionId];
    if (custom?.[0]) return { ...custom[0], items: [...custom[0].items] };
    return { ...section, items: [...section.items] };
  });
}

export function getChildSupplies(child: FournituresChild, overrides?: FournituresProfileOverrides): FournituresSection[] {
  if (child.stage === "ecole") return getEcoleSupplies(child.niveau, overrides);
  if (child.stage === "college") return getCollegeSupplies(child, overrides);
  return getLyceeSupplies(child, overrides);
}

export function formatChildLabel(child: FournituresChild): string {
  if (child.stage === "ecole") {
    const ecoleLabels: Record<import("./fournitures-types").EcoleNiveau, string> = {
      JE1MMEBAYEL: "J.E.1 (Mme BAYEL Christine)",
      JE2MMECARTIER: "J.E.2 (Mme CARTIER Céline)",
      JE3MMEDOUGHTY: "J.E.3 (Mme DOUGHTY Sylvie)",
      JE4MMELOURDEL: "J.E.4 (Mme LOURDEL)",
      CP: "CP",
      CE1: "CE1",
      CE2: "CE2",
      CM1: "CM1",
      CM2: "CM2",
    };
    return `École — ${ecoleLabels[child.niveau] ?? child.niveau}`;
  }
  if (child.stage === "college") {
    if (child.niveau === "6e") return `Collège — 6e (bilingue allemand: ${child.optionBilingueAllemand ? "oui" : "non"})`;
    return `Collège — ${child.niveau} (${child.langue}${child.optionLatin ? " • Latin" : ""}${child.optionOse ? " • OSE" : ""}${child.optionLceAnglais ? " • LCE Anglais" : ""})`;
  }
  return `Lycée — ${child.niveau} (${child.track === "ST2S" ? "ST2S" : "Général"} • LV2 ${child.langue}${child.optionSectionEuropeenne ? " • Section européenne" : ""}${child.latin ? " • Latin" : ""})`;
}

function slugifyFilenamePart(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
}

/** Nom de fichier PDF dérivé de la sélection (ex. college_6e_option_bilingue). */
function formatChildFilenameBase(child: FournituresChild): string {
  if (child.stage === "ecole") {
    const niveauSlug: Record<EcoleNiveau, string> = {
      JE1MMEBAYEL: "je1",
      JE2MMECARTIER: "je2",
      JE3MMEDOUGHTY: "je3",
      JE4MMELOURDEL: "je4_mme_lourdel",
      CP: "cp",
      CE1: "ce1",
      CE2: "ce2",
      CM1: "cm1",
      CM2: "cm2",
    };
    return `ecole_${niveauSlug[child.niveau] ?? slugifyFilenamePart(child.niveau)}`;
  }

  if (child.stage === "college") {
    const parts = ["college", child.niveau];
    if (child.niveau === "6e") {
      if (child.optionBilingueAllemand) parts.push("option_bilingue");
    } else {
      parts.push(slugifyFilenamePart(child.langue));
      if (child.optionLatin) parts.push("latin");
      if (child.optionOse) parts.push("ose");
      if (child.optionLceAnglais) parts.push("lce_anglais");
    }
    return parts.join("_");
  }

  const parts = ["lycee", slugifyFilenamePart(child.niveau), child.track === "ST2S" ? "st2s" : "general"];
  parts.push(slugifyFilenamePart(child.langue));
  if (child.optionSectionEuropeenne) parts.push("section_europeenne");
  if (child.latin) parts.push("latin");
  for (const spec of child.specialites) parts.push(slugifyFilenamePart(spec));
  for (const opt of child.options) parts.push(slugifyFilenamePart(opt));
  return parts.join("_");
}

export function formatSuppliesPdfFilename(children: FournituresChild[]): string {
  const bases = children.map(formatChildFilenameBase).filter(Boolean);
  if (bases.length === 0) return "liste_fournitures.pdf";
  if (bases.length === 1) return `${bases[0]}.pdf`;
  return `liste_fournitures_${bases.join("__")}.pdf`;
}

export function dedupeStrings(items: string[]) {
  const set = new Set(items.map((s) => s.trim()).filter(Boolean));
  return Array.from(set);
}
