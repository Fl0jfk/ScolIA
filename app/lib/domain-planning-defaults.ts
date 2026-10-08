import type { DomainPlanningDomain, DomainPlanningSession, DomainPlanningSignup } from "@/app/lib/domain-planning-types";
import { hasRole } from "@/app/lib/intranet-role-utils";

export const DEFAULT_DOMAIN_PLANNING_ACTIVITY_COLORS: Record<string, string> = {
  "Séance 1": "bg-violet-600 text-white",
  "Séance 2": "bg-indigo-600 text-white",
  "Séance 3": "bg-fuchsia-600 text-white",
};

/** Id du domaine collège historique (renommé librement côté UI, id stable). */
export const DEFAULT_DOMAIN_ID = "evars";

export const DEFAULT_DOMAIN_PLANNING_DOMAINS: DomainPlanningDomain[] = [
  {
    id: DEFAULT_DOMAIN_ID,
    name: "EVARS",
    description: "Éducation à la vie affective, relationnelle et à la sexualité",
    color: "bg-rose-600 text-white",
    coordinatorExternalUserIds: [],
  },
];

function collegeSession(
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
    domainId: DEFAULT_DOMAIN_ID,
    niveau,
    seanceNumber,
    theme,
    intervenantLabel,
    intervenantConstraint,
    mixte,
  };
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

/** Grille vide 2nde / 1ère / Tle × 3 séances pour un nouveau domaine lycée. */
export function buildEmptyLyceeSessions(domainId: string): DomainPlanningSession[] {
  const niveaux: DomainPlanningSession["niveau"][] = ["2nde", "1ere", "tle"];
  const out: DomainPlanningSession[] = [];
  for (const niveau of niveaux) {
    for (const seanceNumber of [1, 2, 3] as const) {
      out.push({
        id: `${domainId}-${niveau}-s${seanceNumber}`,
        domainId,
        niveau,
        seanceNumber,
        theme: "",
        intervenantLabel: "Au choix des professeurs",
        intervenantConstraint: "free",
        mixte: true,
      });
    }
  }
  return out;
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
