import "server-only";

import { and, eq, inArray, sql } from "drizzle-orm";
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

/**
 * Insère l'enregistrement SEULEMENT s'il n'existe pas (clé unique établissement + collection + id).
 * Atomique côté Postgres (ON CONFLICT DO NOTHING) : un seul appelant concurrent obtient `true`.
 */
export async function insertCollectionRecordIfAbsent(
  etablissementId: string,
  collection: CollectionName,
  recordId: string,
  record: Record<string, unknown>,
): Promise<boolean> {
  const db = getDb();
  const attrs = flattenCollectionRecord(record);
  return db.transaction(async (tx) => {
    const inserted = await tx
      .insert(entCollectionRecord)
      .values({
        etablissementId,
        collection,
        recordId,
        status: statusFromRecord(record),
        updatedAt: sql`clock_timestamp()`,
      })
      .onConflictDoNothing()
      .returning({ recordId: entCollectionRecord.recordId });
    if (inserted.length === 0) return false;
    for (let i = 0; i < attrs.length; i += 100) {
      await tx.insert(entCollectionAttr).values(
        attrs.slice(i, i + 100).map((a) => ({
          etablissementId,
          collection,
          recordId,
          path: a.path,
          value: a.value,
        })),
      );
    }
    return true;
  });
}

/* ─── Ajouts voyages v2 : mutation sérialisée et verrou en table (aucune fonction existante modifiée) ─── */

/** Clé du verrou par document. Postgres refuse l'octet nul dans un texte : ne jamais l'utiliser ici. */
export function cleVerrouDocument(etablissementId: string, collection: string, recordId: string): string {
  return `${etablissementId}|${collection}|${recordId}`;
}

type CollectionTx =Parameters<Parameters<ReturnType<typeof getDb>["transaction"]>[0]>[0];

/**
 * Relit, modifie et réécrit UN enregistrement dans une transaction protégée par un verrou advisory
 * transactionnel (`pg_advisory_xact_lock(hashtext(clé))`) : deux mutations du même enregistrement
 * s'exécutent l'une après l'autre, chacune sur la dernière version. Le verrou est libéré à la fin de la
 * transaction (compatible PgBouncer en mode transaction : rien n'est lié à la session).
 *
 * `mutate` doit être SYNCHRONE et ne faire aucune entrée-sortie (pas d'e-mail, pas de S3) : la transaction
 * reste ouverte pendant son exécution. Il reçoit une copie de l'enregistrement et retourne `{ save, value }` :
 * `save: false` = rien n'est écrit. Retourne `null` si l'enregistrement n'existe pas.
 */
export async function mutateCollectionRecordLocked<T extends Record<string, unknown>, R>(
  etablissementId: string,
  collection: CollectionName,
  recordId: string,
  mutate: (current: T) => { save: boolean; value: R },
): Promise<{ value: R; saved: boolean } | null> {
  const db = getDb();
  return db.transaction(async (tx: CollectionTx) => {
    const lockKey = cleVerrouDocument(etablissementId, collection, recordId);
    await tx.execute(sql`select pg_advisory_xact_lock(hashtext(${lockKey}))`);
    const [head] = await tx
      .select({ recordId: entCollectionRecord.recordId })
      .from(entCollectionRecord)
      .where(
        and(
          eq(entCollectionRecord.etablissementId, etablissementId),
          eq(entCollectionRecord.collection, collection),
          eq(entCollectionRecord.recordId, recordId),
        ),
      )
      .limit(1);
    if (!head) return null;
    const rows = await tx
      .select({ path: entCollectionAttr.path, value: entCollectionAttr.value })
      .from(entCollectionAttr)
      .where(
        and(
          eq(entCollectionAttr.etablissementId, etablissementId),
          eq(entCollectionAttr.collection, collection),
          eq(entCollectionAttr.recordId, recordId),
        ),
      );
    const current = inflateFromAttrs(rows) as Record<string, unknown>;
    if (!current.id) current.id = recordId;
    const out = mutate(current as T);
    if (!out.save) return { value: out.value, saved: false };
    const attrs = flattenCollectionRecord(current);
    await tx
      .update(entCollectionRecord)
      .set({
        status: statusFromRecord(current),
        updatedAt: sql`greatest(clock_timestamp(), ${entCollectionRecord.updatedAt} + interval '1 microsecond')`,
      })
      .where(
        and(
          eq(entCollectionRecord.etablissementId, etablissementId),
          eq(entCollectionRecord.collection, collection),
          eq(entCollectionRecord.recordId, recordId),
        ),
      );
    await tx
      .delete(entCollectionAttr)
      .where(
        and(
          eq(entCollectionAttr.etablissementId, etablissementId),
          eq(entCollectionAttr.collection, collection),
          eq(entCollectionAttr.recordId, recordId),
        ),
      );
    for (let i = 0; i < attrs.length; i += 100) {
      await tx.insert(entCollectionAttr).values(
        attrs.slice(i, i + 100).map((a) => ({
          etablissementId,
          collection,
          recordId,
          path: a.path,
          value: a.value,
        })),
      );
    }
    return { value: out.value, saved: true };
  });
}

/**
 * Verrou « en table » (une ligne, sans attribut) : prise atomique par INSERT ... ON CONFLICT, reprise d'un
 * verrou périmé (plus vieux que `ttlSeconds`) par un UPDATE conditionnel — un seul appelant gagne.
 * Aucune connexion n'est gardée ouverte : fonctionne derrière PgBouncer (contrairement à un verrou de session).
 * Retourne true si le verrou est pris ; l'appelant DOIT appeler `releaseCollectionLock` (finally).
 */
export async function tryAcquireCollectionLock(
  etablissementId: string,
  collection: CollectionName,
  recordId: string,
  ttlSeconds: number,
): Promise<boolean> {
  const db = getDb();
  const rows = await db
    .insert(entCollectionRecord)
    .values({ etablissementId, collection, recordId, status: "locked", updatedAt: sql`clock_timestamp()` })
    .onConflictDoUpdate({
      target: [entCollectionRecord.etablissementId, entCollectionRecord.collection, entCollectionRecord.recordId],
      set: { status: "locked", updatedAt: sql`clock_timestamp()` },
      setWhere: sql`${entCollectionRecord.updatedAt} < clock_timestamp() - make_interval(secs => ${ttlSeconds})`,
    })
    .returning({ recordId: entCollectionRecord.recordId });
  return rows.length > 0;
}

export async function releaseCollectionLock(
  etablissementId: string,
  collection: CollectionName,
  recordId: string,
): Promise<void> {
  const db = getDb();
  await db
    .delete(entCollectionRecord)
    .where(
      and(
        eq(entCollectionRecord.etablissementId, etablissementId),
        eq(entCollectionRecord.collection, collection),
        eq(entCollectionRecord.recordId, recordId),
      ),
    );
}

/**
 * Enregistrements d'une collection dont l'attribut texte `path` vaut `value` (ex. les invitations d'un élève).
 * Une requête ciblée au lieu de charger toute la collection ; le filtre parcourt les attributs de la collection
 * côté base (pas d'index dédié : aucune migration ajoutée).
 */
export async function listCollectionRecordsByStringAttr<T extends Record<string, unknown>>(
  etablissementId: string,
  collection: CollectionName,
  path: string,
  value: string,
): Promise<T[]> {
  const db = getDb();
  const hits = await db
    .select({ recordId: entCollectionAttr.recordId })
    .from(entCollectionAttr)
    .where(
      and(
        eq(entCollectionAttr.etablissementId, etablissementId),
        eq(entCollectionAttr.collection, collection),
        eq(entCollectionAttr.path, path),
        eq(entCollectionAttr.value, `s:${value}`),
      ),
    );
  const ids = [...new Set(hits.map((h) => h.recordId))];
  if (ids.length === 0) return [];
  const attrs = await db
    .select()
    .from(entCollectionAttr)
    .where(
      and(
        eq(entCollectionAttr.etablissementId, etablissementId),
        eq(entCollectionAttr.collection, collection),
        inArray(entCollectionAttr.recordId, ids),
      ),
    );
  const byId = new Map<string, { path: string; value: string }[]>();
  for (const a of attrs) {
    const list = byId.get(a.recordId) ?? [];
    list.push({ path: a.path, value: a.value });
    byId.set(a.recordId, list);
  }
  return ids.map((id) => {
    const obj = inflateFromAttrs(byId.get(id) ?? []);
    if (!obj.id) obj.id = id;
    return obj as T;
  });
}
