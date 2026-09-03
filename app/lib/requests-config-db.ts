import "server-only";

import { eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { requestOrg, requestOrgAttr, requestRouting, requestRoutingAttr } from "@/db/schema";
import { flattenToAttrs, inflateFromAttrs } from "@/app/lib/ent-attr-codec";
import { isEntCoreDbEnabled, resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";

const ROUTING_KEY = "settings/requests-routing.json";
const ORG_KEY = "settings/requests-org.json";

export function isRequestsConfigDbEnabled(): boolean {
  return isEntCoreDbEnabled();
}

export async function requestsConfigDbReady(): Promise<string | null> {
  if (!isRequestsConfigDbEnabled()) return null;
  return resolveCurrentEtablissementId();
}

async function readSingletonFromAttrs(
  etablissementId: string,
  attrTable: typeof requestRoutingAttr | typeof requestOrgAttr,
): Promise<unknown | null> {
  try {
    const db = getDb();
    const rows = await db
      .select()
      .from(attrTable)
      .where(eq(attrTable.etablissementId, etablissementId));
    if (rows.length === 0) return null;
    return inflateFromAttrs(rows.map((r) => ({ path: r.path, value: r.value })));
  } catch (e) {
    console.error("[requests-config-db] readSingletonFromAttrs", e);
    return null;
  }
}

async function writeSingletonToAttrs(
  etablissementId: string,
  parentTable: typeof requestRouting | typeof requestOrg,
  attrTable: typeof requestRoutingAttr | typeof requestOrgAttr,
  payload: unknown,
): Promise<void> {
  const db = getDb();
  await db
    .insert(parentTable)
    .values({ etablissementId, updatedAt: new Date() })
    .onConflictDoUpdate({
      target: parentTable.etablissementId,
      set: { updatedAt: new Date() },
    });
  await db.delete(attrTable).where(eq(attrTable.etablissementId, etablissementId));
  const attrs = flattenToAttrs(payload);
  if (attrs.length === 0) return;
  const chunk = 100;
  for (let i = 0; i < attrs.length; i += chunk) {
    await db.insert(attrTable).values(
      attrs.slice(i, i + chunk).map((a) => ({
        etablissementId,
        path: a.path,
        value: a.value,
      })),
    );
  }
}

export async function getRequestsRoutingEnvelopeFromDb(
  etablissementId: string,
): Promise<{ version: number; updatedAt: string; data: unknown } | null> {
  const data = await readSingletonFromAttrs(etablissementId, requestRoutingAttr);
  if (!data || typeof data !== "object") return null;
  const o = data as Record<string, unknown>;
  if (o.data !== undefined) {
    return {
      version: typeof o.version === "number" ? o.version : 1,
      updatedAt: typeof o.updatedAt === "string" ? o.updatedAt : new Date().toISOString(),
      data: o.data,
    };
  }
  return { version: 1, updatedAt: new Date().toISOString(), data };
}

export async function saveRequestsRoutingEnvelopeToDb(
  etablissementId: string,
  envelope: { version?: number; updatedAt?: string; data: unknown },
): Promise<void> {
  await writeSingletonToAttrs(etablissementId, requestRouting, requestRoutingAttr, {
    version: envelope.version ?? 1,
    updatedAt: envelope.updatedAt ?? new Date().toISOString(),
    data: envelope.data,
  });
}

export async function getRequestsOrgEnvelopeFromDb(
  etablissementId: string,
): Promise<{ version: number; updatedAt: string; data: unknown } | null> {
  const data = await readSingletonFromAttrs(etablissementId, requestOrgAttr);
  if (!data || typeof data !== "object") return null;
  const o = data as Record<string, unknown>;
  if (o.data !== undefined) {
    return {
      version: typeof o.version === "number" ? o.version : 1,
      updatedAt: typeof o.updatedAt === "string" ? o.updatedAt : new Date().toISOString(),
      data: o.data,
    };
  }
  return { version: 1, updatedAt: new Date().toISOString(), data };
}

export async function saveRequestsOrgEnvelopeToDb(
  etablissementId: string,
  envelope: { version?: number; updatedAt?: string; data: unknown },
): Promise<void> {
  await writeSingletonToAttrs(etablissementId, requestOrg, requestOrgAttr, {
    version: envelope.version ?? 1,
    updatedAt: envelope.updatedAt ?? new Date().toISOString(),
    data: envelope.data,
  });
}

export function isRequestsConfigTypedPath(relativePath: string): boolean {
  return relativePath === ROUTING_KEY || relativePath === ORG_KEY;
}

export async function migrateRequestsConfigEnvelopeToDb(
  etablissementId: string,
  relativePath: string,
  envelope: unknown,
): Promise<boolean> {
  if (!envelope || typeof envelope !== "object") return false;
  const o = envelope as Record<string, unknown>;
  const wrapped =
    o.data !== undefined
      ? {
          version: typeof o.version === "number" ? o.version : 1,
          updatedAt: typeof o.updatedAt === "string" ? o.updatedAt : new Date().toISOString(),
          data: o.data,
        }
      : { version: 1, updatedAt: new Date().toISOString(), data: envelope };

  if (relativePath === ROUTING_KEY) {
    await saveRequestsRoutingEnvelopeToDb(etablissementId, wrapped);
    return true;
  }
  if (relativePath === ORG_KEY) {
    await saveRequestsOrgEnvelopeToDb(etablissementId, wrapped);
    return true;
  }
  return false;
}
