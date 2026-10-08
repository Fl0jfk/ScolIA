import type { DomainPlanningDomain, DomainPlanningSession, DomainPlanningSignup } from "@/app/lib/domain-planning-types";
import { hasRole } from "@/app/lib/intranet-role-utils";

export const DEFAULT_DOMAIN_PLANNING_ACTIVITY_COLORS: Record<string, string> = {
  "Séance 1": "bg-violet-600 text-white",
  "Séance 2": "bg-indigo-600 text-white",
  "Séance 3": "bg-fuchsia-600 text-white",
};

/** Id du domaine collège historique (renommé librement côté UI, id stable). */
export const DEFAULT_DOMAIN_ID = "evars";

/** Id du domaine lycée par défaut (programme EVARS septembre 2025). */
export const DEFAULT_EVARS_LYCEE_DOMAIN_ID = "evars-lycee";

export const DEFAULT_DOMAIN_PLANNING_DOMAINS: DomainPlanningDomain[] = [
  {
    id: DEFAULT_DOMAIN_ID,
    name: "EVARS collège",
    description: "Éducation à la vie affective, relationnelle et à la sexualité — collège",
    color: "bg-rose-600 text-white",
    coordinatorExternalUserIds: [],
  },
  {
    id: DEFAULT_EVARS_LYCEE_DOMAIN_ID,
    name: "EVARS lycée",
    description: "Éducation à la vie affective, relationnelle et à la sexualité — lycée",
    color: "bg-violet-600 text-white",
    coordinatorExternalUserIds: [],
  },
];

function planningSession(
  domainId: string,
  id: string,
  niveau: DomainPlanningSession["niveau"],
  seanceNumber: 1 | 2 | 3,
  theme: string,
  intervenantLabel: string,
  intervenantConstraint: DomainPlanningSession["intervenantConstraint"],
  mixte: boolean,
): DomainPlanningSession {
  return {
    id,
    domainId,
    niveau,
    seanceNumber,
    theme,
    intervenantLabel,
    intervenantConstraint,
    mixte,
  };
}

function collegeSession(
  id: string,
  niveau: DomainPlanningSession["niveau"],
  seanceNumber: 1 | 2 | 3,
  theme: string,
  intervenantLabel: string,
  intervenantConstraint: DomainPlanningSession["intervenantConstraint"],
  mixte: boolean,
): DomainPlanningSession {
  return planningSession(
    DEFAULT_DOMAIN_ID,
    id,
    niveau,
    seanceNumber,
    theme,
    intervenantLabel,
    intervenantConstraint,
    mixte,
  );
}

/** Séances collège par défaut — structure issue du tableau de positionnement. */
export const DEFAULT_EVARS_SESSIONS: DomainPlanningSession[] = [
  collegeSession("6e-s1", "6e", 1, "La puberté et les transformations du corps", "Profs d'SVT", "svt_only", true),
  collegeSession("6e-s2", "6e", 2, "Construire des relations (famille, amis, amour)", "Association", "fixed_association", false),
  collegeSession("6e-s3", "6e", 3, "Trouver sa place dans la société ; être libre et responsable", "Au choix des professeurs", "free", true),
  collegeSession("5e-s1", "5e", 1, "Le sexe biologique et l'orientation sexuelle", "Profs d'SVT", "svt_only", true),
  collegeSession("5e-s2", "5e", 2, "Choisir ses relations et comprendre ses préférences", "Psychologue / Infirmière", "psy_inf", false),
  collegeSession("5e-s3", "5e", 3, "Vie privée / vie publique, liberté individuelle sur les réseaux sociaux", "Au choix des professeurs", "free", true),
  collegeSession("4e-s1", "4e", 1, "La sexualité, une réalité complexe (plaisir, amour, reproduction)", "Profs d'SVT", "svt_only", true),
  collegeSession("4e-s2", "4e", 2, "Compréhension critique des relations et santé sexuelle", "Association", "fixed_association", false),
  collegeSession("4e-s3", "4e", 3, "Représentations de la sexualité dans l'espace public et égalité", "Au choix des professeurs", "free", true),
  collegeSession("3e-s1", "3e", 1, "Liens entre bonheur, émotions et sexualité", "Profs d'SVT", "svt_only", true),
  collegeSession("3e-s2", "3e", 2, "Relations réciproques et égalitaires ; repérer danger et vulnérabilité", "Psychologue / Infirmière", "psy_inf", false),
  collegeSession("3e-s3", "3e", 3, "La sexualité dans la définition des droits humains", "Au choix des professeurs", "free", true),
];

function lyceeSession(
  id: string,
  niveau: DomainPlanningSession["niveau"],
  seanceNumber: 1 | 2 | 3,
  theme: string,
  intervenantLabel: string,
  intervenantConstraint: DomainPlanningSession["intervenantConstraint"],
  mixte: boolean,
): DomainPlanningSession {
  return planningSession(
    DEFAULT_EVARS_LYCEE_DOMAIN_ID,
    id,
    niveau,
    seanceNumber,
    theme,
    intervenantLabel,
    intervenantConstraint,
    mixte,
  );
}

/**
 * Séances lycée par défaut — programme EVARS septembre 2025 (éducation.gouv.fr/evars).
 * Même principe que le collège : 3 séances / niveau, S1 SVT, S2 association ou psy/inf, S3 libre.
 */
export const DEFAULT_EVARS_LYCEE_SESSIONS: DomainPlanningSession[] = [
  // Seconde
  lyceeSession(
    "2nde-s1",
    "2nde",
    1,
    "Image, estime et confiance en soi",
    "Profs d'SVT",
    "svt_only",
    true,
  ),
  lyceeSession(
    "2nde-s2",
    "2nde",
    2,
    "Reconnaître et comprendre ses émotions",
    "Association",
    "fixed_association",
    false,
  ),
  lyceeSession(
    "2nde-s3",
    "2nde",
    3,
    "L'intimité à l'ère des réseaux sociaux",
    "Au choix des professeurs",
    "free",
    true,
  ),
  // Première
  lyceeSession(
    "1ere-s1",
    "1ere",
    1,
    "Plaisir, excès et conduites à risques : faire des choix éclairés",
    "Profs d'SVT",
    "svt_only",
    true,
  ),
  lyceeSession(
    "1ere-s2",
    "1ere",
    2,
    "Savoir dire oui ou non : le consentement",
    "Psychologue / Infirmière",
    "psy_inf",
    false,
  ),
  lyceeSession(
    "1ere-s3",
    "1ere",
    3,
    "Accueillir la diversité",
    "Au choix des professeurs",
    "free",
    true,
  ),
  // Terminale
  lyceeSession(
    "tle-s1",
    "tle",
    1,
    "Comprendre les enjeux de la pornographie",
    "Profs d'SVT",
    "svt_only",
    true,
  ),
  lyceeSession(
    "tle-s2",
    "tle",
    2,
    "Vivre une sexualité épanouie ou Développer une relation saine",
    "Association",
    "fixed_association",
    false,
  ),
  lyceeSession(
    "tle-s3",
    "tle",
    3,
    "Ma place dans le monde : oser être soi",
    "Au choix des professeurs",
    "free",
    true,
  ),
];

/** Toutes les séances EVARS par défaut (collège + lycée). */
export const DEFAULT_ALL_EVARS_SESSIONS: DomainPlanningSession[] = [
  ...DEFAULT_EVARS_SESSIONS,
  ...DEFAULT_EVARS_LYCEE_SESSIONS,
];

export const TRANSVERSAL_NIVEAUX = ["6e", "5e", "4e", "3e", "2nde", "1ere", "tle"] as const;

export const TRANSVERSAL_NIVEAU_LABELS: Record<string, string> = {
  "6e": "6ème",
  "5e": "5ème",
  "4e": "4ème",
  "3e": "3ème",
  "2nde": "2nde",
  "1ere": "1ère",
  tle: "Terminale",
};

/** Préfixes de codes classe (6A, 2B, TA…) pour rattacher une classe à un niveau. */
const NIVEAU_CLASS_PREFIXES: Record<string, string[]> = {
  "6e": ["6"],
  "5e": ["5"],
  "4e": ["4"],
  "3e": ["3"],
  "2nde": ["2"],
  "1ere": ["1"],
  tle: ["T"],
};

export function isTransversalNiveau(value: unknown): value is DomainPlanningSession["niveau"] {
  return (
    value === "6e" ||
    value === "5e" ||
    value === "4e" ||
    value === "3e" ||
    value === "2nde" ||
    value === "1ere" ||
    value === "tle"
  );
}

/**
 * Grille lycée 2nde / 1ère / Tle × 3 séances, préremplie avec le programme EVARS 2025.
 * Les thèmes et contraintes d'intervenants suivent le même principe que le collège.
 */
export function buildDefaultLyceeSessions(domainId: string): DomainPlanningSession[] {
  return DEFAULT_EVARS_LYCEE_SESSIONS.map((session) => ({
    ...session,
    id: `${domainId}-${session.niveau}-s${session.seanceNumber}`,
    domainId,
  }));
}

/** @deprecated Préférer `buildDefaultLyceeSessions` — alias conservé pour les appels existants. */
export function buildEmptyLyceeSessions(domainId: string): DomainPlanningSession[] {
  return buildDefaultLyceeSessions(domainId);
}

function isLyceeNiveau(niveau: DomainPlanningSession["niveau"]): boolean {
  return niveau === "2nde" || niveau === "1ere" || niveau === "tle";
}

/**
 * Anciennes formulations (flyer mal lu / première version) à remplacer par le programme retenu.
 * Clé = `niveau:seanceNumber`.
 */
const OUTDATED_LYCEE_THEMES_BY_SLOT: Record<string, readonly string[]> = {
  "1ere:1": ["Plaisir, excès, conduites à risque : faire des choix éclairés"],
  "1ere:3": ["Ma place dans le monde : oser être soi"],
  "tle:2": ["Vivre une sexualité épanouie"],
  "tle:3": ["Être libre d'être soi parmi les autres", "Développer des relations saines"],
};

/**
 * Remplit ou corrige les thèmes lycée :
 * - séances encore vides (ancienne grille vide) ;
 * - formulations obsolètes déjà enregistrées (correction programme).
 */
export function hydrateEmptySessionThemes(
  sessions: DomainPlanningSession[],
): DomainPlanningSession[] {
  const lyceeBySlot = new Map(
    DEFAULT_EVARS_LYCEE_SESSIONS.map((s) => [`${s.niveau}:${s.seanceNumber}`, s] as const),
  );
  return sessions.map((session) => {
    if (!isLyceeNiveau(session.niveau)) return session;
    const slotKey = `${session.niveau}:${session.seanceNumber}`;
    const source = lyceeBySlot.get(slotKey);
    if (!source) return session;
    const currentTheme = session.theme.trim();
    const outdated = OUTDATED_LYCEE_THEMES_BY_SLOT[slotKey] || [];
    const needsTheme = !currentTheme || outdated.includes(currentTheme);
    if (!needsTheme) return session;
    const looksLikeEmptyLyceeSeed =
      !currentTheme &&
      session.intervenantConstraint === "free" &&
      session.intervenantLabel === "Au choix des professeurs";
    return {
      ...session,
      theme: source.theme,
      ...(looksLikeEmptyLyceeSeed
        ? {
            intervenantLabel: source.intervenantLabel,
            intervenantConstraint: source.intervenantConstraint,
            mixte: source.mixte,
          }
        : {}),
    };
  });
}

/** Ajoute les séances lycée défaut si aucune séance 2nde/1ère/Tle n'est encore présente. */
export function ensureLyceeSessionsPresent(
  sessions: DomainPlanningSession[],
): DomainPlanningSession[] {
  const hasLycee = sessions.some((s) => isLyceeNiveau(s.niveau));
  return hasLycee ? sessions : [...sessions, ...DEFAULT_EVARS_LYCEE_SESSIONS];
}

/** Ajoute le domaine EVARS lycée défaut s'il manque alors que le collège est présent. */
export function ensureLyceeDomainPresent(domains: DomainPlanningDomain[]): DomainPlanningDomain[] {
  const ids = new Set(domains.map((d) => d.id));
  if (!ids.has(DEFAULT_DOMAIN_ID) || ids.has(DEFAULT_EVARS_LYCEE_DOMAIN_ID)) return domains;
  const lycee = DEFAULT_DOMAIN_PLANNING_DOMAINS.find((d) => d.id === DEFAULT_EVARS_LYCEE_DOMAIN_ID);
  return lycee ? [...domains, lycee] : domains;
}

/** Pôles réservés à d'autres modules (ex. réservation de salles). */
const EXCLUDED_CLASSES_POLES = new Set(["MAINTENANCE"]);

export function sanitizeDomainPlanningClassesByPole(
  classesByPole: Record<string, string[]>,
): Record<string, string[]> {
  const out: Record<string, string[]> = {};
  for (const [pole, classes] of Object.entries(classesByPole)) {
    if (EXCLUDED_CLASSES_POLES.has(pole.toUpperCase())) continue;
    out[pole] = classes;
  }
  return out;
}

export function classesForTransversalNiveau(
  niveau: string,
  classesByPole: Record<string, string[]>,
): string[] {
  const prefixes = NIVEAU_CLASS_PREFIXES[niveau] || [niveau.replace(/e$/, "")];
  const all = Object.values(classesByPole).flat();
  return all
    .filter((c) => {
      const upper = c.toUpperCase();
      return prefixes.some((p) => upper.startsWith(p.toUpperCase()));
    })
    .sort((a, b) => a.localeCompare(b, "fr"));
}

export function isSvtSubject(subject: string): boolean {
  const s = subject.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
  return /\bsvt\b/.test(s) || /sciences?\s*(de\s*la\s*)?vie/.test(s) || /profs?\s*d['']?svt/.test(s);
}

const SVT_LOCKED_SUBJECT = "Profs d'SVT";
const PSY_INF_LOCKED_SUBJECT = "Psychologue / Infirmière";
export const ASSOCIATION_LOCKED_SUBJECT = "Association";
export const ASSOCIATION_LOCKED_IDEA = "Association";

export function lockedSubjectForSession(session: DomainPlanningSession): string | null {
  switch (session.intervenantConstraint) {
    case "svt_only":
      return SVT_LOCKED_SUBJECT;
    case "fixed_association":
      return ASSOCIATION_LOCKED_SUBJECT;
    case "psy_inf":
      return PSY_INF_LOCKED_SUBJECT;
    default:
      return null;
  }
}

export function lockedSessionIdeaForSession(session: DomainPlanningSession): string | null {
  if (session.intervenantConstraint === "fixed_association") return ASSOCIATION_LOCKED_IDEA;
  return null;
}

function hasPsyInfRole(roles: string[]): boolean {
  return hasRole(roles, "infirmerie") || hasRole(roles, "psychologue");
}

function hasTeacherRole(roles: string[]): boolean {
  return hasRole(roles, "professeur") || hasRole(roles, "surveillant");
}

export function canUserSignupOnSession(
  session: DomainPlanningSession,
  roles: string[],
  _isCoordinator: boolean,
): boolean {
  if (session.intervenantConstraint === "fixed_association") return false;
  if (session.intervenantConstraint === "psy_inf") return hasPsyInfRole(roles);
  return true;
}

export function signupValidationStatus(
  signup: DomainPlanningSignup,
  session?: DomainPlanningSession | null,
): DomainPlanningSignup["validationStatus"] {
  if (signup.validationStatus) return signup.validationStatus;
  if (session?.intervenantConstraint === "free") return "pending";
  return "validated";
}

export function signupRequiresSessionIdea(session: DomainPlanningSession): boolean {
  return session.intervenantConstraint !== "fixed_association";
}

export function signupNeedsCoordinatorReview(
  signup: DomainPlanningSignup,
  session?: DomainPlanningSession | null,
): boolean {
  const status = signupValidationStatus(signup, session);
  return status === "pending" || status === "changes_requested";
}

export function normalizeSessionConstraint(
  constraint: unknown,
  intervenantLabel?: string,
): DomainPlanningSession["intervenantConstraint"] | null {
  if (
    constraint === "svt_only" ||
    constraint === "free" ||
    constraint === "fixed_association" ||
    constraint === "psy_inf"
  ) {
    return constraint;
  }
  if (constraint === "fixed") {
    const label = (intervenantLabel || "")
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase();
    if (label.includes("association")) return "fixed_association";
    if (label.includes("psychologue") || label.includes("infirmiere")) return "psy_inf";
    return "fixed_association";
  }
  return null;
}

function withDefaultDomainPlanningActivities<T extends { activityColors: Record<string, string> }>(
  config: T,
): T {
  return {
    ...config,
    activityColors: { ...DEFAULT_DOMAIN_PLANNING_ACTIVITY_COLORS, ...config.activityColors },
  };
}

export function normalizeDomainPlanningModule<
  T extends { classesByPole: Record<string, string[]>; activityColors: Record<string, string> },
>(config: T): T {
  return {
    ...withDefaultDomainPlanningActivities(config),
    classesByPole: sanitizeDomainPlanningClassesByPole(config.classesByPole),
  };
}
