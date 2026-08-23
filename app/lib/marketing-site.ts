/** Contenu éditorial du site commercial ScolIA — School + IA. */
export const MARKETING = {
  productName: "ScolIA",
  productNameExplanation: "School + IA",
  tagline: "Intranet pour établissements scolaires",
  contactEmail: process.env.NEXT_PUBLIC_SCOLA_CONTACT_EMAIL?.trim() || "contact@scolia.fr",
  contactCtaLabel: "Nous contacter",
  pricingPromise:
    "Tarif fondateur pour les premiers établissements : le palier se calcule selon votre effectif, avec le pack Microsoft adapté.",

  legal: {
    companyName: process.env.NEXT_PUBLIC_SCOLA_LEGAL_COMPANY?.trim() || "[Raison sociale à compléter]",
    legalForm: "SAS",
    shareCapital: "[Capital social]",
    address: "[Adresse du siège social]",
    rcs: "[Ville] RCS [n°]",
    siret: "[SIRET]",
    vat: "[TVA intracommunautaire]",
    publisherName: "[Directeur / directrice de la publication]",
    hostName: "Scaleway",
    hostRegion: "France — Paris (fr-par)",
    hostAddress: "Scaleway SAS, 8 rue de la Ville l’Évêque, 75008 Paris",
    dpoEmail: process.env.NEXT_PUBLIC_SCOLA_DPO_EMAIL?.trim() || "dpo@scolia.fr",
  },
} as const;

export const TRUST_STRIP = [
  { label: "IA française", detail: "Mistral — OCR & aide" },
  { label: "Serveurs France", detail: "Scaleway Paris · RGPD" },
  { label: "Stack FR", detail: "OVH · EasyTransac" },
  { label: "Microsoft 365", detail: "Vos habitudes, intactes" },
] as const;

export const BENEFITS = [
  {
    title: "Gain de temps",
    desc: "L’IA classe, les parcours valident : moins de double saisie, moins de relances.",
  },
  {
    title: "Économie de papier",
    desc: "Dossiers numériques, moins de classeurs, moins d’archives à manipuler.",
  },
  {
    title: "Meilleure communication",
    desc: "Profs, direction, compta, familles : tout le monde voit le même statut.",
  },
] as const;

/** Cinq piliers — hooks courts + bénéfices. */
export const KEY_PILLARS = [
  {
    id: "docs",
    title: "Documents élèves",
    hook: "L’IA range. Vous respirez.",
    accent: "#2F6B4A",
    lead: "Un lot de bulletins ou un relevé du bac : dépôt, OCR Mistral, classement OneDrive.",
    outcomes: [
      "Moins de papier, plus de dossiers à jour",
      "Gros gain de temps pour l’administratif",
      "Plus de tri manuel dans les classeurs",
    ],
  },
  {
    id: "travels",
    title: "Sorties scolaires",
    hook: "Tout le monde sur la même page.",
    accent: "#234B73",
    lead: "Devis signés, onglet compta, validations direction — sans fil d’e-mails.",
    outcomes: [
      "Meilleure communication école ↔ familles ↔ bus",
      "Signature de devis et suivi compta intégrés",
      "Statut clair pour chaque sortie",
    ],
  },
  {
    id: "rooms",
    title: "Réservation de salles",
    hook: "Une grille. Zéro conflit.",
    accent: "#4C3D7A",
    lead: "Planning partagé en temps réel — fini le tableur qui diverge.",
    outcomes: [
      "Disponibilités visibles pour toute l’équipe",
      "Réservation en quelques clics",
      "Moins de doubles bookings",
    ],
  },
  {
    id: "rh",
    title: "Ressources humaines",
    hook: "Absences et arrivées, au même endroit.",
    accent: "#6B3A4A",
    lead: "Se déclarer absent, et aussi accueillir un nouveau salarié — vrai module RH.",
    outcomes: [
      "Déclaration d’absence simple",
      "Onboarding nouveaux collaborateurs",
      "Dossiers et validations selon les droits",
    ],
  },
  {
    id: "tickets",
    title: "Ticketing",
    hook: "Une demande. Le bon service. Un suivi.",
    accent: "#8B5E34",
    lead: "Maintenance, admin, vie scolaire : ticket créé, routé, traité en Kanban — fini les mails perdus.",
    outcomes: [
      "Routage vers la bonne corbeille",
      "Suivi visible pour le demandeur",
      "Moins de relances orales et de doublons",
    ],
  },
] as const;

/** Compléments (hors les piliers principaux). */
const REST_CAPABILITIES = [
  {
    title: "Cloud personnel",
    desc: "Fichiers de travail et partages au quotidien — sans USB ni boîtes mail saturées.",
  },
  {
    title: "Internat",
    desc: "Appels, sorties et suivi de vie d’internat dans le même abonnement.",
  },
] as const;

export const PLATFORM_CAPABILITIES = [
  {
    title: "Documents élèves",
    desc: "IA : dépôt → classement auto. Moins de papier.",
  },
  {
    title: "Sorties scolaires",
    desc: "Devis signés, compta, communication unifiée.",
  },
  {
    title: "Réservation de salles",
    desc: "Planning live, sans Excel.",
  },
  {
    title: "Ressources humaines",
    desc: "Absences + onboarding nouveaux salariés.",
  },
  {
    title: "Ticketing",
    desc: "Demandes, corbeilles et suivi Kanban.",
  },
  {
    title: "Licences Microsoft",
    desc: "A1 / A3 Éducation selon l’effectif.",
  },
] as const;

const AUDIENCES = [
  {
    title: "Direction & OGEC",
    desc: "Pilotage, validations, sorties, RH et vision d'ensemble.",
  },
  {
    title: "Administratif & comptabilité",
    desc: "Corbeilles, dossiers, suivi financier et traitements du quotidien.",
  },
  {
    title: "Maintenance",
    desc: "Demandes techniques, tickets et suivi des interventions.",
  },
  {
    title: "Enseignants & vie scolaire",
    desc: "Documents, salles, sorties et absences — bureau ou mobile.",
  },
] as const;

/** Types d'établissements cibles — page d'accueil. */
export const ESTABLISHMENT_TARGETS = [
  {
    id: "ecole",
    title: "Écoles",
    desc: "Primaire : documents élèves, absences, salles et sorties du quotidien.",
  },
  {
    id: "college",
    title: "Collèges",
    desc: "Workflows métier pour la vie scolaire, les voyages et le suivi RH.",
  },
  {
    id: "lycee",
    title: "Lycées",
    desc: "Stages, conventions, sorties, dossiers élèves et organisation interne.",
  },
  {
    id: "groupe",
    title: "Groupes scolaires & OGEC",
    desc: "Un intranet unique pour plusieurs établissements sous la même tutelle.",
  },
] as const;

export const PARTNERS = [
  {
    id: "scaleway",
    name: "Scaleway",
    role: "Hébergement",
    detail: "Cloud souverain français — application et données hébergées à Paris (fr-par).",
    logoPath: "/partners/scaleway.svg",
    sovereign: true,
  },
  {
    id: "mistral",
    name: "Mistral AI",
    role: "IA & OCR",
    detail: "Intelligence artificielle française — assistant, analyse documentaire et OCR.",
    logoPath: "/partners/mistral.svg",
    sovereign: true,
  },
  {
    id: "easytransac",
    name: "EasyTransac",
    role: "Paiement",
    detail: "Startup française — encaissement des mensualités d'abonnement.",
    logoPath: "/partners/easytransac.png",
    sovereign: true,
  },
  {
    id: "ovh",
    name: "OVH",
    role: "Messagerie",
    detail: "Messagerie professionnelle via OVH — e-mails de l'établissement gérés en France.",
    logoPath: "/partners/ovh.svg",
    sovereign: true,
  },
  {
    id: "microsoft",
    name: "Microsoft Éducation",
    role: "Licences bureautique",
    detail: "Membre du Partner Program — packs A1 / A3 selon effectif.",
    logoPath: "/partners/microsoft.svg",
    sovereign: false,
  },
] as const;

/** Message souveraineté — page d'accueil & tarifs. */
export const SOVEREIGNTY = {
  title: "Souveraineté numérique",
  intro:
    "ScolIA s'appuie prioritairement sur des acteurs français : hébergement Scaleway, intelligence artificielle Mistral, paiements EasyTransac, messagerie OVH. Authentification Better-Auth (sessions en France). Hors France : Microsoft Éducation (licences).",
  bullets: [
    "Scaleway — cloud français, hébergement à Paris.",
    "Mistral AI — IA française pour l'assistant, l'analyse et l'OCR.",
    "EasyTransac — startup française pour les mensualités.",
    "OVH — messagerie professionnelle gérée en France.",
    "Licences Microsoft Éducation · authentification Better-Auth.",
  ],
} as const;

export const RGPD_COMPACT = {
  title: "Données & confiance",
  summary:
    "ScolIA orchestre les workflows de l'établissement. Hébergement Scaleway en France, IA française Mistral, paiements EasyTransac, messagerie OVH, licences Microsoft Éducation dans l'abonnement.",
  bullets: [
    "Les dossiers élèves sensibles sont orientés vers vos espaces Microsoft : l'établissement reste maître de ses données.",
    "Hébergement France (Scaleway) · IA française (Mistral) · paiement français (EasyTransac) · mail OVH.",
    "Licences Microsoft A1 / A3 Éducation selon effectif · authentification Better-Auth.",
  ],
} as const;

export const RGPD_HIGHLIGHTS = {
  title: "Données & RGPD",
  intro:
    "ScolIA est une plateforme de traitement et d'orchestration des workflows. L'établissement conserve la responsabilité sur ses données sensibles.",
  processingModel:
    "ScolIA fait circuler, valider et tracer les démarches. Pour les dossiers élèves et contenus sensibles, des workflows orientent le dépôt vers Microsoft 365. L'établissement reste responsable de traitement.",
  microsoftPartner:
    "ScolIA est membre du Microsoft Partner Program : licences A1 et A3 Éducation incluses selon l'effectif, pour équiper direction, administratif et enseignants sans passer par des licences Business.",
  points: [
    {
      label: "Traitement, pas archivage",
      detail: "Workflows métier plutôt qu'une plateforme de stockage généraliste.",
    },
    {
      label: "Hébergement en France",
      detail: "Scaleway — infrastructure cloud française, région Paris (fr-par).",
    },
    {
      label: "IA française",
      detail: "Mistral AI : assistant, aide documentaire et OCR — entreprise française.",
    },
    {
      label: "Paiement français",
      detail: "EasyTransac, startup française, pour les mensualités d'abonnement.",
    },
    {
      label: "Messagerie OVH",
      detail: "E-mails de l'établissement gérés via OVH, en France.",
    },
    {
      label: "Microsoft Éducation",
      detail: "Licences A1 / A3 incluses selon l'effectif.",
    },
    {
      label: "Authentification",
      detail: "Better-Auth — sessions hébergées en France (PostgreSQL).",
    },
  ],
  reassurance:
    "Droits RGPD exercables auprès de notre référent données. Détails des sous-traitants dans les mentions légales.",
} as const;

const TRUST_ITEMS = RGPD_COMPACT.bullets.map((detail, i) => ({
  label: ["Vos données", "Stack française", "Microsoft & auth"][i] ?? "Confiance",
  detail,
}));

export const STATS = [
  { value: "4", label: "piliers métier" },
  { value: "1", label: "tarif selon effectif" },
  { value: "FR", label: "Scaleway · Mistral · OVH" },
  { value: "MS", label: "licences incluses" },
] as const;

type PricingPlan = {
  id: string;
  name: string;
  /** Libellé effectif (ex. Moins de 500 élèves). */
  audienceLabel: string;
  priceMonthly: number;
  priceLabel: string;
  priceHint: string;
  description: string;
  microsoftA3: number;
  microsoftA1: number;
  features: string[];
};

const BASE_FEATURES = [
  "Les 4 piliers : documents élèves, sorties, salles, RH",
  "Internat, cloud personnel et système de demandes",
  "Assistant IA Mistral (OCR & aide documentaire)",
  "Hébergement France — Scaleway Paris",
  "Messagerie professionnelle — OVH",
  "Personnalisation logo & identité",
  "Mises à jour et accompagnement à la prise en main",
] as const;

export const PRICING_INCLUDED = [
  ...BASE_FEATURES,
  "Licences Microsoft Éducation (A1 / A3) selon effectif",
] as const;

const PRICING_PLANS: PricingPlan[] = [
  {
    id: "essentielle",
    name: "Essentielle",
    audienceLabel: "Moins de 500 élèves",
    priceMonthly: 299,
    priceLabel: "299 € / mois",
    priceHint: "Palier calculé automatiquement selon votre effectif",
    description:
      "Même plateforme ScolIA. Moins d'élèves = généralement moins de personnel : le pack de licences Microsoft est dimensionné en conséquence.",
    microsoftA3: 5,
    microsoftA1: 50,
    features: [
      ...BASE_FEATURES,
      "5 licences Microsoft A3 Éducation",
      "50 licences Microsoft A1 Éducation",
    ],
  },
  {
    id: "standard",
    name: "Standard",
    audienceLabel: "500 à 1 000 élèves",
    priceMonthly: 499,
    priceLabel: "499 € / mois",
    priceHint: "Palier calculé automatiquement selon votre effectif",
    description:
      "Même plateforme ScolIA. L'effectif détermine le tarif et le volume de licences Microsoft (direction, administratif, éducation).",
    microsoftA3: 10,
    microsoftA1: 100,
    features: [
      ...BASE_FEATURES,
      "10 licences Microsoft A3 Éducation",
      "100 licences Microsoft A1 Éducation",
    ],
  },
  {
    id: "plus",
    name: "Plus",
    audienceLabel: "Plus de 1 000 élèves",
    priceMonthly: 699,
    priceLabel: "699 € / mois",
    priceHint: "Palier calculé automatiquement selon votre effectif",
    description:
      "Même plateforme ScolIA. Grands effectifs : pack Microsoft renforcé, sans modules à « choisir » — tout reste inclus.",
    microsoftA3: 15,
    microsoftA1: 150,
    features: [
      ...BASE_FEATURES,
      "15 licences Microsoft A3 Éducation",
      "150 licences Microsoft A1 Éducation",
    ],
  },
];

/** Message tarif fondateur — page tarifs (pas d'urgence artificielle). */
export const FOUNDING_PRICING = {
  badge: "Tarif fondateur",
  title: "Pour les premiers établissements",
  lead:
    "Vous rejoignez ScolIA parmi les premiers. Le palier calculé selon votre effectif est un tarif fondateur : prévisible, sans modules à débloquer plus tard.",
  promises: [
    {
      title: "Gelé 24 mois",
      detail:
        "Le palier d'entrée est maintenu pendant deux ans, quoi qu'il arrive.",
    },
    {
      title: "Tout le futur inclus",
      detail:
        "Les fonctionnalités actuelles et à venir sont comprises dans ce prix. Pas de demi-abonnement, pas de modules payants à activer ensuite.",
    },
    {
      title: "Même plateforme pour tous",
      detail:
        "Seul le volume de licences Microsoft change avec l'effectif. Le reste de ScolIA est identique pour chaque établissement.",
    },
  ],
} as const;

/** Déduit le palier tarifaire + licences Microsoft à partir de l'effectif. */
export function resolvePricingPlan(studentCount: number): PricingPlan {
  const n = Math.max(0, Math.floor(Number(studentCount) || 0));
  if (n < 500) return PRICING_PLANS[0]!;
  if (n <= 1000) return PRICING_PLANS[1]!;
  return PRICING_PLANS[2]!;
}

export const MICROSOFT_PRICING_NOTE = {
  title: "Licences Microsoft Éducation — inclus",
  eyebrow: "Microsoft Partner Program",
  partnerBadgeLabel: "Membre du Microsoft Partner Program",
  /** Badge officiel : téléchargez-le depuis Partner Center (Logo Builder) → public/partners/microsoft-partner.svg ou .png */
  partnerLogoPath: "/partners/microsoft-partner.svg",
  intro:
    "Chaque abonnement ScolIA inclut un pack Microsoft Éducation pour équiper direction, services administratifs et enseignants.",
  partnerNote:
    "En tant que membre du Microsoft Partner Program, ScolIA provisionne les licences Éducation (A1 / A3) dans le cadre de l'abonnement, via le canal partenaire.",
  bullets: [
    "Licences A3 pour les profils administratifs et de direction (Word, Excel, Outlook…).",
    "Licences A1 pour les enseignants (outils Office en ligne).",
    "Volumes de base définis selon l'effectif de l'établissement.",
    "Licences A3 Éducation supplémentaires disponibles sur demande : le tarif mensuel est alors ajusté selon le volume.",
  ],
  disclaimer:
    "Les volumes A1 / A3 indiqués correspondent à chaque palier d'effectif. Besoin de licences A3 supplémentaires ? Contactez-nous pour un devis adapté.",
} as const;

export const PRICING_FAQ = [
  {
    q: "Qu'est-ce que le tarif fondateur ?",
    a: "C'est le tarif réservé aux premiers établissements qui rejoignent ScolIA. Le palier calculé selon votre effectif est gelé pendant 24 mois.",
  },
  {
    q: "Les nouvelles fonctionnalités seront-elles payantes ?",
    a: "Non. Tout ce qui est dans ScolIA aujourd'hui, et tout ce qui sera ajouté ensuite, est compris dans le prix. Pas de modules à débloquer, pas de demi-abonnement.",
  },
  {
    q: "Y a-t-il des modules en option ?",
    a: "Non. Documents élèves, sorties, salles, RH, internat, cloud personnel, demandes et IA sont inclus — et les évolutions futures aussi.",
  },
  {
    q: "Comment est calculé mon tarif ?",
    a: "Vous n'avez pas de formule à choisir : le palier dépend de votre effectif. Moins de 500 élèves → 299 €/mois ; 500 à 1 000 → 499 €/mois ; plus de 1 000 → 699 €/mois. Moins d'élèves = généralement moins de licences éducation / admin à fournir.",
  },
  {
    q: "Que contiennent les licences Microsoft ?",
    a: "ScolIA est membre du Microsoft Partner Program. Moins de 500 élèves : 5× A3 + 50× A1. 500 à 1 000 : 10× A3 + 100× A1. Plus de 1 000 : 15× A3 + 150× A1.",
  },
  {
    q: "Puis-je ajouter des licences A3 si le pack ne suffit pas ?",
    a: "Oui. Des licences Microsoft A3 Éducation supplémentaires peuvent être ajoutées sur demande. Le montant mensuel est alors ajusté selon le volume nécessaire. Contactez-nous pour un devis.",
  },
  {
    q: "Puis-je résilier quand je veux ?",
    a: "Oui. L'abonnement est mensuel, sans engagement de durée. Contactez-nous pour un devis ou une mise en route.",
  },
  {
    q: "Où sont hébergées les données ?",
    a: "En France, chez Scaleway (Paris), cloud français. L'IA et l'OCR s'appuient sur Mistral AI. La messagerie passe par OVH. La bureautique repose sur Microsoft Éducation ; l'authentification sur Better-Auth (PostgreSQL France).",
  },
  {
    q: "Comment sont réglées les mensualités ?",
    a: "Via EasyTransac, startup française de paiement (PCI DSS). Contactez-nous pour la mise en place de l'abonnement.",
  },
  {
    q: "Comment démarrer ?",
    a: "Indiquez votre effectif : le tarif fondateur et le pack de licences se déterminent automatiquement. Nous planifions ensuite la mise en route avec la direction ou l'OGEC.",
  },
] as const;
