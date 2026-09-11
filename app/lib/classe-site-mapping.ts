import "server-only";

import { and, eq, inArray, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { classeSiteMapping, eleve, etablissementSite } from "@/db/schema";
import { loadOfficialSchoolClasses } from "@/app/lib/nomenclature-classes";
import {
  classeMappingKey,
  guessSiteKindForClass,
  suggestSiecleCode,
  type ClasseSiteMappingRecord,
  type SiecleDivisionOption,
} from "@/app/lib/classe-site-mapping-logic";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import type { EstablishmentKind } from "@/app/lib/app-config-schemas";

export type { ClasseSiteMappingRecord, SiecleDivisionOption };

export type ClasseMappingSiteRef = {
  siteId: string;
  label: string;
  kind: string | null;
};

export type ObservedClasseRow = {
  className: string;
  classKey: string;
  eleveCount: number;
  mappedSiteId: string | null;
  mappedSiecleCode: string | null;
  suggestedSiteId: string | null;
  suggestedSiecleCode: string | null;
};

function siteIdForKind(
  kind: EstablishmentKind,
  sites: ClasseMappingSiteRef[],
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

let mappingTableEnsured = false;
let mappingTableMissing = false;

/** Création additive — la table peut manquer tant que la migration 0036 n’est pas passée. */
export async function ensureClasseSiteMappingTable(): Promise<void> {
  if (mappingTableEnsured) return;
  const db = getDb();
  await db.execute(
    sql.raw(`
      CREATE TABLE IF NOT EXISTS "classe_site_mapping" (
        "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
        "etablissement_id" uuid NOT NULL REFERENCES "etablissement"("id") ON DELETE CASCADE,
        "class_key" text NOT NULL,
        "class_name" text NOT NULL,
        "site_id" text NOT NULL,
        "siecle_code" text,
        "created_at" timestamp with time zone DEFAULT now() NOT NULL,
        "updated_at" timestamp with time zone DEFAULT now() NOT NULL
      )
    `),
  );
  await db.execute(
    sql.raw(
      `CREATE UNIQUE INDEX IF NOT EXISTS "classe_site_mapping_class_uidx" ON "classe_site_mapping" ("etablissement_id","class_key")`,
    ),
  );
  await db.execute(
    sql.raw(
      `CREATE INDEX IF NOT EXISTS "classe_site_mapping_etablissement_idx" ON "classe_site_mapping" ("etablissement_id")`,
    ),
  );
  await db.execute(
    sql.raw(
      `CREATE INDEX IF NOT EXISTS "classe_site_mapping_site_idx" ON "classe_site_mapping" ("etablissement_id","site_id")`,
    ),
  );
  mappingTableEnsured = true;
  mappingTableMissing = false;
}

export async function loadClasseSiteMappings(
  etablissementId: string,
): Promise<ClasseSiteMappingRecord[]> {
  if (mappingTableMissing && !mappingTableEnsured) return [];
  try {
    const db = getDb();
    const rows = await db
      .select({
        classKey: classeSiteMapping.classKey,
        className: classeSiteMapping.className,
        siteId: classeSiteMapping.siteId,
        siecleCode: classeSiteMapping.siecleCode,
      })
      .from(classeSiteMapping)
      .where(eq(classeSiteMapping.etablissementId, etablissementId));
    return rows.map((r) => ({
      classKey: r.classKey,
      className: r.className,
      siteId: r.siteId,
      siecleCode: r.siecleCode,
    }));
  } catch (e) {
    mappingTableMissing = true;
    console.warn("[classe-site-mapping] lecture impossible (table absente ?)", e);
    return [];
  }
}

export async function listObservedClassNames(etablissementId: string): Promise<string[]> {
  const db = getDb();
  const rows = await db
    .selectDistinct({ classe: eleve.classe })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.status, "inscrit")));
  const seen = new Set<string>();
  const out: string[] = [];
  for (const r of rows) {
    const cls = String(r.classe || "").trim();
    if (!cls) continue;
    const key = classeMappingKey(cls);
    if (!key || seen.has(key)) continue;
    seen.add(key);
    out.push(cls);
  }
  out.sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base", numeric: true }));
  return out;
}

export async function saveClasseSiteMappings(opts: {
  etablissementId: string;
  mappings: Array<{ className: string; siteId: string; siecleCode?: string | null }>;
  allowedSiteIds: Set<string>;
}): Promise<ClasseSiteMappingRecord[]> {
  await ensureClasseSiteMappingTable();
  const db = getDb();
  const now = new Date();

  const incoming: ClasseSiteMappingRecord[] = [];
  const seenKeys = new Set<string>();
  for (const raw of opts.mappings) {
    const className = String(raw.className || "").trim();
    const siteId = String(raw.siteId || "").trim();
    if (!className || !siteId) continue;
    if (!opts.allowedSiteIds.has(siteId)) {
      throw new Error(`Site inconnu : ${siteId}`);
    }
    const classKey = classeMappingKey(className);
    if (!classKey || seenKeys.has(classKey)) continue;
    seenKeys.add(classKey);
    const siecle = String(raw.siecleCode || "").trim();
    incoming.push({
      classKey,
      className,
      siteId,
      siecleCode: siecle || null,
    });
  }

  const existing = await loadClasseSiteMappings(opts.etablissementId);
  const toDelete = existing
    .filter((row) => !seenKeys.has(row.classKey))
    .map((row) => row.classKey);

  if (toDelete.length > 0) {
    await db
      .delete(classeSiteMapping)
      .where(
        and(
          eq(classeSiteMapping.etablissementId, opts.etablissementId),
          inArray(classeSiteMapping.classKey, toDelete),
        ),
      );
  }

  for (const row of incoming) {
    await db
      .insert(classeSiteMapping)
      .values({
        etablissementId: opts.etablissementId,
        classKey: row.classKey,
        className: row.className,
        siteId: row.siteId,
        siecleCode: row.siecleCode,
        updatedAt: now,
      })
      .onConflictDoUpdate({
        target: [classeSiteMapping.etablissementId, classeSiteMapping.classKey],
        set: {
          className: row.className,
          siteId: row.siteId,
          siecleCode: row.siecleCode,
          updatedAt: now,
        },
      });
  }

  return loadClasseSiteMappings(opts.etablissementId);
}

export async function loadClasseMappingWorkspace(etablissementId: string): Promise<{
  sites: ClasseMappingSiteRef[];
  siecleDivisions: SiecleDivisionOption[];
  classes: ObservedClasseRow[];
}> {
  try {
    await ensureClasseSiteMappingTable();
  } catch (e) {
    console.warn("[classe-site-mapping] création table impossible", e);
  }
  const db = getDb();
  const [sites, mappings, countRows, official] = await Promise.all([
    db
      .select({
        siteId: etablissementSite.siteId,
        label: etablissementSite.label,
        kind: etablissementSite.kind,
      })
      .from(etablissementSite)
      .where(eq(etablissementSite.etablissementId, etablissementId)),
    loadClasseSiteMappings(etablissementId),
    db
      .select({
        classe: eleve.classe,
        n: sql<number>`count(*)::int`,
      })
      .from(eleve)
      .where(eq(eleve.etablissementId, etablissementId))
      .groupBy(eleve.classe),
    loadOfficialSchoolClasses(etablissementId),
  ]);

  const siecleDivisions: SiecleDivisionOption[] = (official?.divisions ?? []).map((d) => ({
    code: d.code,
    libelle: d.libelleLong || d.libelleCourt || null,
  }));
  siecleDivisions.sort((a, b) =>
    a.code.localeCompare(b.code, "fr", { sensitivity: "base", numeric: true }),
  );

  const mappingByKey = new Map(mappings.map((m) => [m.classKey, m]));
  const byKey = new Map<string, ObservedClasseRow>();

  const pushClass = (className: string, eleveCount: number) => {
    const cls = className.trim();
    if (!cls) return;
    const classKey = classeMappingKey(cls);
    if (!classKey) return;
    const existing = byKey.get(classKey);
    if (existing) {
      existing.eleveCount += eleveCount;
      return;
    }
    const mapped = mappingByKey.get(classKey);
    const guessedKind = guessSiteKindForClass(cls);
    const suggestedSiteId = guessedKind ? siteIdForKind(guessedKind, sites) : null;
    byKey.set(classKey, {
      className: mapped?.className || cls,
      classKey,
      eleveCount,
      mappedSiteId: mapped?.siteId ?? null,
      mappedSiecleCode: mapped?.siecleCode ?? null,
      suggestedSiteId,
      suggestedSiecleCode: suggestSiecleCode(cls, siecleDivisions),
    });
  };

  for (const row of countRows) {
    pushClass(String(row.classe || ""), Number(row.n) || 0);
  }
  for (const m of mappings) {
    pushClass(m.className, 0);
  }
  for (const d of siecleDivisions) {
    pushClass(d.code, 0);
  }

  const classes = [...byKey.values()].sort((a, b) =>
    a.className.localeCompare(b.className, "fr", { sensitivity: "base", numeric: true }),
  );

  return { sites, siecleDivisions, classes };
}
