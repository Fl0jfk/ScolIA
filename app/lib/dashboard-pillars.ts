import type { DashboardCategory } from "@/app/lib/intranet-modules";
import { hasGlobalAdminRole, hasMasterRole, hasRole } from "@/app/lib/intranet-role-utils";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import {
  ADMINISTRATIF_PROF_MODULE_IDS,
  isProfesseurScopedDossierViewer,
} from "@/app/lib/eleve-dossier-scope";

export type DashboardPillarId =
  | "administratif"
  | "etablissement"
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
    description: "Dossiers élèves, notes & bulletins, stages",
    // Profs : pilier visible mais modules réduits (voir moduleIdsForPillarViewer).
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "admin",
      "professeur",
      "infirmerie",
      "comptabilite",
      "psychologue",
      "surveillant",
      "cpe",
    ],
    moduleIds: [
      "eleve-dossier",
      "notes",
      "groupes-pedagogiques",
      "stages",
      "agent-ia-ocr",
      "certificates",
    ],
  },
  {
    id: "etablissement",
    title: "Établissement",
    href: "/etablissement",
    description: "Paramètres, événements, annuaire, communication",
    allowedRoles: [
      ...DIRECTIONS,
      "administratif",
      "admin",
      "professeur",
      "maintenance",
      "infirmerie",
      "comptabilite",
      "psychologue",
      "surveillant",
      "cpe",
    ],
    moduleIds: [
      "admin-settings",
      "organigramme",
      "evenements",
      "communication",
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
      "surveillant",
      "comptabilite",
      "maintenance",
      "infirmerie",
      "psychologue",
      "surveillant",
      "cpe",
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
    description: "Internat, calendrier, groupes pédagogiques",
    allowedRoles: [
      ...DIRECTIONS,
      "cpe",
      "surveillant",
      "administratif",
      "admin",
      "professeur",
      "infirmerie",
    ],
    moduleIds: [
      "internat",
      "vs-calendrier",
      "groupes-pedagogiques",
    ],
  },
  {
    id: "compta_rh",
    title: "Comptabilité & RH",
    href: "/compta-rh",
    description: "RH, paie, facturation, absences pro",
    allowedRoles: [
      ...DIRECTIONS,
      "comptabilite",
      "administratif",
      "admin",
      "maintenance",
      "professeur",
      "infirmerie",
      "psychologue",
      "surveillant",
      "cpe",
    ],
    moduleIds: ["rh", "mon-planning", "conformite-rgpd"],
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
  "groupes-pedagogiques": "administratif",
  "facturation-familles": "compta_rh",
  stages: "administratif",
  "agent-ia-ocr": "administratif",
  certificates: "administratif",
  "pilotage-eleves": "administratif",
  organigramme: "etablissement",
  evenements: "etablissement",
  communication: "etablissement",
  "admin-settings": "etablissement",
  "conformite-rgpd": "etablissement",
  "chatbot-knowledge": "etablissement",
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
  "vs-calendrier": "vie_scolaire",
  "vs-appels": "vie_scolaire",
  "vs-absences": "vie_scolaire",
  "vs-sanctions": "vie_scolaire",
  "vs-carnet": "vie_scolaire",
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

/** Le rôle a-t-il le droit de voir ce pilier ?
 * Règle produit : le pilier apparaît dès qu’au moins un de ses modules est accessible
 * (sauf Santé, cloisonnement strict).
 */
export function pillarAllowedForRoles(
  pillar: DashboardPillarDef,
  roles: string[],
  opts?: { orgAdmin?: boolean; accessibleModuleIds?: Set<string> },
): boolean {
  // Santé : cloisonnement strict (infirmier / psychologue uniquement).
  if (pillar.id === "sante") {
    return pillar.allowedRoles.some((r) => hasRole(roles, r));
  }
  if (hasMasterRole(roles) || hasGlobalAdminRole(roles) || opts?.orgAdmin) {
    return true;
  }
  if (opts?.accessibleModuleIds) {
    const moduleIds = moduleIdsForPillarViewer(pillar, roles, opts);
    return moduleIds.some((id) => opts.accessibleModuleIds!.has(id));
  }
  return pillar.allowedRoles.some((r) => hasRole(roles, r));
}

export function pillarHasVisibleModules(
  pillar: DashboardPillarDef,
  categories: DashboardCategory[],
  roles?: string[],
  opts?: { orgAdmin?: boolean; accessibleModuleIds?: Set<string> },
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
  const visible = categoriesForPillar(pillar, categories, roles, opts);
  if (opts?.accessibleModuleIds) {
    return visible.some((c) => opts.accessibleModuleIds!.has(c.moduleId));
  }
  return visible.length > 0;
}
