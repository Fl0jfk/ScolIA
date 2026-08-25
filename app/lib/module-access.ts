import {
  eleveDossierSectionsForRoles,
  type EleveDossierSection,
} from "@/app/lib/eleve-dossier-access";
import { hasGlobalAdminRole, hasMasterRole, hasRole } from "@/app/lib/intranet-role-utils";
import { INTRANET_ROLE_OPTIONS } from "@/app/lib/intranet-roles";
import {
  INTRANET_MODULES,
  getIntranetModuleById,
  rolesAllowModule,
  type IntranetModule,
} from "@/app/lib/intranet-modules";
import { DASHBOARD_PILLARS, moduleIdToPillarId } from "@/app/lib/dashboard-pillars";
import {
  customDefaultDossierSectionsForRole,
  customDefaultModulesForRole,
} from "@/app/lib/module-access-defaults";

/** Modules exclus de la matrice admin. */
const SKIP_MODULE_IDS = new Set([
  "pillar-administratif",
  "pillar-etablissement",
  "pillar-services",
  "pillar-vie-scolaire",
  "pillar-compta-rh",
  "scolia-ai",
  "legacy-hub-redirects",
  "rh-paie-spec",
  "dashboard-week-sheet",
  "vs-absences",
  "pilotage-eleves",
  "facturation-familles",
]);

export type ModuleAccessOverride = {
  modules: string[];
  dossierSections?: EleveDossierSection[];
};

/** @deprecated alias — préférer ModuleAccessOverride */
export type ModuleAccessRoleOverride = ModuleAccessOverride;

export type ModuleAccessConfig = {
  /** Baseline optionnelle par rôle (si aucune fiche personne). */
  byRole: Record<string, ModuleAccessOverride>;
  /** Droits individuels — prioritaire sur le rôle. Clé = Better-Auth user.id */
  byUser: Record<string, ModuleAccessOverride>;
};

export type ModuleAccessLookup = {
  userId?: string | null;
  businessUserId?: string | null;
};

export const DOSSIER_SECTION_OPTIONS: { id: EleveDossierSection; label: string }[] = [
  { id: "identite", label: "Identité" },
  { id: "scolarite", label: "Scolarité" },
  { id: "famille", label: "Famille" },
  { id: "documents", label: "Documents" },
  { id: "notes", label: "Notes" },
  { id: "vie_scolaire", label: "Vie scolaire" },
  { id: "sante", label: "Santé" },
  { id: "facturation", label: "Facturation" },
];

const ALL_DOSSIER_SECTIONS = DOSSIER_SECTION_OPTIONS.map((s) => s.id);

function parseOverride(value: unknown): ModuleAccessOverride | null {
  if (!value || typeof value !== "object") return null;
  const row = value as Record<string, unknown>;
  const modules = Array.isArray(row.modules)
    ? row.modules.map((m) => String(m).trim()).filter(Boolean)
    : [];
  const dossierSections = Array.isArray(row.dossierSections)
    ? row.dossierSections
        .map((s) => String(s).trim())
        .filter((s): s is EleveDossierSection =>
          ALL_DOSSIER_SECTIONS.includes(s as EleveDossierSection),
        )
    : undefined;
  return {
    modules: [...new Set(modules)],
    ...(dossierSections && dossierSections.length > 0 ? { dossierSections } : {}),
  };
}

export function defaultModuleAccess(): ModuleAccessConfig {
  return { byRole: {}, byUser: {} };
}

export function parseModuleAccess(raw: unknown): ModuleAccessConfig {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const byRoleRaw =
    o.byRole && typeof o.byRole === "object" ? (o.byRole as Record<string, unknown>) : {};
  const byUserRaw =
    o.byUser && typeof o.byUser === "object" ? (o.byUser as Record<string, unknown>) : {};
  const byRole: Record<string, ModuleAccessOverride> = {};
  const byUser: Record<string, ModuleAccessOverride> = {};
  for (const [role, value] of Object.entries(byRoleRaw)) {
    const parsed = parseOverride(value);
    if (role.trim() && parsed) byRole[role] = parsed;
  }
  for (const [userId, value] of Object.entries(byUserRaw)) {
    const parsed = parseOverride(value);
    if (userId.trim() && parsed) byUser[userId] = parsed;
  }
  return { byRole, byUser };
}

export function listConfigurableModules(): IntranetModule[] {
  return INTRANET_MODULES.filter((m) => {
    if (SKIP_MODULE_IDS.has(m.id)) return false;
    if (m.dashboard) return true;
    return moduleIdToPillarId(m.id) != null;
  });
}

export function defaultModulesForRole(role: string): string[] {
  const custom = customDefaultModulesForRole(role);
  if (custom) return custom;
  return listConfigurableModules()
    .filter((m) => m.allowedRoles.some((r) => hasRole([role], r)))
    .map((m) => m.id);
}

export function defaultDossierSectionsForRole(role: string): EleveDossierSection[] {
  const custom = customDefaultDossierSectionsForRole(role);
  if (custom) return custom as EleveDossierSection[];
  return [...eleveDossierSectionsForRoles([role])];
}

/** Union des modules par défaut pour un jeu de rôles. */
export function defaultModulesForRoles(roles: string[]): string[] {
  const out = new Set<string>();
  for (const role of roles) {
    if (role === "admin") {
      return listConfigurableModules().map((m) => m.id);
    }
    for (const id of defaultModulesForRole(role)) out.add(id);
  }
  return [...out];
}

export function defaultDossierSectionsForRoles(roles: string[]): EleveDossierSection[] {
  const out = new Set<EleveDossierSection>();
  let anyCustom = false;
  for (const role of roles) {
    const custom = customDefaultDossierSectionsForRole(role);
    if (custom) {
      anyCustom = true;
      for (const s of custom) out.add(s as EleveDossierSection);
    }
  }
  if (anyCustom) {
    // Union des defaults métier + sections natives des rôles sans defaults custom.
    for (const role of roles) {
      if (customDefaultDossierSectionsForRole(role)) continue;
      for (const s of eleveDossierSectionsForRoles([role])) out.add(s);
    }
    return [...out];
  }
  return [...eleveDossierSectionsForRoles(roles)];
}

export function findUserOverride(
  config: ModuleAccessConfig | null | undefined,
  lookup?: ModuleAccessLookup | null,
): ModuleAccessOverride | null {
  if (!config?.byUser) return null;
  const ids = [lookup?.userId, lookup?.businessUserId]
    .map((x) => (typeof x === "string" ? x.trim() : ""))
    .filter(Boolean);
  for (const id of ids) {
    const hit = config.byUser[id];
    if (hit) return hit;
  }
  return null;
}

export function effectiveModulesForRole(
  role: string,
  config: ModuleAccessConfig | null | undefined,
): string[] {
  const override = config?.byRole?.[role];
  if (override) return [...override.modules];
  return defaultModulesForRole(role);
}

export function effectiveDossierSectionsForRole(
  role: string,
  config: ModuleAccessConfig | null | undefined,
): EleveDossierSection[] {
  const override = config?.byRole?.[role];
  if (override?.dossierSections?.length) return [...override.dossierSections];
  return defaultDossierSectionsForRole(role);
}

/** Modules effectifs pour une personne (priorité fiche user → rôles → defaults). */
export function effectiveModulesForUser(
  roles: string[],
  config: ModuleAccessConfig | null | undefined,
  lookup?: ModuleAccessLookup | null,
): string[] {
  const userOv = findUserOverride(config, lookup);
  if (userOv) return [...userOv.modules];

  const out = new Set<string>();
  let anyRoleOverride = false;
  for (const role of roles) {
    const roleOv = config?.byRole?.[role];
    if (roleOv) {
      anyRoleOverride = true;
      for (const id of roleOv.modules) out.add(id);
    }
  }
  if (anyRoleOverride) {
    for (const role of roles) {
      if (config?.byRole?.[role]) continue;
      for (const id of defaultModulesForRole(role)) out.add(id);
    }
    return [...out];
  }
  return defaultModulesForRoles(roles);
}

export function effectiveDossierSectionsForUser(
  roles: string[],
  config: ModuleAccessConfig | null | undefined,
  lookup?: ModuleAccessLookup | null,
): EleveDossierSection[] {
  const userOv = findUserOverride(config, lookup);
  if (userOv?.dossierSections?.length) return [...userOv.dossierSections];

  const hasRoleOverride = Boolean(
    config && roles.some((r) => config.byRole[r]?.dossierSections?.length),
  );
  if (hasRoleOverride) {
    const out = new Set<EleveDossierSection>();
    for (const role of roles) {
      for (const s of effectiveDossierSectionsForRole(role, config)) out.add(s);
    }
    return [...out];
  }
  return defaultDossierSectionsForRoles(roles);
}

export function modulesGroupedByPillar(): Array<{
  pillarId: string;
  title: string;
  modules: Array<{ id: string; name: string; description?: string }>;
}> {
  const configurable = listConfigurableModules();
  const byPillar = new Map<string, Array<{ id: string; name: string; description?: string }>>();
  for (const pillar of DASHBOARD_PILLARS) {
    byPillar.set(pillar.id, []);
  }
  const orphan: Array<{ id: string; name: string; description?: string }> = [];

  for (const mod of configurable) {
    const name = mod.dashboard?.name || mod.id;
    const description = mod.dashboard?.description;
    const pillarId = moduleIdToPillarId(mod.id);
    const entry = { id: mod.id, name, description };
    if (pillarId && byPillar.has(pillarId)) {
      byPillar.get(pillarId)!.push(entry);
    } else {
      orphan.push(entry);
    }
  }

  const groups: Array<{
    pillarId: string;
    title: string;
    modules: Array<{ id: string; name: string; description?: string }>;
  }> = DASHBOARD_PILLARS.map((p) => ({
    pillarId: p.id,
    title: p.title,
    modules: byPillar.get(p.id) ?? [],
  })).filter((g) => g.modules.length > 0);

  if (orphan.length > 0) {
    groups.push({ pillarId: "autres", title: "Autres", modules: orphan });
  }
  return groups;
}

export function roleOptionsForModuleAccess(): { slug: string; label: string }[] {
  return INTRANET_ROLE_OPTIONS.filter(
    (r) => r.slug !== "parent" && r.slug !== "eleve" && r.slug !== "admin",
  );
}

export function accessibleModuleIdsForRoles(
  roles: string[],
  isOrgAdmin: boolean,
  access?: ModuleAccessConfig | null,
  lookup?: ModuleAccessLookup | null,
): Set<string> {
  if (hasMasterRole(roles) || hasGlobalAdminRole(roles) || isOrgAdmin || roles.includes("admin")) {
    return new Set(listConfigurableModules().map((m) => m.id));
  }
  const ids = new Set<string>();
  for (const m of INTRANET_MODULES) {
    if (rolesAllowModule(roles, m, isOrgAdmin, access, lookup ?? null)) {
      ids.add(m.id);
    }
  }
  return ids;
}

export function dossierSectionsForRolesWithAccess(
  roles: string[],
  opts: { orgAdmin?: boolean; platformAdmin?: boolean },
  access?: ModuleAccessConfig | null,
  lookup?: ModuleAccessLookup | null,
): Set<EleveDossierSection> {
  if (opts.platformAdmin || opts.orgAdmin || hasGlobalAdminRole(roles) || roles.includes("admin")) {
    return new Set(ALL_DOSSIER_SECTIONS);
  }
  const userOv = findUserOverride(access, lookup);
  if (userOv?.dossierSections?.length) {
    return new Set(userOv.dossierSections);
  }
  const hasRoleOverride = Boolean(access && Object.keys(access.byRole).length > 0);
  if (!hasRoleOverride) {
    return eleveDossierSectionsForRoles(roles, opts);
  }
  const out = new Set<EleveDossierSection>();
  for (const role of roles) {
    for (const s of effectiveDossierSectionsForRole(role, access)) out.add(s);
  }
  if (out.size === 0) {
    out.add("identite");
    out.add("scolarite");
  }
  return out;
}

export { getIntranetModuleById };
