import "server-only";

import { and, eq, sql } from "drizzle-orm";
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
  const memoKey = `${etablissementId}\u0000${collection}\u0000${recordId}`;

  // 1) Version de l'enregistrement (lecture par clé primaire, très légère).
  //    `updated_at` en texte pour garder la microseconde (Date JS = milliseconde).
  const [head] = await db
    .select({ version: sql<string>`${entCollectionRecord.updatedAt}::text` })
    .from(entCollectionRecord)
    .where(
      and(
        eq(entCollectionRecord.etablissementId, etablissementId),
        eq(entCollectionRecord.collection, collection),
        eq(entCollectionRecord.recordId, recordId),
      ),
    )
    .limit(1);
  if (!head) {
    recordMemo.delete(memoKey);
    return null;
  }

  // 2) Copie en mémoire encore valable : même version qu'en base.
  const memo = recordMemo.get(memoKey);
  if (memo && memo.version === head.version) {
    return structuredClone(memo.obj) as T;
  }

  // 3) Relecture des attributs. Jointure sur l'enregistrement dans la même
  //    requête : version et attributs proviennent du même instantané.
  const rows = await db
    .select({
      path: entCollectionAttr.path,
      value: entCollectionAttr.value,
      version: sql<string>`${entCollectionRecord.updatedAt}::text`,
    })
    .from(entCollectionAttr)
    .innerJoin(
      entCollectionRecord,
      and(
        eq(entCollectionRecord.etablissementId, entCollectionAttr.etablissementId),
        eq(entCollectionRecord.collection, entCollectionAttr.collection),
        eq(entCollectionRecord.recordId, entCollectionAttr.recordId),
      ),
    )
    .where(
      and(
        eq(entCollectionAttr.etablissementId, etablissementId),
        eq(entCollectionAttr.collection, collection),
        eq(entCollectionAttr.recordId, recordId),
      ),
    );
  const obj = inflateFromAttrs(rows.map((a) => ({ path: a.path, value: a.value })));
  if (!obj.id) obj.id = recordId;
  if (rows.length >= RECORD_MEMO_MIN_ATTRS) {
    rememberRecord(memoKey, rows[0].version, obj);
    return structuredClone(obj) as T;
  }
  return obj as T;
}

/**
 * Copie en mémoire (par instance) des gros documents — fonctionne sans Valkey.
 * Toujours validée par `updated_at` en base avant usage : une écriture faite
 * depuis n'importe quelle instance change la version, la copie est alors relue.
 * Toutes les écritures passent par `upsertCollectionRecord` (version mise à jour
 * dans la même transaction) ou suppriment l'enregistrement.
 * Seuls les documents volumineux sont gardés (les petits sont lus vite),
 * nombre borné, rendus en copie pour qu'un appelant ne modifie pas la mémoire.
 */
const RECORD_MEMO_MIN_ATTRS = 200;
const RECORD_MEMO_MAX_ENTRIES = 100;
const recordMemo = new Map<string, { version: string; obj: Record<string, unknown> }>();

function rememberRecord(key: string, version: string, obj: Record<string, unknown>) {
  recordMemo.delete(key);
  recordMemo.set(key, { version, obj });
  while (recordMemo.size > RECORD_MEMO_MAX_ENTRIES) {
    const oldest = recordMemo.keys().next().value;
    if (oldest === undefined) break;
    recordMemo.delete(oldest);
  }
}

export async function upsertCollectionRecord(
  etablissementId: string,
  collection: CollectionName,
  recordId: string,
  record: Record<string, unknown>,
): Promise<void> {
  const db = getDb();
  const attrs = flattenCollectionRecord(record);

  // Version = horloge base à la microseconde, strictement croissante à chaque
  // écriture : sert à valider la copie mémoire de getCollectionRecord.
  await db.transaction(async (tx) => {
    await tx
      .insert(entCollectionRecord)
      .values({
        etablissementId,
        collection,
        recordId,
        status: statusFromRecord(record),
        updatedAt: sql`clock_timestamp()`,
      })
      .onConflictDoUpdate({
        target: [
          entCollectionRecord.etablissementId,
          entCollectionRecord.collection,
          entCollectionRecord.recordId,
        ],
        set: {
          status: statusFromRecord(record),
          updatedAt: sql`greatest(clock_timestamp(), ${entCollectionRecord.updatedAt} + interval '1 microsecond')`,
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
