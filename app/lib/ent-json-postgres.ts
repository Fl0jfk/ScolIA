import "server-only";

import {
  collectionDbReady,
  getCollectionRecord,
  getCollectionSingleton,
  putCollectionSingleton,
  upsertCollectionRecord,
} from "@/app/lib/ent-collection-db";
import { s3Key } from "@/app/lib/s3-path";

/** Mappe un chemin S3 JSON historique → collection + recordId relationnels. */
export function jsonPathToCollection(relativePath: string): {
  collection: string;
  recordId: string;
  singleton: boolean;
} {
  const key = s3Key(relativePath).replace(/\.json$/i, "");
  const parts = key.split("/").filter(Boolean);
  if (parts.length === 0) return { collection: "root", recordId: "_", singleton: true };
  const last = parts[parts.length - 1] ?? "";
  const singletonNames = new Set([
    "index",
    "current",
    "workspace",
    "config",
    "domains",
    "bookings",
    "sessions",
    "signups",
    "rooms",
    "profiles",
    "matches",
    "channels",
    "messages",
    "shared-documents",
    "leave-requests",
  ]);
  if (parts.length === 1 || singletonNames.has(last)) {
    return { collection: parts.join("__"), recordId: "_", singleton: true };
  }
  return {
    collection: parts.slice(0, -1).join("__"),
    recordId: last,
    singleton: false,
  };
}

async function tryTypedGet<T>(
  relativePath: string,
): Promise<{ data: T; key: string } | null | undefined> {
  const key = s3Key(relativePath);
  const absenceMatch = /^absences\/([^/]+)\.json$/i.exec(key);
  if (absenceMatch && absenceMatch[1] !== "index") {
    const { absencesDbReady, getAbsenceFromDb } = await import("@/app/lib/absence-db");
    const etabId = await absencesDbReady();
    if (!etabId) return undefined;
    const row = await getAbsenceFromDb(etabId, absenceMatch[1]);
    return row ? { data: row as T, key } : null;
  }
  if (key === "absences/index.json") {
    const { absencesDbReady, listAbsencesFromDb } = await import("@/app/lib/absence-db");
    const etabId = await absencesDbReady();
    if (!etabId) return undefined;
    return { data: (await listAbsencesFromDb(etabId)) as T, key };
  }
  const travelMatch = /^travels\/([^/]+)\.json$/i.exec(key);
  if (travelMatch && travelMatch[1] !== "index") {
    const { travelsDbReady, getTravelFromDb } = await import("@/app/lib/travel-db");
    const etabId = await travelsDbReady();
    if (!etabId) return undefined;
    const row = await getTravelFromDb(etabId, travelMatch[1]);
    return row ? { data: row as T, key } : null;
  }
  if (key === "travels/index.json") {
    const { travelsDbReady, listTravelsFromDb } = await import("@/app/lib/travel-db");
    const etabId = await travelsDbReady();
    if (!etabId) return undefined;
    return { data: (await listTravelsFromDb(etabId)) as T, key };
  }
  return undefined;
}

async function tryTypedPut(relativePath: string, data: unknown): Promise<string | null> {
  const key = s3Key(relativePath);
  const absenceMatch = /^absences\/([^/]+)\.json$/i.exec(key);
  if (absenceMatch && absenceMatch[1] !== "index") {
    const { absencesDbReady, upsertAbsenceInDb } = await import("@/app/lib/absence-db");
    const etabId = await absencesDbReady();
    if (!etabId) throw new Error("[ent] Postgres requis");
    await upsertAbsenceInDb(etabId, data as import("@/app/lib/absences-types").AbsenceRecord);
    return key;
  }
  if (key === "absences/index.json" && Array.isArray(data)) {
    const { absencesDbReady, replaceAbsencesInDb } = await import("@/app/lib/absence-db");
    const etabId = await absencesDbReady();
    if (!etabId) throw new Error("[ent] Postgres requis");
    await replaceAbsencesInDb(etabId, data as import("@/app/lib/absences-types").AbsenceRecord[]);
    return key;
  }
  const travelMatch = /^travels\/([^/]+)\.json$/i.exec(key);
  if (travelMatch && travelMatch[1] !== "index") {
    const { travelsDbReady, upsertTravelInDb } = await import("@/app/lib/travel-db");
    const etabId = await travelsDbReady();
    if (!etabId) throw new Error("[ent] Postgres requis");
    await upsertTravelInDb(etabId, data as import("@/app/lib/travels-types").TravelsTrip);
    return key;
  }
  if (key === "travels/index.json" && Array.isArray(data)) {
    const { travelsDbReady, replaceTravelsInDb } = await import("@/app/lib/travel-db");
    const etabId = await travelsDbReady();
    if (!etabId) throw new Error("[ent] Postgres requis");
    await replaceTravelsInDb(etabId, data as import("@/app/lib/travels-types").TravelsTrip[]);
    return key;
  }
  return null;
}

export async function getJsonFromPostgres<T>(
  relativePath: string,
): Promise<{ data: T; key: string } | null> {
  const typed = await tryTypedGet<T>(relativePath);
  if (typed !== undefined) return typed;

  const etabId = await collectionDbReady();
  if (!etabId) return null;
  const key = s3Key(relativePath);
  const { collection, recordId, singleton } = jsonPathToCollection(relativePath);
  if (singleton) {
    const data = await getCollectionSingleton<T & (Record<string, unknown> | unknown[])>(
      etabId,
      collection,
    );
    if (data === null) return null;
    return { data: data as T, key };
  }
  const row = await getCollectionRecord<Record<string, unknown>>(etabId, collection, recordId);
  if (!row) return null;
  if ("__root" in row && Object.keys(row).every((k) => k === "__root" || k === "id")) {
    return { data: row.__root as T, key };
  }
  return { data: row as T, key };
}

export async function putJsonToPostgres(relativePath: string, data: unknown): Promise<string> {
  const typed = await tryTypedPut(relativePath, data);
  if (typed) return typed;

  const etabId = await collectionDbReady();
  if (!etabId) {
    throw new Error(
      `[ent] putJson impossible sans Postgres (ENT_CORE_DB / DATABASE_URL) pour ${relativePath}`,
    );
  }
  const key = s3Key(relativePath);
  const { collection, recordId, singleton } = jsonPathToCollection(relativePath);
  if (singleton) {
    await putCollectionSingleton(etabId, collection, data);
  } else if (!data || typeof data !== "object" || Array.isArray(data)) {
    await upsertCollectionRecord(etabId, collection, recordId, { __root: data as unknown });
  } else {
    const obj = { ...(data as Record<string, unknown>) };
    if (obj.id == null) obj.id = recordId;
    await upsertCollectionRecord(etabId, collection, recordId, obj);
  }
  return key;
}

/** Liste les enregistrements d’un « dossier » JSON (ex. documents/shares → collection documents__shares). */
export async function listJsonRecordsInDir<T extends Record<string, unknown>>(
  relativeDir: string,
): Promise<T[]> {
  const etabId = await collectionDbReady();
  if (!etabId) return [];
  const dir = relativeDir.replace(/^\/+/, "").replace(/\/+$/, "");
  if (!dir) return [];
  const { collection, singleton } = jsonPathToCollection(`${dir}/__probe__.json`);
  if (singleton) return [];
  const { listCollectionRecords } = await import("@/app/lib/ent-collection-db");
  return listCollectionRecords<T>(etabId, collection);
}

export async function deleteJsonFromPostgres(relativePath: string): Promise<void> {
  const etabId = await collectionDbReady();
  if (!etabId) return;
  const { collection, recordId, singleton } = jsonPathToCollection(relativePath);
  const { deleteCollectionRecord } = await import("@/app/lib/ent-collection-db");
  await deleteCollectionRecord(etabId, collection, singleton ? "_" : recordId);
}
