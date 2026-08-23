import type { DashboardCategory } from "@/app/lib/intranet-modules";
import { hasGlobalAdminRole, hasMasterRole, hasRole } from "@/app/lib/intranet-role-utils";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import {
  ADMINISTRATIF_PROF_MODULE_IDS,
  isProfesseurScopedDossierViewer,
} from "@/app/lib/eleve-dossier-scope";

export type DashboardPillarId =
  | "administratif"
  | "services"
  | "vie_scolaire"
  | "compta_rh"
  | "sante";

export type DashboardPillarDef = {
  id: DashboardPillarId;
  title: string;
  href: string;
  description: string;
  /** Qui voit ce pilier sur l’accueil (en plus du filtre modules). */
  allowedRoles: string[];
  /** Modules intranet affichés sur la page hub / comme raccourcis de base. */
  moduleIds: string[];
};

const DIRECTIONS = [...INTRANET_DIRECTION_SLUGS];

/**
 * Piliers métier — grille filtrée par rôle.
 * Un module = un pilier « maison » (sauf affichage multi-hub volontaire).
 */
export const DASHBOARD_PILLARS: DashboardPillarDef[] = [
  {
    id: "administratif",
    title: "Administratif",
    href: "/administratif",
    description: "Dossiers, notes & bulletins, stages, établissement",
    // Profs : pilier visible mais modules réduits (voir moduleIdsForPillarViewer).
    allowedRoles: [...DIRECTIONS, "administratif", "admin", "professeur"],
    moduleIds: [
      "eleve-dossier",
      "notes",
      "stages",
      "agent-ia-ocr",
      "certificates",
      "pilotage-eleves",
      "organigramme",
      "evenements",
      "communication",
      "admin-settings",
      "conformite-rgpd",
      "chatbot-knowledge",
    ],
  },
  {
    id: "services",
    title: "Services",
    href: "/services",
    description: "Voyages, salles, demandes, outils du quotidien",
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "professeur",
      "cpe",
      "education",
      "comptabilite",
      "maintenance",
      "admin",
    ],
    moduleIds: [
      "travels",
      "prof-room",
      "requests-staff",
      "domain-planning",
      "documents",
      "toolbox",
      "channels",
      "assistance",
    ],
  },
  {
    id: "vie_scolaire",
    title: "Vie scolaire",
    href: "/vie-scolaire",
    description: "Internat, absences élèves, vie scolaire live",
    allowedRoles: [...DIRECTIONS, "cpe", "education"],
    moduleIds: ["internat"],
  },
  {
    id: "compta_rh",
    title: "Comptabilité & RH",
    href: "/compta-rh",
    description: "RH, paie, facturation, absences pro",
    allowedRoles: [...DIRECTIONS, "comptabilite", "administratif", "admin", "maintenance"],
    moduleIds: ["rh", "mon-planning", "documents", "conformite-rgpd"],
  },
  {
    id: "sante",
    title: "Santé",
    href: "/sante",
    description: "Infirmerie, PAP",
    allowedRoles: ["infirmerie", "psychologue"],
    moduleIds: ["sante"],
  },
];

/** Pilier « maison » pour les raccourcis dashboard (un module → un pilier signals). */
const PRIMARY_PILLAR_BY_MODULE: Record<string, DashboardPillarId> = {
  "eleve-dossier": "administratif",
  notes: "administratif",
  stages: "administratif",
  "agent-ia-ocr": "administratif",
  certificates: "administratif",
  "pilotage-eleves": "administratif",
  organigramme: "administratif",
  evenements: "administratif",
  communication: "administratif",
  "admin-settings": "administratif",
  "conformite-rgpd": "administratif",
  "chatbot-knowledge": "administratif",
  travels: "services",
  "prof-room": "services",
  "requests-staff": "services",
  "domain-planning": "services",
  documents: "services",
  toolbox: "services",
  channels: "services",
  assistance: "services",
  "photocopies-couleur": "services",
  covoiturage: "services",
  internat: "vie_scolaire",
  rh: "compta_rh",
  "mon-planning": "compta_rh",
  absences: "compta_rh",
  "demandes-hse": "compta_rh",
  sante: "sante",
};

export function moduleIdToPillarId(moduleId: string): DashboardPillarId | null {
  return PRIMARY_PILLAR_BY_MODULE[moduleId] ?? null;
}

export function moduleIdsForPillarViewer(
  pillar: DashboardPillarDef,
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): string[] {
  if (
    pillar.id === "administratif" &&
    isProfesseurScopedDossierViewer({
      roles,
      orgAdmin: opts?.orgAdmin,
      platformAdmin: opts?.platformAdmin,
    })
  ) {
    return [...ADMINISTRATIF_PROF_MODULE_IDS];
  }
  return pillar.moduleIds;
}

export function categoriesForPillar(
  pillar: DashboardPillarDef,
  categories: DashboardCategory[],
  roles?: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): DashboardCategory[] {
  const moduleIds =
    roles !== undefined
      ? moduleIdsForPillarViewer(pillar, roles, opts)
      : pillar.moduleIds;
  const order = new Map(moduleIds.map((id, i) => [id, i]));
  return categories
    .filter((c) => order.has(c.moduleId))
    .sort((a, b) => (order.get(a.moduleId) ?? 0) - (order.get(b.moduleId) ?? 0));
}

/** Le rôle a-t-il le droit de voir ce pilier (indépendamment des modules) ? */
export function pillarAllowedForRoles(
  pillar: DashboardPillarDef,
  roles: string[],
  opts?: { orgAdmin?: boolean },
): boolean {
  // Santé : cloisonnement strict (infirmier / psychologue uniquement).
  if (pillar.id === "sante") {
    return pillar.allowedRoles.some((r) => hasRole(roles, r));
  }
  if (hasMasterRole(roles) || hasGlobalAdminRole(roles) || opts?.orgAdmin) {
    return true;
  }
  return pillar.allowedRoles.some((r) => hasRole(roles, r));
}

export function pillarHasVisibleModules(
  pillar: DashboardPillarDef,
  categories: DashboardCategory[],
  roles?: string[],
  opts?: { orgAdmin?: boolean },
): boolean {
  if (roles && !pillarAllowedForRoles(pillar, roles, opts)) {
    return false;
  }
  if (pillar.id === "compta_rh") {
    const hasRh = categories.some(
      (c) => c.moduleId === "rh" || c.moduleId === "mon-planning",
    );
    if (hasRh) return true;
  }
  return categoriesForPillar(pillar, categories, roles, opts).length > 0;
}
