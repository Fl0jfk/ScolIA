import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { entCollectionAttr, entCollectionRecord } from "@/db/schema";
import { flattenCollectionRecord, inflateFromAttrs } from "@/app/lib/ent-attr-codec";
import {
  isEntCoreDbEnabled,
  resolveCurrentEtablissementId,
} from "@/app/lib/ent-core-db";

export type CollectionName = string;

function statusFromRecord(record: Record<string, unknown>): string | null {
  const s = record.status ?? record.workflowStatus ?? record.state;
  return s == null ? null : String(s);
}

export async function listCollectionRecords<T extends Record<string, unknown>>(
  etablissementId: string,
  collection: CollectionName,
): Promise<T[]> {
  const db = getDb();
  const rows = await db
    .select()
    .from(entCollectionRecord)
    .where(
      and(
        eq(entCollectionRecord.etablissementId, etablissementId),
        eq(entCollectionRecord.collection, collection),
      ),
    );
  if (rows.length === 0) return [];
  const attrs = await db
    .select()
    .from(entCollectionAttr)
    .where(
      and(
        eq(entCollectionAttr.etablissementId, etablissementId),
        eq(entCollectionAttr.collection, collection),
      ),
    );
  const byId = new Map<string, { path: string; value: string }[]>();
  for (const a of attrs) {
    const list = byId.get(a.recordId) ?? [];
    list.push({ path: a.path, value: a.value });
    byId.set(a.recordId, list);
  }
  return rows.map((r) => {
    const obj = inflateFromAttrs(byId.get(r.recordId) ?? []);
    if (!obj.id) obj.id = r.recordId;
    if (r.status && obj.status == null) obj.status = r.status;
    return obj as T;
  });
}

export async function getCollectionRecord<T extends Record<string, unknown>>(
  etablissementId: string,
  collection: CollectionName,
  recordId: string,
): Promise<T | null> {
  const db = getDb();
  const [row] = await db
    .select()
    .from(entCollectionRecord)
    .where(
      and(
        eq(entCollectionRecord.etablissementId, etablissementId),
        eq(entCollectionRecord.collection, collection),
        eq(entCollectionRecord.recordId, recordId),
      ),
    )
    .limit(1);
  if (!row) return null;
  const attrs = await db
    .select()
    .from(entCollectionAttr)
    .where(
      and(
        eq(entCollectionAttr.etablissementId, etablissementId),
        eq(entCollectionAttr.collection, collection),
        eq(entCollectionAttr.recordId, recordId),
      ),
    );
  const obj = inflateFromAttrs(attrs.map((a) => ({ path: a.path, value: a.value })));
  if (!obj.id) obj.id = recordId;
  return obj as T;
}

export async function upsertCollectionRecord(
  etablissementId: string,
  collection: CollectionName,
  recordId: string,
  record: Record<string, unknown>,
): Promise<void> {
  const db = getDb();
  const now = new Date();
  const attrs = flattenCollectionRecord(record);

  await db.transaction(async (tx) => {
    await tx
      .insert(entCollectionRecord)
      .values({
        etablissementId,
        collection,
        recordId,
        status: statusFromRecord(record),
        updatedAt: now,
      })
      .onConflictDoUpdate({
        target: [
          entCollectionRecord.etablissementId,
          entCollectionRecord.collection,
          entCollectionRecord.recordId,
        ],
        set: {
          status: statusFromRecord(record),
          updatedAt: now,
        },
      });
    await tx
      .delete(entCollectionAttr)
      .where(
        and(
          eq(entCollectionAttr.etablissementId, etablissementId),
          eq(entCollectionAttr.collection, collection),
          eq(entCollectionAttr.recordId, recordId),
        ),
      );
    if (attrs.length === 0) return;
    const chunk = 100;
    for (let i = 0; i < attrs.length; i += chunk) {
      await tx.insert(entCollectionAttr).values(
        attrs.slice(i, i + chunk).map((a) => ({
          etablissementId,
          collection,
          recordId,
          path: a.path,
          value: a.value,
        })),
      );
    }
  });
}

export async function deleteCollectionRecord(
  etablissementId: string,
  collection: CollectionName,
  recordId: string,
): Promise<void> {
  const db = getDb();
  await db.transaction(async (tx) => {
    await tx
      .delete(entCollectionAttr)
      .where(
        and(
          eq(entCollectionAttr.etablissementId, etablissementId),
          eq(entCollectionAttr.collection, collection),
          eq(entCollectionAttr.recordId, recordId),
        ),
      );
    await tx
      .delete(entCollectionRecord)
      .where(
        and(
          eq(entCollectionRecord.etablissementId, etablissementId),
          eq(entCollectionRecord.collection, collection),
          eq(entCollectionRecord.recordId, recordId),
        ),
      );
  });
}

export async function replaceCollectionRecords(
  etablissementId: string,
  collection: CollectionName,
  records: { recordId: string; payload: Record<string, unknown> }[],
): Promise<number> {
  const db = getDb();
  await db
    .delete(entCollectionRecord)
    .where(
      and(
        eq(entCollectionRecord.etablissementId, etablissementId),
        eq(entCollectionRecord.collection, collection),
      ),
    );
  for (const r of records) {
    await upsertCollectionRecord(etablissementId, collection, r.recordId, r.payload);
  }
  return records.length;
}

export async function countCollection(
  etablissementId: string,
  collection: CollectionName,
): Promise<number> {
  const db = getDb();
  const rows = await db
    .select({ id: entCollectionRecord.recordId })
    .from(entCollectionRecord)
    .where(
      and(
        eq(entCollectionRecord.etablissementId, etablissementId),
        eq(entCollectionRecord.collection, collection),
      ),
    );
  return rows.length;
}

/** Singleton document (index / config) stocké comme recordId = "_". */
export async function getCollectionSingleton<T extends Record<string, unknown> | unknown[]>(
  etablissementId: string,
  collection: CollectionName,
): Promise<T | null> {
  const row = await getCollectionRecord<{ __root?: T } & Record<string, unknown>>(
    etablissementId,
    collection,
    "_",
  );
  if (!row) return null;
  if ("__root" in row) return row.__root as T;
  return row as T;
}

export async function putCollectionSingleton(
  etablissementId: string,
  collection: CollectionName,
  data: unknown,
): Promise<void> {
  if (Array.isArray(data) || data === null || typeof data !== "object") {
    await upsertCollectionRecord(etablissementId, collection, "_", { __root: data as unknown });
    return;
  }
  await upsertCollectionRecord(
    etablissementId,
    collection,
    "_",
    data as Record<string, unknown>,
  );
}

export async function collectionDbReady(): Promise<string | null> {
  if (!isEntCoreDbEnabled()) return null;
  return resolveCurrentEtablissementId();
}
