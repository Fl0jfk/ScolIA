import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import type { EstablishmentKind } from "@/app/lib/app-config-schemas";
import {
  DEFAULT_CLASSES_BY_POLE,
  DEFAULT_ECOLE_CLASSES,
  foldSchoolClass,
  resolveClassesByPoleCatalog,
} from "@/app/lib/school-classes-catalog";
import { loadOfficialSchoolClasses, mergeClassesByPoleWithSiecle } from "@/app/lib/nomenclature-classes";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";

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
  const compact = key.replace(/[\s._\-/]+/g, "");
  const folded = key.toLowerCase().replace(/[\s._\-/]+/g, " ").trim();

  // École / maternelle / élémentaire (CP A, CE1-B, TPS, « École PS », etc.)
  if (
    /^(TPS|PS|MS|GS|CP|CE1|CE2|CM1|CM2|M[1-3])\b/.test(key) ||
    /^(TPS|PS|MS|GS|CP|CE1|CE2|CM1|CM2)/.test(compact) ||
    /\b(MATERNELLE|ELEMENTAIRE|PRIMAIRE|ECOLE)\b/.test(key)
  ) {
    return "ecole";
  }
  if (/\b(tps|ps|ms|gs|cp|ce1|ce2|cm1|cm2)\b/.test(folded)) return "ecole";

  // Collège : 6A…3F, 6ème A, 3e2…
  if (
    /^[3456][A-Z0-9]{0,2}$/.test(compact) ||
    /^[3456](E|EME|ÈME)/.test(compact) ||
    /\b[3456]\s*(E|EME|ÈME)?\b/.test(key)
  ) {
    return "college";
  }

  // Lycée : 2A, 1B, TA, 2nde, 1re, Tle…
  if (
    /^(2NDE|2DE|1RE|1ERE|TLE|TERMINALE|SECONDE|PREMIERE)/.test(compact) ||
    /^[12T][A-Z0-9]{0,2}$/.test(compact) ||
    /^T[A-Z]$/.test(compact)
  ) {
    return "lycee";
  }

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
  if (matches.length >= 1) return matches[0]!.siteId;
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
  if (
    blob.includes("ecole") ||
    blob.includes("primaire") ||
    blob.includes("elementaire") ||
    blob.includes("maternelle")
  ) {
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
  return resolveClassesByPoleCatalog(profRoom, domainPlanning);
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
  const folded = foldSchoolClass(cls);
  for (const [key, siteId] of catalog.classToSiteId.entries()) {
    if (normalizeClassKey(key) === norm) return siteId;
    if (folded && foldSchoolClass(key) === folded) return siteId;
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

const CATALOG_CACHE_MS = 60_000;
const catalogCache = new Map<string, { at: number; catalog: EleveDossierClassCatalog }>();

export async function buildEleveDossierClassCatalog(
  sites: DossierSiteRef[],
): Promise<EleveDossierClassCatalog> {
  const cacheKey = sites
    .map((s) => `${s.siteId}:${s.label}:${s.kind ?? ""}`)
    .sort()
    .join("|");
  const cached = catalogCache.get(cacheKey);
  if (cached && Date.now() - cached.at < CATALOG_CACHE_MS) {
    return cached.catalog;
  }

  const config = await loadAppConfig();
  const etabId = await resolveCurrentEtablissementId().catch(() => null);
  const official = etabId ? await loadOfficialSchoolClasses(etabId) : null;

  let merged = mergeClassesByPole(
    config.profRoom?.classesByPole || {},
    config.domainPlanning?.classesByPole || {},
  );
  if (Object.keys(merged).length === 0) {
    merged = { ...DEFAULT_CLASSES_BY_POLE };
  }

  if (official?.hasLockedSiecle) {
    merged = mergeClassesByPoleWithSiecle(official, merged);
  }

  // Site école présent mais aucun pôle école dans la config → injecter le catalogue école.
  const hasEcoleSite = sites.some(
    (s) =>
      inferEstablishmentKind({ kind: s.kind ?? undefined, id: s.siteId, label: s.label }) ===
      "ecole",
  );
  const hasEcolePole = Object.keys(merged).some((pole) => inferKindFromPole(pole) === "ecole");
  if (hasEcoleSite && !hasEcolePole) {
    merged = { ÉCOLE: [...DEFAULT_ECOLE_CLASSES], ...merged };
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

  const catalog: EleveDossierClassCatalog = {
    sites,
    siteLabelById,
    classToSiteId,
    classOptions,
  };
  catalogCache.set(cacheKey, { at: Date.now(), catalog });
  return catalog;
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

  /** Une entrée par forme compactée ; les libellés réellement présents en base priment. */
  const byFold = new Map<string, DossierClassOption>();

  for (const opt of fromCatalog) {
    const fold = foldSchoolClass(opt.value);
    if (!fold) continue;
    if (!byFold.has(fold)) byFold.set(fold, opt);
  }

  for (const rawCls of extraClasses) {
    const cls = rawCls.trim();
    if (!cls) continue;
    const clsSiteId = resolveSiteIdForClass(cls, catalog);
    if (site && clsSiteId !== site) continue;
    const fold = foldSchoolClass(cls);
    if (!fold) continue;
    const siteLabel = resolveSiteLabel(clsSiteId, catalog);
    const observed: DossierClassOption = {
      value: cls,
      label: classOptionLabel(cls, siteLabel),
      siteId: clsSiteId,
      siteLabel,
    };
    const existing = byFold.get(fold);
    if (!existing) {
      byFold.set(fold, observed);
      continue;
    }
    // Préférer le libellé tel qu’importé (PS A) plutôt que le synthétique catalogue (PSA).
    byFold.set(fold, {
      ...existing,
      value: cls,
      label: classOptionLabel(cls, existing.siteLabel ?? siteLabel),
      siteId: existing.siteId ?? clsSiteId,
      siteLabel: existing.siteLabel ?? siteLabel,
    });
  }

  // Si des classes réelles sont connues, retirer les libellés école injectés sans aucun élève.
  const observedFolds = new Set(
    extraClasses.map((c) => foldSchoolClass(c)).filter(Boolean),
  );
  if (observedFolds.size > 0) {
    const syntheticEcole = new Set(DEFAULT_ECOLE_CLASSES.map((c) => foldSchoolClass(c)));
    for (const fold of [...byFold.keys()]) {
      if (syntheticEcole.has(fold) && !observedFolds.has(fold)) {
        byFold.delete(fold);
      }
    }
  }

  const merged = [...byFold.values()];
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
    photoKey?: string | null;
    photoUrl?: string | null;
    secteur?: string | null;
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
  photoKey?: string | null;
  photoUrl?: string | null;
} {
  const classSiteId = resolveSiteIdForClass(row.classe, catalog);
  const secteurKind = inferKindFromPole(String(row.secteur || ""));
  const secteurSiteId = secteurKind ? siteIdForKind(secteurKind, catalog.sites) : null;
  // Classe > secteur import > scolarité (souvent mal rattachée avant correction).
  const resolvedSiteId = classSiteId ?? secteurSiteId ?? row.siteId;
  const classSiteLabel = resolveSiteLabel(classSiteId ?? resolvedSiteId, catalog);
  const siteLabel = resolveSiteLabel(resolvedSiteId, catalog);

  return {
    ...row,
    siteId: resolvedSiteId,
    siteLabel,
    classeLabel: row.classe ? classOptionLabel(row.classe, classSiteLabel) : null,
  };
}
