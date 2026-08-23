import type { DomainPlanningDomain, DomainPlanningSession, DomainPlanningSignup } from "@/app/lib/domain-planning-types";
import { hasRole } from "@/app/lib/intranet-role-utils";

export const DEFAULT_DOMAIN_PLANNING_ACTIVITY_COLORS: Record<string, string> = {
  "Séance 1": "bg-violet-600 text-white",
  "Séance 2": "bg-indigo-600 text-white",
  "Séance 3": "bg-fuchsia-600 text-white",
};

export const DEFAULT_DOMAIN_PLANNING_DOMAINS: DomainPlanningDomain[] = [
  {
    id: "evars",
    name: "EVARS",
    description: "Éducation à la vie affective, relationnelle et à la sexualité",
    color: "bg-rose-600 text-white",
    coordinatorExternalUserIds: [],
  },
];

/** Séances EVARS collège — structure issue du tableau de positionnement. */
export const DEFAULT_EVARS_SESSIONS: DomainPlanningSession[] = [
  {
    id: "6e-s1",
    niveau: "6e",
    seanceNumber: 1,
    theme: "La puberté et les transformations du corps",
    intervenantLabel: "Profs d'SVT",
    intervenantConstraint: "svt_only",
    mixte: true,
  },
  {
    id: "6e-s2",
    niveau: "6e",
    seanceNumber: 2,
    theme: "Construire des relations (famille, amis, amour)",
    intervenantLabel: "Association",
    intervenantConstraint: "fixed_association",
    mixte: false,
  },
  {
    id: "6e-s3",
    niveau: "6e",
    seanceNumber: 3,
    theme: "Trouver sa place dans la société ; être libre et responsable",
    intervenantLabel: "Au choix des professeurs",
    intervenantConstraint: "free",
    mixte: true,
  },
  {
    id: "5e-s1",
    niveau: "5e",
    seanceNumber: 1,
    theme: "Le sexe biologique et l'orientation sexuelle",
    intervenantLabel: "Profs d'SVT",
    intervenantConstraint: "svt_only",
    mixte: true,
  },
  {
    id: "5e-s2",
    niveau: "5e",
    seanceNumber: 2,
    theme: "Choisir ses relations et comprendre ses préférences",
    intervenantLabel: "Psychologue / Infirmière",
    intervenantConstraint: "psy_inf",
    mixte: false,
  },
  {
    id: "5e-s3",
    niveau: "5e",
    seanceNumber: 3,
    theme: "Vie privée / vie publique, liberté individuelle sur les réseaux sociaux",
    intervenantLabel: "Au choix des professeurs",
    intervenantConstraint: "free",
    mixte: true,
  },
  {
    id: "4e-s1",
    niveau: "4e",
    seanceNumber: 1,
    theme: "La sexualité, une réalité complexe (plaisir, amour, reproduction)",
    intervenantLabel: "Profs d'SVT",
    intervenantConstraint: "svt_only",
    mixte: true,
  },
  {
    id: "4e-s2",
    niveau: "4e",
    seanceNumber: 2,
    theme: "Compréhension critique des relations et santé sexuelle",
    intervenantLabel: "Association",
    intervenantConstraint: "fixed_association",
    mixte: false,
  },
  {
    id: "4e-s3",
    niveau: "4e",
    seanceNumber: 3,
    theme: "Représentations de la sexualité dans l'espace public et égalité",
    intervenantLabel: "Au choix des professeurs",
    intervenantConstraint: "free",
    mixte: true,
  },
  {
    id: "3e-s1",
    niveau: "3e",
    seanceNumber: 1,
    theme: "Liens entre bonheur, émotions et sexualité",
    intervenantLabel: "Profs d'SVT",
    intervenantConstraint: "svt_only",
    mixte: true,
  },
  {
    id: "3e-s2",
    niveau: "3e",
    seanceNumber: 2,
    theme: "Relations réciproques et égalitaires ; repérer danger et vulnérabilité",
    intervenantLabel: "Psychologue / Infirmière",
    intervenantConstraint: "psy_inf",
    mixte: false,
  },
  {
    id: "3e-s3",
    niveau: "3e",
    seanceNumber: 3,
    theme: "La sexualité dans la définition des droits humains",
    intervenantLabel: "Au choix des professeurs",
    intervenantConstraint: "free",
    mixte: true,
  },
];

export const TRANSVERSAL_NIVEAUX = ["6e", "5e", "4e", "3e"] as const;

export const TRANSVERSAL_NIVEAU_LABELS: Record<string, string> = {
  "6e": "6ème",
  "5e": "5ème",
  "4e": "4ème",
  "3e": "3ème",
};

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
  const prefix = niveau.replace(/e$/, "");
  const all = Object.values(classesByPole).flat();
  return all
    .filter((c) => c.toUpperCase().startsWith(prefix.toUpperCase()))
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
  return hasRole(roles, "professeur") || hasRole(roles, "education");
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
