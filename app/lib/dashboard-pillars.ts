import type { DashboardCategory } from "@/app/lib/intranet-modules";
import { hasGlobalAdminRole, hasMasterRole, hasRole } from "@/app/lib/intranet-role-utils";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";

export type DashboardPillarId =
  | "administratif"
  | "vie_scolaire"
  | "notes"
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

export const DASHBOARD_PILLARS: DashboardPillarDef[] = [
  {
    id: "administratif",
    title: "Administratif",
    href: "/administratif",
    description: "Dossiers, préinscriptions, voyages, stages, établissement",
    allowedRoles: [...DIRECTIONS, "administratif", "admin"],
    moduleIds: [
      "eleve-dossier",
      "agent-ia-ocr",
      "travels",
      "stages",
      "certificates",
      "pilotage-eleves",
      "organigramme",
      "evenements",
      "communication",
      "admin-settings",
      "conformite-rgpd",
      "chatbot-knowledge",
      "requests-staff",
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
    description: "Internat, absences, vie scolaire live",
    allowedRoles: [...DIRECTIONS, "cpe", "education"],
    moduleIds: [
      "internat",
      "eleve-dossier",
      "travels",
      "stages",
      "domain-planning",
      "prof-room",
      "requests-staff",
    ],
  },
  {
    id: "notes",
    title: "Notes",
    href: "/notes",
    description: "Notes, bulletins, classes",
    allowedRoles: [...DIRECTIONS, "professeur"],
    moduleIds: ["notes", "pilotage-eleves", "eleve-dossier", "prof-room", "domain-planning"],
  },
  {
    id: "compta_rh",
    title: "Comptabilité & RH",
    href: "/compta-rh",
    description: "RH, paie, facturation, absences pro",
    allowedRoles: [...DIRECTIONS, "comptabilite", "administratif", "admin", "maintenance"],
    moduleIds: ["rh", "mon-planning", "documents", "conformite-rgpd", "travels"],
  },
  {
    id: "sante",
    title: "Santé",
    href: "/sante",
    description: "Infirmerie, PAP, bien-être",
    allowedRoles: ["infirmerie", "psychologue"],
    moduleIds: ["sante", "eleve-dossier", "bien-etre-referent"],
  },
];

const MODULE_TO_PILLAR = new Map<string, DashboardPillarId>();
for (const pillar of DASHBOARD_PILLARS) {
  for (const moduleId of pillar.moduleIds) {
    if (!MODULE_TO_PILLAR.has(moduleId)) {
      MODULE_TO_PILLAR.set(moduleId, pillar.id);
    }
  }
}
/** Modules signals / alias → pilier principal. */
MODULE_TO_PILLAR.set("absences", "compta_rh");
MODULE_TO_PILLAR.set("demandes-hse", "compta_rh");
MODULE_TO_PILLAR.set("photocopies-couleur", "administratif");
MODULE_TO_PILLAR.set("covoiturage", "administratif");

export function moduleIdToPillarId(moduleId: string): DashboardPillarId | null {
  return MODULE_TO_PILLAR.get(moduleId) ?? null;
}

export function categoriesForPillar(
  pillar: DashboardPillarDef,
  categories: DashboardCategory[],
): DashboardCategory[] {
  const order = new Map(pillar.moduleIds.map((id, i) => [id, i]));
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
  return categoriesForPillar(pillar, categories).length > 0;
}
