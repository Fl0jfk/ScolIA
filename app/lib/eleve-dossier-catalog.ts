import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { sanitizeDomainPlanningClassesByPole } from "@/app/lib/domain-planning-defaults";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import type { EstablishmentKind } from "@/app/lib/app-config-schemas";

export type DossierSiteRef = {
  siteId: string;
  label: string;
  kind?: string | null;
};

export type DossierClassOption = {
  value: string;
  label: string;
  siteId: string | null;
  siteLabel: string | null;
};

export type EleveDossierClassCatalog = {
  sites: DossierSiteRef[];
  siteLabelById: Map<string, string>;
  classToSiteId: Map<string, string>;
  classOptions: DossierClassOption[];
};

/** Référentiel par défaut (aligné réservation de salles) quand la config tenant est vide. */
const DEFAULT_CLASSES_BY_POLE: Record<string, string[]> = {
  ÉCOLE: ["CP", "CE1", "CE2", "CM1", "CM2"],
  COLLÈGE: [
    "6A", "6B", "6C", "6D", "6E", "6F",
    "5A", "5B", "5C", "5D", "5E", "5F",
    "4A", "4B", "4C", "4D", "4E", "4F",
    "3A", "3B", "3C", "3D", "3E", "3F",
  ],
  LYCÉE: [
    "2A", "2B", "2C", "2D", "2E",
    "1A", "1B", "1C", "1D", "1E", "1F",
    "TA", "TB", "TC", "TD", "TE", "TF",
  ],
};

function normalizeClassKey(className: string): string {
  return className
    .trim()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[°º]/g, "")
    .toUpperCase();
}

function inferSiteKindFromClassName(className: string): EstablishmentKind | null {
  const key = normalizeClassKey(className);
  if (!key) return null;
  if (/^(CP|CE1|CE2|CM1|CM2|M[1-3]|GS|MS|PS)/.test(key)) return "ecole";
  if (/^[3456][A-Z]?$/.test(key)) return "college";
  if (/^2[A-Z]?$/.test(key) || /^1[A-Z]?$/.test(key) || /^T[A-Z]?$/.test(key)) return "lycee";
  return null;
}

function siteIdForKind(
  kind: EstablishmentKind,
  sites: DossierSiteRef[],
): string | null {
  const matches = sites.filter(
    (s) =>
      inferEstablishmentKind({ kind: s.kind ?? undefined, id: s.siteId, label: s.label }) ===
      kind,
  );
  if (matches.length === 1) return matches[0]!.siteId;
  const byId = sites.find((s) => s.siteId === kind);
  return byId?.siteId ?? null;
}

function fold(s: string): string {
  return s
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[_\s-]+/g, "");
}

function inferKindFromPole(pole: string): EstablishmentKind | null {
  const blob = fold(pole);
  if (blob.includes("ecole") || blob.includes("primaire") || blob.includes("elementaire")) {
    return "ecole";
  }
  if (blob.includes("college")) return "college";
  if (blob.includes("lycee")) return "lycee";
  return null;
}

function resolvePoleToSiteId(pole: string, sites: DossierSiteRef[]): string | null {
  const foldedPole = fold(pole);
  if (!foldedPole) return null;

  const direct = sites.find(
    (s) => fold(s.siteId) === foldedPole || fold(s.label) === foldedPole,
  );
  if (direct) return direct.siteId;

  const kind = inferKindFromPole(pole);
  if (!kind) return null;

  const byKind = sites.filter(
    (s) => inferEstablishmentKind({ kind: s.kind ?? undefined, id: s.siteId, label: s.label }) === kind,
  );
  if (byKind.length === 1) return byKind[0]!.siteId;

  const byId = sites.find((s) => s.siteId === kind);
  if (byId) return byId.siteId;

  return null;
}

function mergeClassesByPole(
  profRoom: Record<string, string[]>,
  domainPlanning: Record<string, string[]>,
): Record<string, string[]> {
  const merged: Record<string, string[]> = { ...domainPlanning };
  for (const [pole, list] of Object.entries(profRoom)) {
    const cur = merged[pole] || [];
    const next = [...cur];
    for (const cls of list || []) {
      const trimmed = String(cls).trim();
      if (trimmed && !next.includes(trimmed)) next.push(trimmed);
    }
    merged[pole] = next;
  }
  return merged;
}

export function classOptionLabel(className: string, siteLabel: string | null): string {
  const cls = className.trim();
  if (!cls) return "";
  return siteLabel ? `${cls} — ${siteLabel}` : cls;
}

export function resolveSiteLabel(
  siteId: string | null | undefined,
  catalog: Pick<EleveDossierClassCatalog, "siteLabelById">,
): string | null {
  if (!siteId) return null;
  return catalog.siteLabelById.get(siteId) ?? null;
}

export function resolveSiteIdForClass(
  className: string | null | undefined,
  catalog: Pick<EleveDossierClassCatalog, "classToSiteId" | "sites">,
): string | null {
  const cls = String(className || "").trim();
  if (!cls) return null;

  const direct = catalog.classToSiteId.get(cls);
  if (direct) return direct;

  const norm = normalizeClassKey(cls);
  for (const [key, siteId] of catalog.classToSiteId.entries()) {
    if (normalizeClassKey(key) === norm) return siteId;
  }

  const kind = inferSiteKindFromClassName(cls);
  if (kind) return siteIdForKind(kind, catalog.sites);

  return null;
}

/** Élèves de démo / tests — exclus de la liste hub tant que le référentiel n’est pas branché. */
export function isExcludedFromDossierList(row: {
  nom: string;
  prenom?: string;
  sourceKey?: string;
}): boolean {
  if (fold(row.nom) === "mangisson") return true;
  return false;
}

export async function buildEleveDossierClassCatalog(
  sites: DossierSiteRef[],
): Promise<EleveDossierClassCatalog> {
  const config = await loadAppConfig();
  const profRoom = sanitizeDomainPlanningClassesByPole(config.profRoom?.classesByPole || {});
  const domainPlanning = sanitizeDomainPlanningClassesByPole(
    config.domainPlanning?.classesByPole || {},
  );
  let merged = mergeClassesByPole(profRoom, domainPlanning);
  if (Object.keys(merged).length === 0) {
    merged = DEFAULT_CLASSES_BY_POLE;
  }

  const siteLabelById = new Map<string, string>();
  for (const site of sites) {
    siteLabelById.set(site.siteId, site.label.trim() || site.siteId);
  }

  const classToSiteId = new Map<string, string>();
  const classOptions: DossierClassOption[] = [];

  for (const [pole, classes] of Object.entries(merged)) {
    const siteId = resolvePoleToSiteId(pole, sites);
    const siteLabel = siteId ? resolveSiteLabel(siteId, { siteLabelById }) : pole.trim() || null;

    for (const rawCls of classes) {
      const cls = String(rawCls).trim();
      if (!cls) continue;
      if (siteId) {
        if (!classToSiteId.has(cls)) classToSiteId.set(cls, siteId);
        const norm = normalizeClassKey(cls);
        if (norm !== cls && !classToSiteId.has(norm)) classToSiteId.set(norm, siteId);
      }
      classOptions.push({
        value: cls,
        label: classOptionLabel(cls, siteLabel),
        siteId,
        siteLabel,
      });
    }
  }

  for (const opt of classOptions) {
    if (opt.siteId) continue;
    const inferred = resolveSiteIdForClass(opt.value, { classToSiteId, sites });
    if (!inferred) continue;
    opt.siteId = inferred;
    opt.siteLabel = resolveSiteLabel(inferred, { siteLabelById });
    opt.label = classOptionLabel(opt.value, opt.siteLabel);
    if (!classToSiteId.has(opt.value)) classToSiteId.set(opt.value, inferred);
  }

  classOptions.sort((a, b) =>
    a.label.localeCompare(b.label, "fr", { sensitivity: "base", numeric: true }),
  );

  return {
    sites,
    siteLabelById,
    classToSiteId,
    classOptions,
  };
}

export function dossierClassOptionsForSite(
  catalog: EleveDossierClassCatalog,
  siteId: string | null | undefined,
  extraClasses: string[] = [],
): DossierClassOption[] {
  const site = siteId?.trim() || "";
  const fromCatalog = site
    ? catalog.classOptions.filter((o) => o.siteId === site)
    : catalog.classOptions;

  const seen = new Set(fromCatalog.map((o) => o.value));
  const merged = [...fromCatalog];

  for (const rawCls of extraClasses) {
    const cls = rawCls.trim();
    if (!cls || seen.has(cls)) continue;
    const clsSiteId = resolveSiteIdForClass(cls, catalog);
    if (site && clsSiteId !== site) continue;
    const siteLabel = resolveSiteLabel(clsSiteId, catalog);
    merged.push({
      value: cls,
      label: classOptionLabel(cls, siteLabel),
      siteId: clsSiteId,
      siteLabel,
    });
    seen.add(cls);
  }

  merged.sort((a, b) =>
    a.label.localeCompare(b.label, "fr", { sensitivity: "base", numeric: true }),
  );
  return merged;
}

export function enrichEleveDossierListItem(
  row: {
    id: string;
    nom: string;
    prenom: string;
    classe: string | null;
    status: string;
    siteId: string | null;
    folderName: string;
    ine: string | null;
  },
  catalog: EleveDossierClassCatalog,
): {
  id: string;
  nom: string;
  prenom: string;
  classe: string | null;
  classeLabel: string | null;
  status: string;
  siteId: string | null;
  siteLabel: string | null;
  folderName: string;
  ine: string | null;
} {
  const classSiteId = resolveSiteIdForClass(row.classe, catalog);
  const classSiteLabel = resolveSiteLabel(classSiteId, catalog);
  const resolvedSiteId = row.siteId ?? classSiteId;
  const siteLabel = resolveSiteLabel(resolvedSiteId, catalog);

  return {
    ...row,
    siteId: resolvedSiteId,
    siteLabel,
    classeLabel: row.classe ? classOptionLabel(row.classe, classSiteLabel) : null,
  };
}
