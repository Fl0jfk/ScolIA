import "server-only";

import { and, eq, inArray, sql } from "drizzle-orm";
import { isPgliteIntegrationTest } from "@/app/lib/scola-test-runtime";
import { getDb } from "@/db/index";
import {
  stageApplication,
  stageApplicationAttr,
  stageConvention,
  stageConventionAttr,
  stageOffer,
  stageOfferAttr,
  stageToken,
  stageTokenAttr,
} from "@/db/schema";
import { flattenToAttrs, inflateFromAttrs } from "@/app/lib/ent-attr-codec";
import {
  isEntCoreDbEnabled,
  resolveCurrentEtablissementId,
} from "@/app/lib/ent-core-db";
import type {
  StageConvention,
  StageConventionIndexEntry,
  StageOffer,
  StageOfferApplication,
  StageOfferIndexEntry,
  StageOfferCandidatureTokenRef,
  StageSignCodeLookupRef,
  StageSignTokenRef,
  StageStudentTokenRef,
} from "@/app/lib/stage-types";

export type StageTokenKind =
  | "sign"
  | "sign_code"
  | "student"
  | "offer_candidature"
  | "periods"
  | "referents"
  | "constraints"
  | "watchers"
  | "auto_purge"
  | "identity_otp"
  | "identity_recipient_choice"
  | "identity_proof";

const SKIP_CONVENTION_ROOT = new Set(["id", "status", "updatedAt"]);
const SKIP_OFFER_ROOT = new Set(["id", "status", "updatedAt", "companyName"]);
const SKIP_APPLICATION_ROOT = new Set(["id", "offerId", "status", "updatedAt"]);

function parseTs(raw: string | undefined | null): Date {
  if (!raw) return new Date();
  const d = new Date(raw);
  return Number.isNaN(d.getTime()) ? new Date() : d;
}

export async function stagesDbReady(): Promise<string | null> {
  if (!isEntCoreDbEnabled()) return null;
  return resolveCurrentEtablissementId();
}

function asConvention(row: Record<string, unknown>, fallbackId: string): StageConvention | null {
  const id = String(row.id ?? fallbackId ?? "").trim();
  if (!id) return null;
  if (!row.student || typeof row.student !== "object") return null;
  if (!row.company || typeof row.company !== "object") return null;
  if (!row.schedule || typeof row.schedule !== "object") return null;
  return { ...row, id } as StageConvention;
}

function asOffer(row: Record<string, unknown>, fallbackId: string): StageOffer | null {
  const id = String(row.id ?? fallbackId ?? "").trim();
  if (!id) return null;
  if (!row.companyName && !row.kind) return null;
  return { ...row, id } as StageOffer;
}

/** Chemins EAV nécessaires à l’index léger (évite d’hydrater signatures / historique). */
export const CONVENTION_INDEX_ATTR_PATHS = [
  "student.firstName",
  "student.lastName",
  "student.className",
  "student.level",
  "company.name",
  "internshipKind",
  "schedule.periodStart",
  "schedule.periodEnd",
  "schoolYear",
  "stageLabel",
  "teacherReferent.email",
] as const;

export function conventionToIndexEntry(c: StageConvention): StageConventionIndexEntry {
  return {
    id: c.id,
    status: c.status,
    studentName: `${c.student.firstName} ${c.student.lastName}`.trim(),
    className: c.student.className,
    level: c.student.level,
    companyName: c.company.name,
    internshipKind: c.internshipKind,
    periodStart: c.schedule.periodStart,
    periodEnd: c.schedule.periodEnd,
    schoolYear: c.schoolYear,
    updatedAt: c.updatedAt,
    stageLabel: c.stageLabel?.trim() || undefined,
    teacherReferentEmail: c.teacherReferent.email?.toLowerCase() || undefined,
  };
}

function offerToIndexEntry(o: StageOffer): StageOfferIndexEntry {
  return {
    id: o.id,
    kind: o.kind,
    status: o.status,
    companyName: o.companyName,
    targetLevels: o.targetLevels,
    schoolYear: o.schoolYear,
    createdAt: o.createdAt,
  };
}

/* -------------------------------------------------------------------------- */
/* Conventions                                                                */
/* -------------------------------------------------------------------------- */

function groupAttrsByConventionId(
  attrs: { id: string; path: string; value: string }[],
): Map<string, { path: string; value: string }[]> {
  const byId = new Map<string, { path: string; value: string }[]>();
  for (const a of attrs) {
    const list = byId.get(a.id) ?? [];
    list.push({ path: a.path, value: a.value });
    byId.set(a.id, list);
  }
  return byId;
}

function conventionFromMainAndAttrs(
  m: typeof stageConvention.$inferSelect,
  attrRows: { path: string; value: string }[],
): StageConvention | null {
  const inflated = inflateFromAttrs(attrRows);
  return asConvention(
    {
      ...inflated,
      id: m.id,
      status: m.status ?? inflated.status,
      updatedAt: m.updatedAt?.toISOString?.() ?? inflated.updatedAt,
    },
    m.id,
  );
}

export async function getConventionsFromDb(
  etablissementId: string,
  ids: string[],
): Promise<StageConvention[]> {
  const unique = [...new Set(ids.map((id) => id.trim()).filter(Boolean))];
  if (unique.length === 0) return [];

  const db = getDb();
  const out: StageConvention[] = [];
  const chunkSize = 500;

  for (let i = 0; i < unique.length; i += chunkSize) {
    const chunk = unique.slice(i, i + chunkSize);
    const [mains, attrs] = await Promise.all([
      db
        .select()
        .from(stageConvention)
        .where(
          and(
            eq(stageConvention.etablissementId, etablissementId),
            inArray(stageConvention.id, chunk),
          ),
        ),
      db
        .select({
          id: stageConventionAttr.conventionId,
          path: stageConventionAttr.path,
          value: stageConventionAttr.value,
        })
        .from(stageConventionAttr)
        .where(
          and(
            eq(stageConventionAttr.etablissementId, etablissementId),
            inArray(stageConventionAttr.conventionId, chunk),
          ),
        ),
    ]);
    const byId = groupAttrsByConventionId(attrs);
    for (const m of mains) {
      const c = conventionFromMainAndAttrs(m, byId.get(m.id) ?? []);
      if (c) out.push(c);
      else console.error("[stage-db] convention illisible", m.id);
    }
  }

  return out;
}

export async function listConventionsFromDb(etablissementId: string): Promise<StageConvention[]> {
  const db = getDb();
  const mains = await db
    .select({ id: stageConvention.id })
    .from(stageConvention)
    .where(eq(stageConvention.etablissementId, etablissementId));
  const list = await getConventionsFromDb(etablissementId, mains.map((m) => m.id));
  return list.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
}

export async function getConventionFromDb(
  etablissementId: string,
  id: string,
): Promise<StageConvention | null> {
  const db = getDb();
  const [m] = await db
    .select()
    .from(stageConvention)
    .where(and(eq(stageConvention.etablissementId, etablissementId), eq(stageConvention.id, id)))
    .limit(1);
  if (!m) return null;
  const attrs = await db
    .select({
      path: stageConventionAttr.path,
      value: stageConventionAttr.value,
    })
    .from(stageConventionAttr)
    .where(
      and(
        eq(stageConventionAttr.etablissementId, etablissementId),
        eq(stageConventionAttr.conventionId, m.id),
      ),
    );
  const convention = conventionFromMainAndAttrs(m, attrs);
  if (!convention) {
    console.error("[stage-db] convention illisible", m.id);
  }
  return convention;
}

type StageDbTx = Parameters<Parameters<ReturnType<typeof getDb>["transaction"]>[0]>[0];

async function acquireLegacyMigrationAdvisoryLock(
  tx: StageDbTx,
  etablissementId: string,
): Promise<void> {
  if (isPgliteIntegrationTest()) return;
  await tx.execute(
    sql`SELECT pg_advisory_xact_lock(${STAGE_LEGACY_MIGRATION_LOCK_NS}, hashtext(${etablissementId}))`,
  );
}

/** Ne bloque pas la lecture d’index si une autre transaction migre déjà. */
async function tryAcquireLegacyMigrationAdvisoryLock(
  tx: StageDbTx,
  etablissementId: string,
): Promise<boolean> {
  if (isPgliteIntegrationTest()) return true;
  const result = await tx.execute(
    sql`SELECT pg_try_advisory_xact_lock(${STAGE_LEGACY_MIGRATION_LOCK_NS}, hashtext(${etablissementId})) AS locked`,
  );
  const rows = Array.isArray(result)
    ? result
    : (result as { rows?: Array<{ locked: boolean }> }).rows ?? [];
  const row = rows[0] as { locked?: boolean } | undefined;
  return row?.locked === true;
}

async function legacyConventionIdsBlockedByGlobalPk(
  queryable: StageDbTx,
  etablissementId: string,
  legacyIds: Iterable<string>,
): Promise<Set<string>> {
  const blocked = new Set<string>();
  for (const rawId of legacyIds) {
    const id = String(rawId ?? "").trim();
    if (!id) continue;
    const [row] = await queryable
      .select({ etablissementId: stageConvention.etablissementId })
      .from(stageConvention)
      .where(eq(stageConvention.id, id))
      .limit(1);
    if (row && row.etablissementId !== etablissementId) blocked.add(id);
  }
  return blocked;
}

function logLegacyConventionGlobalPkBlocked(etablissementId: string, ids: string[]): void {
  if (ids.length === 0) return;
  console.warn("[stage-db] conventions legacy ignorées (id PK global autre établissement)", {
    etablissementId,
    count: ids.length,
    ids,
  });
}

async function conventionMainRowExists(etablissementId: string, id: string): Promise<boolean> {
  const db = getDb();
  const [row] = await db
    .select({ id: stageConvention.id })
    .from(stageConvention)
    .where(
      and(eq(stageConvention.etablissementId, etablissementId), eq(stageConvention.id, id.trim())),
    )
    .limit(1);
  return Boolean(row);
}

async function insertConventionFromLegacyIfAbsentInTx(
  tx: StageDbTx,
  etablissementId: string,
  convention: StageConvention,
): Promise<boolean> {
  const id = String(convention.id).trim();
  if (!id) return false;
  const status = String(convention.status ?? "");
  const updatedAt = parseTs(convention.updatedAt);

  const rest: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(convention as unknown as Record<string, unknown>)) {
    if (SKIP_CONVENTION_ROOT.has(k)) continue;
    rest[k] = v;
  }
  const attrs = flattenToAttrs(rest);

  const inserted = await tx
    .insert(stageConvention)
    .values({
      id,
      etablissementId,
      status,
      updatedAt,
    })
    .onConflictDoNothing({ target: stageConvention.id })
    .returning({ id: stageConvention.id });
  if (inserted.length === 0) {
    const [existing] = await tx
      .select({ etablissementId: stageConvention.etablissementId })
      .from(stageConvention)
      .where(eq(stageConvention.id, id))
      .limit(1);
    if (existing && existing.etablissementId !== etablissementId) {
      logLegacyConventionGlobalPkBlocked(etablissementId, [id]);
    }
    return false;
  }

  if (attrs.length > 0) {
    const chunk = 80;
    for (let i = 0; i < attrs.length; i += chunk) {
      await tx.insert(stageConventionAttr).values(
        attrs.slice(i, i + chunk).map((a) => ({
          etablissementId,
          conventionId: id,
          path: a.path,
          value: a.value,
        })),
      );
    }
  }
  return true;
}

/** Insère une convention legacy uniquement si l’id n’existe pas encore (jamais d’écrasement). */
export async function insertConventionFromLegacyIfAbsent(
  etablissementId: string,
  convention: StageConvention,
): Promise<boolean> {
  const db = getDb();
  let inserted = false;
  await db.transaction(async (tx) => {
    await acquireLegacyMigrationAdvisoryLock(tx, etablissementId);
    inserted = await insertConventionFromLegacyIfAbsentInTx(tx, etablissementId, convention);
  });
  return inserted;
}

export async function upsertConventionInDb(
  etablissementId: string,
  convention: StageConvention,
): Promise<void> {
  const db = getDb();
  const id = String(convention.id).trim();
  if (!id) throw new Error("[stage-db] convention.id requis");
  const status = String(convention.status ?? "");
  const updatedAt = parseTs(convention.updatedAt);

  const rest: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(convention as unknown as Record<string, unknown>)) {
    if (SKIP_CONVENTION_ROOT.has(k)) continue;
    rest[k] = v;
  }
  const attrs = flattenToAttrs(rest);

  await db.transaction(async (tx) => {
    await tx
      .insert(stageConvention)
      .values({
        id,
        etablissementId,
        status,
        updatedAt,
      })
      .onConflictDoUpdate({
        target: stageConvention.id,
        set: {
          etablissementId,
          status,
          updatedAt,
        },
      });

    await tx
      .delete(stageConventionAttr)
      .where(
        and(
          eq(stageConventionAttr.etablissementId, etablissementId),
          eq(stageConventionAttr.conventionId, id),
        ),
      );
    if (attrs.length > 0) {
      const chunk = 80;
      for (let i = 0; i < attrs.length; i += chunk) {
        await tx.insert(stageConventionAttr).values(
          attrs.slice(i, i + chunk).map((a) => ({
            etablissementId,
            conventionId: id,
            path: a.path,
            value: a.value,
          })),
        );
      }
    }
  });
}

export function conventionIndexEntriesFromDbRows(
  mains: Array<typeof stageConvention.$inferSelect>,
  attrs: Array<{ id: string; path: string; value: string }>,
): StageConventionIndexEntry[] {
  const byId = groupAttrsByConventionId(attrs);
  return mains
    .map((m) => {
      const convention = conventionFromMainAndAttrs(m, byId.get(m.id) ?? []);
      if (!convention) return null;
      return conventionToIndexEntry(convention);
    })
    .filter((e): e is StageConventionIndexEntry => e !== null)
    .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
}

/** Nombre maximal de requêtes SQL pour l’index (2 par chunk de 500 conventions). */
export const CONVENTION_INDEX_MAX_DB_QUERIES_PER_CHUNK = 2;

const LEGACY_CONVENTIONS_COLLECTION = "stages__conventions";
const CONVENTIONS_LEGACY_MARKER_KIND: StageTokenKind = "auto_purge";
/** Verrou advisory par établissement (namespace fixe + hashtext(etablissement_id)). */
const STAGE_LEGACY_MIGRATION_LOCK_NS = 584_921;
const LEGACY_MIGRATION_ERROR_BACKOFF_MS = 60_000;

/** Établissements dont le marqueur legacy est confirmé (évite les lectures SQL répétées). */
const conventionsLegacyMigrationDone = new Set<string>();
/** Après échec migration lazy : ne pas réessayer immédiatement à chaque lecture d’index. */
const legacyMigrationBackoffUntil = new Map<string, number>();

let legacyCollectConventionIdsCallsForTest = 0;

/** Réinitialise caches mémoire legacy (tests PGlite uniquement). */
export function resetConventionsLegacyMigrationMemoryForTests(): void {
  conventionsLegacyMigrationDone.clear();
  legacyMigrationBackoffUntil.clear();
  legacyCollectConventionIdsCallsForTest = 0;
}

/** Compteur d’appels à `collectLegacyConventionIds` (tests PGlite uniquement). */
export function getLegacyCollectConventionIdsCallsForTests(): number {
  return legacyCollectConventionIdsCallsForTest;
}

function legacyMigrationMarkerStorageKey(etablissementId: string): string {
  return `conventions_legacy_synced:${etablissementId}`;
}

function legacyMigrationMarkerTokenPk(etablissementId: string): string {
  return `${CONVENTIONS_LEGACY_MARKER_KIND}:${legacyMigrationMarkerStorageKey(etablissementId)}`;
}

function legacyMigrationMarkerCompleteFromAttrs(
  attrRows: Array<{ path: string; value: string }>,
): boolean {
  if (attrRows.length === 0) return false;
  const nested = inflateFromAttrs(attrRows);
  return nested.complete === true;
}

function noteConventionsLegacyMigrationComplete(etablissementId: string): void {
  conventionsLegacyMigrationDone.add(etablissementId);
}

async function isConventionsLegacyMigrationCompleteInTx(
  tx: StageDbTx,
  etablissementId: string,
): Promise<boolean> {
  if (conventionsLegacyMigrationDone.has(etablissementId)) return true;
  const tokenPk = legacyMigrationMarkerTokenPk(etablissementId);
  const rows = await tx
    .select({ path: stageTokenAttr.path, value: stageTokenAttr.value })
    .from(stageTokenAttr)
    .where(
      and(eq(stageTokenAttr.etablissementId, etablissementId), eq(stageTokenAttr.token, tokenPk)),
    );
  const complete = legacyMigrationMarkerCompleteFromAttrs(rows);
  if (complete) noteConventionsLegacyMigrationComplete(etablissementId);
  return complete;
}

export async function isConventionsLegacyMigrationComplete(
  etablissementId: string,
): Promise<boolean> {
  if (conventionsLegacyMigrationDone.has(etablissementId)) return true;
  const db = getDb();
  const tokenPk = legacyMigrationMarkerTokenPk(etablissementId);
  const rows = await db
    .select({ path: stageTokenAttr.path, value: stageTokenAttr.value })
    .from(stageTokenAttr)
    .where(
      and(eq(stageTokenAttr.etablissementId, etablissementId), eq(stageTokenAttr.token, tokenPk)),
    );
  const complete = legacyMigrationMarkerCompleteFromAttrs(rows);
  if (complete) noteConventionsLegacyMigrationComplete(etablissementId);
  return complete;
}

async function writeLegacyMigrationMarkerInTx(tx: StageDbTx, etablissementId: string): Promise<void> {
  const tokenPk = legacyMigrationMarkerTokenPk(etablissementId);
  const updatedAt = new Date();
  await tx
    .insert(stageToken)
    .values({
      token: tokenPk,
      etablissementId,
      kind: CONVENTIONS_LEGACY_MARKER_KIND,
      updatedAt,
    })
    .onConflictDoNothing({ target: stageToken.token });

  const markerAttrs = flattenToAttrs({
    complete: true,
    at: updatedAt.toISOString(),
  });
  await tx
    .delete(stageTokenAttr)
    .where(
      and(eq(stageTokenAttr.etablissementId, etablissementId), eq(stageTokenAttr.token, tokenPk)),
    );
  if (markerAttrs.length > 0) {
    await tx.insert(stageTokenAttr).values(
      markerAttrs.map((a) => ({
        etablissementId,
        token: tokenPk,
        path: a.path,
        value: a.value,
      })),
    );
  }
  noteConventionsLegacyMigrationComplete(etablissementId);
}

async function writeLegacyMigrationMarker(etablissementId: string): Promise<void> {
  const db = getDb();
  await db.transaction(async (tx) => {
    const locked = await tryAcquireLegacyMigrationAdvisoryLock(tx, etablissementId);
    if (!locked) return;
    if (await isConventionsLegacyMigrationCompleteInTx(tx, etablissementId)) return;
    await writeLegacyMigrationMarkerInTx(tx, etablissementId);
  });
}

async function collectLegacyConventionIds(etablissementId: string): Promise<Set<string>> {
  if (isPgliteIntegrationTest()) legacyCollectConventionIdsCallsForTest += 1;
  const ids = new Set<string>();
  const { listCollectionRecords, getCollectionRecord } = await import("@/app/lib/ent-collection-db");
  for (const row of await listCollectionRecords<Record<string, unknown>>(
    etablissementId,
    LEGACY_CONVENTIONS_COLLECTION,
  )) {
    const id = String(row.id ?? "").trim();
    if (id) ids.add(id);
  }
  const indexRec = await getCollectionRecord<Record<string, unknown>>(
    etablissementId,
    "stages",
    "conventions-index",
  );
  if (indexRec) {
    const raw =
      "__root" in indexRec && Array.isArray(indexRec.__root)
        ? indexRec.__root
        : Array.isArray(indexRec)
          ? indexRec
          : null;
    if (Array.isArray(raw)) {
      for (const entry of raw) {
        const id =
          entry && typeof entry === "object"
            ? String((entry as { id?: string }).id ?? "").trim()
            : "";
        if (id) ids.add(id);
      }
    }
  }
  return ids;
}

async function typedConventionIdSet(etablissementId: string): Promise<Set<string>> {
  const db = getDb();
  const rows = await db
    .select({ id: stageConvention.id })
    .from(stageConvention)
    .where(eq(stageConvention.etablissementId, etablissementId));
  return new Set(rows.map((r) => r.id));
}

/** Ids encore en collection legacy mais migrables (asConvention OK) et absents des tables typées. */
async function legacyConventionIdsPendingMigration(etablissementId: string): Promise<string[]> {
  const legacyIds = await collectLegacyConventionIds(etablissementId);
  if (legacyIds.size === 0) return [];
  const typed = await typedConventionIdSet(etablissementId);
  const db = getDb();
  const blockedGlobal = await legacyConventionIdsBlockedByGlobalPk(db, etablissementId, legacyIds);
  if (blockedGlobal.size > 0) {
    logLegacyConventionGlobalPkBlocked(etablissementId, [...blockedGlobal]);
  }
  const { getCollectionRecord } = await import("@/app/lib/ent-collection-db");
  const pending: string[] = [];
  for (const id of legacyIds) {
    if (typed.has(id) || blockedGlobal.has(id)) continue;
    const one = await getCollectionRecord<Record<string, unknown>>(
      etablissementId,
      LEGACY_CONVENTIONS_COLLECTION,
      id,
    );
    if (!one) continue;
    if (asConvention(one, id)) pending.push(id);
  }
  return pending;
}

async function migrateLegacyConventionRowsInTx(
  tx: StageDbTx,
  etablissementId: string,
  typedIds: Set<string>,
  prefetched: {
    legacyRows: Record<string, unknown>[];
    indexLegacyById: Map<string, Record<string, unknown>>;
  },
): Promise<void> {
  for (const row of prefetched.legacyRows) {
    const id = String(row.id ?? "").trim();
    if (!id || typedIds.has(id)) continue;
    const convention = asConvention(row, id);
    if (!convention) continue;
    const inserted = await insertConventionFromLegacyIfAbsentInTx(tx, etablissementId, convention);
    if (inserted) typedIds.add(id);
  }

  for (const [id, one] of prefetched.indexLegacyById) {
    if (!id || typedIds.has(id)) continue;
    const convention = asConvention(one, id);
    if (!convention) continue;
    const inserted = await insertConventionFromLegacyIfAbsentInTx(tx, etablissementId, convention);
    if (inserted) typedIds.add(id);
  }
}

async function runLegacyConventionMigrationTransaction(
  etablissementId: string,
  options?: { waitForAdvisoryLock?: boolean },
): Promise<void> {
  const { listCollectionRecords, getCollectionRecord } = await import(
    "@/app/lib/ent-collection-db"
  );
  const legacyRows = await listCollectionRecords<Record<string, unknown>>(
    etablissementId,
    LEGACY_CONVENTIONS_COLLECTION,
  );
  const indexRec = await getCollectionRecord<Record<string, unknown>>(
    etablissementId,
    "stages",
    "conventions-index",
  );
  const indexLegacyById = new Map<string, Record<string, unknown>>();
  if (indexRec) {
    const raw =
      "__root" in indexRec && Array.isArray(indexRec.__root)
        ? indexRec.__root
        : Array.isArray(indexRec)
          ? indexRec
          : null;
    if (Array.isArray(raw)) {
      for (const entry of raw) {
        const id =
          entry && typeof entry === "object"
            ? String((entry as { id?: string }).id ?? "").trim()
            : "";
        if (!id) continue;
        const one = await getCollectionRecord<Record<string, unknown>>(
          etablissementId,
          LEGACY_CONVENTIONS_COLLECTION,
          id,
        );
        if (one) indexLegacyById.set(id, one);
      }
    }
  }

  const allLegacyIds = await collectLegacyConventionIds(etablissementId);

  const db = getDb();
  await db.transaction(async (tx) => {
    const locked = options?.waitForAdvisoryLock
      ? await (async () => {
          await acquireLegacyMigrationAdvisoryLock(tx, etablissementId);
          return true;
        })()
      : await tryAcquireLegacyMigrationAdvisoryLock(tx, etablissementId);
    if (!locked) return;
    if (await isConventionsLegacyMigrationCompleteInTx(tx, etablissementId)) return;

    const typedIds = new Set(
      (
        await tx
          .select({ id: stageConvention.id })
          .from(stageConvention)
          .where(eq(stageConvention.etablissementId, etablissementId))
      ).map((r) => r.id),
    );

    const blockedGlobal = await legacyConventionIdsBlockedByGlobalPk(
      tx,
      etablissementId,
      allLegacyIds,
    );
    if (blockedGlobal.size > 0) {
      logLegacyConventionGlobalPkBlocked(etablissementId, [...blockedGlobal]);
    }

    await migrateLegacyConventionRowsInTx(tx, etablissementId, typedIds, {
      legacyRows,
      indexLegacyById,
    });

    let stillPendingMigratable = false;
    for (const id of allLegacyIds) {
      if (typedIds.has(id) || blockedGlobal.has(id)) continue;
      const row =
        legacyRows.find((r) => String(r.id ?? "").trim() === id) ?? indexLegacyById.get(id);
      if (row && asConvention(row, id)) {
        stillPendingMigratable = true;
        break;
      }
    }
    if (!stillPendingMigratable) {
      await writeLegacyMigrationMarkerInTx(tx, etablissementId);
    }
  });
}

/**
 * Repli lecture index : migration legacy idempotente tant que des conventions
 * `ent_collection` restent à copier (marqueur persistant par établissement).
 */
async function ensureLegacyConventionsForIndexRead(etablissementId: string): Promise<void> {
  if (conventionsLegacyMigrationDone.has(etablissementId)) return;
  if (await isConventionsLegacyMigrationComplete(etablissementId)) return;

  const pending = await legacyConventionIdsPendingMigration(etablissementId);
  if (pending.length === 0) {
    await writeLegacyMigrationMarker(etablissementId);
    return;
  }

  await runLegacyConventionMigrationTransaction(etablissementId, { waitForAdvisoryLock: false });
}

export async function listConventionIndexFromDb(
  etablissementId: string,
): Promise<StageConventionIndexEntry[]> {
  const backoffUntil = legacyMigrationBackoffUntil.get(etablissementId);
  if (backoffUntil === undefined || Date.now() >= backoffUntil) {
    try {
      await ensureLegacyConventionsForIndexRead(etablissementId);
    } catch (error) {
      legacyMigrationBackoffUntil.set(
        etablissementId,
        Date.now() + LEGACY_MIGRATION_ERROR_BACKOFF_MS,
      );
      console.error("[stage-db] migration legacy conventions (lecture index)", etablissementId, error);
    }
  }

  const db = getDb();
  return db.transaction(
    async (tx) => {
      const [mains, attrs] = await Promise.all([
        tx
          .select()
          .from(stageConvention)
          .where(eq(stageConvention.etablissementId, etablissementId)),
        tx
          .select({
            id: stageConventionAttr.conventionId,
            path: stageConventionAttr.path,
            value: stageConventionAttr.value,
          })
          .from(stageConventionAttr)
          .where(
            and(
              eq(stageConventionAttr.etablissementId, etablissementId),
              inArray(stageConventionAttr.path, [...CONVENTION_INDEX_ATTR_PATHS]),
            ),
          ),
      ]);
      return conventionIndexEntriesFromDbRows(mains, attrs);
    },
    { isolationLevel: "repeatable read", accessMode: "read only" },
  );
}

/**
 * Copie les conventions encore en `ent_collection` vers `stage_convention`
 * (upsert unitaire, jamais de wipe). Les ids déjà typés ne sont pas écrasés.
 */
export async function ensureConventionsMigratedFromCollection(
  etablissementId: string,
): Promise<StageConvention[]> {
  await runLegacyConventionMigrationTransaction(etablissementId, { waitForAdvisoryLock: true });
  return listConventionsFromDb(etablissementId);
}

export async function getConventionFromDbOrMigrate(
  etablissementId: string,
  id: string,
): Promise<StageConvention | null> {
  const trimmed = id.trim();
  if (!trimmed) return null;

  if (await conventionMainRowExists(etablissementId, trimmed)) {
    const hit = await getConventionFromDb(etablissementId, trimmed);
    if (hit) return hit;
    console.error(
      "[stage-db] convention en base illisible, repli legacy ignoré (pas d’écrasement)",
      trimmed,
    );
    return null;
  }

  const { getCollectionRecord } = await import("@/app/lib/ent-collection-db");
  const legacy = await getCollectionRecord<Record<string, unknown>>(
    etablissementId,
    LEGACY_CONVENTIONS_COLLECTION,
    trimmed,
  );
  if (!legacy) return null;
  const convention = asConvention(legacy, trimmed);
  if (!convention) return null;
  await insertConventionFromLegacyIfAbsent(etablissementId, convention);
  return getConventionFromDb(etablissementId, trimmed);
}

/* -------------------------------------------------------------------------- */
/* Offers                                                                     */
/* -------------------------------------------------------------------------- */

async function hydrateOffer(
  etablissementId: string,
  m: typeof stageOffer.$inferSelect,
): Promise<StageOffer> {
  const db = getDb();
  const attrs = await db
    .select()
    .from(stageOfferAttr)
    .where(
      and(eq(stageOfferAttr.etablissementId, etablissementId), eq(stageOfferAttr.offerId, m.id)),
    );
  const inflated = inflateFromAttrs(attrs.map((a) => ({ path: a.path, value: a.value })));
  const offer = asOffer(
    {
      ...inflated,
      id: m.id,
      status: m.status ?? inflated.status,
      companyName: inflated.companyName ?? m.title ?? "",
      updatedAt: m.updatedAt?.toISOString?.() ?? inflated.updatedAt,
    },
    m.id,
  );
  if (!offer) throw new Error(`[stage-db] Offre ${m.id} illisible.`);
  return offer;
}

export async function listOffersFromDb(etablissementId: string): Promise<StageOffer[]> {
  const db = getDb();
  const mains = await db
    .select()
    .from(stageOffer)
    .where(eq(stageOffer.etablissementId, etablissementId));
  const out: StageOffer[] = [];
  for (const m of mains) {
    try {
      out.push(await hydrateOffer(etablissementId, m));
    } catch (e) {
      console.error("[stage-db] hydrate offer", m.id, e);
    }
  }
  return out;
}

export async function getOfferFromDb(
  etablissementId: string,
  id: string,
): Promise<StageOffer | null> {
  const db = getDb();
  const [m] = await db
    .select()
    .from(stageOffer)
    .where(and(eq(stageOffer.etablissementId, etablissementId), eq(stageOffer.id, id)))
    .limit(1);
  if (!m) return null;
  return hydrateOffer(etablissementId, m);
}

export async function upsertOfferInDb(etablissementId: string, offer: StageOffer): Promise<void> {
  const db = getDb();
  const id = String(offer.id).trim();
  if (!id) throw new Error("[stage-db] offer.id requis");
  const status = String(offer.status ?? "");
  const title = String(offer.companyName ?? "").trim();
  const updatedAt = parseTs(offer.updatedAt ?? offer.createdAt);

  await db
    .insert(stageOffer)
    .values({ id, etablissementId, title, status, updatedAt })
    .onConflictDoUpdate({
      target: stageOffer.id,
      set: { etablissementId, title, status, updatedAt },
    });

  const rest: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(offer as unknown as Record<string, unknown>)) {
    if (SKIP_OFFER_ROOT.has(k)) continue;
    rest[k] = v;
  }
  // companyName utile en attrs aussi (title = projection)
  rest.companyName = offer.companyName;

  await db
    .delete(stageOfferAttr)
    .where(
      and(eq(stageOfferAttr.etablissementId, etablissementId), eq(stageOfferAttr.offerId, id)),
    );
  const attrs = flattenToAttrs(rest);
  if (attrs.length > 0) {
    const chunk = 80;
    for (let i = 0; i < attrs.length; i += chunk) {
      await db.insert(stageOfferAttr).values(
        attrs.slice(i, i + chunk).map((a) => ({
          etablissementId,
          offerId: id,
          path: a.path,
          value: a.value,
        })),
      );
    }
  }
}

export async function listOfferIndexFromDb(
  etablissementId: string,
): Promise<StageOfferIndexEntry[]> {
  const list = await ensureOffersMigratedFromCollection(etablissementId);
  return list.map(offerToIndexEntry);
}

export async function ensureOffersMigratedFromCollection(
  etablissementId: string,
): Promise<StageOffer[]> {
  const existing = await listOffersFromDb(etablissementId);
  const existingIds = new Set(existing.map((o) => o.id));
  const { listCollectionRecords, getCollectionRecord } = await import(
    "@/app/lib/ent-collection-db"
  );
  const legacyRows = await listCollectionRecords<Record<string, unknown>>(
    etablissementId,
    "stages__offers",
  );
  for (const row of legacyRows) {
    const id = String(row.id ?? "").trim();
    if (!id || existingIds.has(id)) continue;
    const offer = asOffer(row, id);
    if (!offer) continue;
    await upsertOfferInDb(etablissementId, offer);
    existingIds.add(id);
  }

  const indexRec = await getCollectionRecord<Record<string, unknown>>(
    etablissementId,
    "stages",
    "offers-index",
  );
  if (indexRec) {
    const raw =
      "__root" in indexRec && Array.isArray(indexRec.__root)
        ? indexRec.__root
        : Array.isArray(indexRec)
          ? indexRec
          : null;
    if (Array.isArray(raw)) {
      for (const entry of raw) {
        const id =
          entry && typeof entry === "object"
            ? String((entry as { id?: string }).id ?? "").trim()
            : "";
        if (!id || existingIds.has(id)) continue;
        const one = await getCollectionRecord<Record<string, unknown>>(
          etablissementId,
          "stages__offers",
          id,
        );
        if (!one) continue;
        const offer = asOffer(one, id);
        if (!offer) continue;
        await upsertOfferInDb(etablissementId, offer);
        existingIds.add(id);
      }
    }
  }
  return listOffersFromDb(etablissementId);
}

export async function getOfferFromDbOrMigrate(
  etablissementId: string,
  id: string,
): Promise<StageOffer | null> {
  const hit = await getOfferFromDb(etablissementId, id);
  if (hit) return hit;
  const { getCollectionRecord } = await import("@/app/lib/ent-collection-db");
  const legacy = await getCollectionRecord<Record<string, unknown>>(
    etablissementId,
    "stages__offers",
    id,
  );
  if (!legacy) return null;
  const offer = asOffer(legacy, id);
  if (!offer) return null;
  await upsertOfferInDb(etablissementId, offer);
  return getOfferFromDb(etablissementId, id);
}

/* -------------------------------------------------------------------------- */
/* Applications (1 ligne / candidature)                                       */
/* -------------------------------------------------------------------------- */

async function hydrateApplication(
  etablissementId: string,
  m: typeof stageApplication.$inferSelect,
): Promise<StageOfferApplication> {
  const db = getDb();
  const attrs = await db
    .select()
    .from(stageApplicationAttr)
    .where(
      and(
        eq(stageApplicationAttr.etablissementId, etablissementId),
        eq(stageApplicationAttr.applicationId, m.id),
      ),
    );
  const inflated = inflateFromAttrs(attrs.map((a) => ({ path: a.path, value: a.value })));
  return {
    id: m.id,
    offerId: String(m.offerId ?? inflated.offerId ?? ""),
    conventionId: String(inflated.conventionId ?? ""),
    studentFirstName: String(inflated.studentFirstName ?? ""),
    studentLastName: String(inflated.studentLastName ?? ""),
    studentClassName: String(inflated.studentClassName ?? ""),
    studentLevel: String(inflated.studentLevel ?? ""),
    createdAt: String(inflated.createdAt ?? m.updatedAt?.toISOString?.() ?? ""),
  };
}

export async function listApplicationsForOfferFromDb(
  etablissementId: string,
  offerId: string,
): Promise<StageOfferApplication[]> {
  await ensureApplicationsMigratedForOffer(etablissementId, offerId);
  const db = getDb();
  const mains = await db
    .select()
    .from(stageApplication)
    .where(
      and(
        eq(stageApplication.etablissementId, etablissementId),
        eq(stageApplication.offerId, offerId),
      ),
    );
  const out: StageOfferApplication[] = [];
  for (const m of mains) {
    out.push(await hydrateApplication(etablissementId, m));
  }
  return out.sort((a, b) => b.createdAt.localeCompare(a.createdAt));
}

export async function upsertApplicationInDb(
  etablissementId: string,
  app: StageOfferApplication,
): Promise<void> {
  const db = getDb();
  const id = String(app.id).trim();
  if (!id) throw new Error("[stage-db] application.id requis");
  const offerId = String(app.offerId ?? "").trim() || null;
  const updatedAt = parseTs(app.createdAt);

  await db
    .insert(stageApplication)
    .values({ id, etablissementId, offerId, status: null, updatedAt })
    .onConflictDoUpdate({
      target: stageApplication.id,
      set: { etablissementId, offerId, updatedAt },
    });

  const rest: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(app as unknown as Record<string, unknown>)) {
    if (SKIP_APPLICATION_ROOT.has(k)) continue;
    rest[k] = v;
  }

  await db
    .delete(stageApplicationAttr)
    .where(
      and(
        eq(stageApplicationAttr.etablissementId, etablissementId),
        eq(stageApplicationAttr.applicationId, id),
      ),
    );
  const attrs = flattenToAttrs(rest);
  if (attrs.length > 0) {
    await db.insert(stageApplicationAttr).values(
      attrs.map((a) => ({
        etablissementId,
        applicationId: id,
        path: a.path,
        value: a.value,
      })),
    );
  }
}

/**
 * Remplace les candidatures d’une offre (scoped offerId uniquement — pas de wipe tenant).
 */
export async function replaceOfferApplicationsInDb(
  etablissementId: string,
  offerId: string,
  apps: StageOfferApplication[],
): Promise<void> {
  const db = getDb();
  const existing = await db
    .select({ id: stageApplication.id })
    .from(stageApplication)
    .where(
      and(
        eq(stageApplication.etablissementId, etablissementId),
        eq(stageApplication.offerId, offerId),
      ),
    );
  for (const row of existing) {
    await db
      .delete(stageApplication)
      .where(
        and(
          eq(stageApplication.etablissementId, etablissementId),
          eq(stageApplication.id, row.id),
        ),
      );
  }
  for (const app of apps) {
    await upsertApplicationInDb(etablissementId, { ...app, offerId });
  }
}

async function ensureApplicationsMigratedForOffer(
  etablissementId: string,
  offerId: string,
): Promise<void> {
  const db = getDb();
  const [any] = await db
    .select({ id: stageApplication.id })
    .from(stageApplication)
    .where(
      and(
        eq(stageApplication.etablissementId, etablissementId),
        eq(stageApplication.offerId, offerId),
      ),
    )
    .limit(1);
  if (any) return;

  const { getCollectionRecord } = await import("@/app/lib/ent-collection-db");
  const legacy = await getCollectionRecord<Record<string, unknown>>(
    etablissementId,
    "stages__offer-applications",
    offerId,
  );
  if (!legacy) return;
  const list =
    "__root" in legacy && Array.isArray(legacy.__root)
      ? legacy.__root
      : Array.isArray(legacy)
        ? legacy
        : null;
  if (!Array.isArray(list) || list.length === 0) return;
  for (const raw of list) {
    if (!raw || typeof raw !== "object") continue;
    const app = raw as StageOfferApplication;
    if (!app.id) continue;
    await upsertApplicationInDb(etablissementId, { ...app, offerId });
  }
}

/* -------------------------------------------------------------------------- */
/* Tokens + docs config / OTP (stage_token)                                   */
/* -------------------------------------------------------------------------- */

export function stageTokenStorageKey(kind: StageTokenKind, key: string): string {
  return `${kind}:${key}`;
}

async function hydrateTokenPayload<T extends Record<string, unknown>>(
  etablissementId: string,
  tokenPk: string,
): Promise<T | null> {
  const db = getDb();
  const [m] = await db
    .select()
    .from(stageToken)
    .where(and(eq(stageToken.etablissementId, etablissementId), eq(stageToken.token, tokenPk)))
    .limit(1);
  if (!m) return null;
  const attrs = await db
    .select()
    .from(stageTokenAttr)
    .where(
      and(eq(stageTokenAttr.etablissementId, etablissementId), eq(stageTokenAttr.token, tokenPk)),
    );
  return inflateFromAttrs(attrs.map((a) => ({ path: a.path, value: a.value }))) as T;
}

export async function upsertStageTokenInDb(
  etablissementId: string,
  kind: StageTokenKind,
  key: string,
  payload: Record<string, unknown>,
): Promise<void> {
  const db = getDb();
  const tokenPk = stageTokenStorageKey(kind, key);
  const updatedAt = new Date();

  await db
    .insert(stageToken)
    .values({ token: tokenPk, etablissementId, kind, updatedAt })
    .onConflictDoUpdate({
      target: stageToken.token,
      set: { etablissementId, kind, updatedAt },
    });

  await db
    .delete(stageTokenAttr)
    .where(
      and(eq(stageTokenAttr.etablissementId, etablissementId), eq(stageTokenAttr.token, tokenPk)),
    );
  const attrs = flattenToAttrs(payload);
  if (attrs.length > 0) {
    await db.insert(stageTokenAttr).values(
      attrs.map((a) => ({
        etablissementId,
        token: tokenPk,
        path: a.path,
        value: a.value,
      })),
    );
  }
}

export async function getStageTokenFromDb<T extends Record<string, unknown>>(
  etablissementId: string,
  kind: StageTokenKind,
  key: string,
): Promise<T | null> {
  const tokenPk = stageTokenStorageKey(kind, key);
  const hit = await hydrateTokenPayload<T>(etablissementId, tokenPk);
  if (hit) return hit;
  return migrateOneTokenFromCollection(etablissementId, kind, key);
}

export async function deleteStageTokenFromDb(
  etablissementId: string,
  kind: StageTokenKind,
  key: string,
): Promise<void> {
  const db = getDb();
  const tokenPk = stageTokenStorageKey(kind, key);
  await db
    .delete(stageToken)
    .where(and(eq(stageToken.etablissementId, etablissementId), eq(stageToken.token, tokenPk)));
}

function collectionForTokenKind(kind: StageTokenKind): string | null {
  switch (kind) {
    case "sign":
      return "stages__sign-tokens";
    case "sign_code":
      return "stages__sign-code-lookup";
    case "student":
      return "stages__student-tokens";
    case "offer_candidature":
      return "stages__offer-candidature-tokens";
    case "periods":
      return "stages__periods";
    case "referents":
      return "stages__referents";
    case "constraints":
      return "stages__constraints";
    case "watchers":
      return "stages__watchers";
    case "identity_otp":
      return "stages__identity-otp";
    case "identity_recipient_choice":
      return "stages__identity-recipient-choice";
    case "identity_proof":
      return "stages__identity-proof";
    case "auto_purge":
      return "stages";
    default:
      return null;
  }
}

function payloadFromCollectionRow(row: Record<string, unknown>): Record<string, unknown> {
  if ("__root" in row && Object.keys(row).every((k) => k === "__root" || k === "id")) {
    const root = row.__root;
    if (root && typeof root === "object" && !Array.isArray(root)) {
      return { ...(root as Record<string, unknown>) };
    }
    return { __root: root as unknown };
  }
  const copy = { ...row };
  delete copy.id;
  return copy;
}

async function migrateOneTokenFromCollection<T extends Record<string, unknown>>(
  etablissementId: string,
  kind: StageTokenKind,
  key: string,
): Promise<T | null> {
  const collection = collectionForTokenKind(kind);
  if (!collection) return null;
  const { getCollectionRecord } = await import("@/app/lib/ent-collection-db");
  const recordId = kind === "auto_purge" ? "auto-purge-state" : key;
  const legacy = await getCollectionRecord<Record<string, unknown>>(
    etablissementId,
    collection,
    recordId,
  );
  if (!legacy) return null;
  const obj = payloadFromCollectionRow(legacy);
  await upsertStageTokenInDb(etablissementId, kind, key, obj);
  return hydrateTokenPayload<T>(etablissementId, stageTokenStorageKey(kind, key));
}

/** Migre en masse les tokens d’un kind depuis ent_collection (ids manquants seulement). */
export async function ensureTokensMigratedFromCollection(
  etablissementId: string,
  kind: StageTokenKind,
): Promise<number> {
  const collection = collectionForTokenKind(kind);
  if (!collection) return 0;

  // `stages` contient aussi offers-index / conventions-index — ne prendre que auto-purge-state.
  if (kind === "auto_purge") {
    const hit = await migrateOneTokenFromCollection(etablissementId, "auto_purge", "state");
    return hit ? 1 : 0;
  }

  const { listCollectionRecords } = await import("@/app/lib/ent-collection-db");
  const rows = await listCollectionRecords<Record<string, unknown>>(etablissementId, collection);
  let n = 0;
  for (const row of rows) {
    const storageKey = String(row.id ?? "").trim();
    if (!storageKey) continue;
    const existing = await hydrateTokenPayload(
      etablissementId,
      stageTokenStorageKey(kind, storageKey),
    );
    if (existing) continue;
    await upsertStageTokenInDb(etablissementId, kind, storageKey, payloadFromCollectionRow(row));
    n += 1;
  }
  return n;
}

/* Typed helpers for callers / bridge */

export async function saveSignTokenTyped(
  etablissementId: string,
  token: string,
  ref: StageSignTokenRef,
): Promise<void> {
  await upsertStageTokenInDb(etablissementId, "sign", token, { ...ref });
}

export async function getSignTokenTyped(
  etablissementId: string,
  token: string,
): Promise<StageSignTokenRef | null> {
  return getStageTokenFromDb<StageSignTokenRef>(etablissementId, "sign", token);
}

export async function saveStudentTokenTyped(
  etablissementId: string,
  token: string,
  ref: StageStudentTokenRef,
): Promise<void> {
  await upsertStageTokenInDb(etablissementId, "student", token, { ...ref });
}

export async function getStudentTokenTyped(
  etablissementId: string,
  token: string,
): Promise<StageStudentTokenRef | null> {
  return getStageTokenFromDb<StageStudentTokenRef>(etablissementId, "student", token);
}

export async function saveOfferCandidatureTokenTyped(
  etablissementId: string,
  token: string,
  ref: StageOfferCandidatureTokenRef,
): Promise<void> {
  await upsertStageTokenInDb(etablissementId, "offer_candidature", token, { ...ref });
}

export async function getOfferCandidatureTokenTyped(
  etablissementId: string,
  token: string,
): Promise<StageOfferCandidatureTokenRef | null> {
  return getStageTokenFromDb<StageOfferCandidatureTokenRef>(
    etablissementId,
    "offer_candidature",
    token,
  );
}

export async function saveSignCodeLookupTyped(
  etablissementId: string,
  lookupKey: string,
  ref: StageSignCodeLookupRef,
): Promise<void> {
  await upsertStageTokenInDb(etablissementId, "sign_code", lookupKey, { ...ref });
}

export async function getSignCodeLookupTyped(
  etablissementId: string,
  lookupKey: string,
): Promise<StageSignCodeLookupRef | null> {
  return getStageTokenFromDb<StageSignCodeLookupRef>(etablissementId, "sign_code", lookupKey);
}

/** Backfill one-shot : toutes collections stages → tables typées (idempotent). */
export async function migrateAllStagesFromCollection(etablissementId: string): Promise<{
  conventions: number;
  offers: number;
  tokens: number;
}> {
  const conventions = await ensureConventionsMigratedFromCollection(etablissementId);
  const offers = await ensureOffersMigratedFromCollection(etablissementId);
  const kinds: StageTokenKind[] = [
    "sign",
    "sign_code",
    "student",
    "offer_candidature",
    "periods",
    "referents",
    "constraints",
    "watchers",
    "auto_purge",
    "identity_otp",
    "identity_recipient_choice",
    "identity_proof",
  ];
  let tokens = 0;
  for (const kind of kinds) {
    tokens += await ensureTokensMigratedFromCollection(etablissementId, kind);
  }
  // applications: per offer from offers list + legacy collection
  for (const o of offers) {
    await ensureApplicationsMigratedForOffer(etablissementId, o.id);
  }
  return { conventions: conventions.length, offers: offers.length, tokens };
}
