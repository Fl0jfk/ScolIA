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
  // Absences : plus de pont via chemins *.json — utiliser absence-db / absences-storage.
  if (/^absences\//i.test(key)) {
    throw new Error(
      "[ent] Absences Postgres uniquement — ne pas lire absences/*.json via getJson",
    );
  }
  if (/^requests\//i.test(key)) {
    const { requestsDbReady, getRequestFromDb, ensureRequestsMigratedFromCollection } = await import(
      "@/app/lib/request-db"
    );
    const etabId = await requestsDbReady();
    if (!etabId) return undefined;
    if (key === "requests/index.json") {
      return { data: (await ensureRequestsMigratedFromCollection(etabId)) as T, key };
    }
    const match = /^requests\/([^/]+)\.json$/i.exec(key);
    if (match && match[1] !== "index") {
      const row = await getRequestFromDb(etabId, match[1]);
      return row ? { data: row as T, key } : null;
    }
    throw new Error("[ent] Demandes Postgres uniquement — chemin requests/*.json invalide");
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
  if (relativePath === "settings/requests-routing.json") {
    const { requestsConfigDbReady, getRequestsRoutingEnvelopeFromDb } = await import(
      "@/app/lib/requests-config-db"
    );
    const etabId = await requestsConfigDbReady();
    if (!etabId) return undefined;
    const env = await getRequestsRoutingEnvelopeFromDb(etabId);
    return env ? { data: env as T, key } : null;
  }
  if (relativePath === "settings/requests-org.json") {
    const { requestsConfigDbReady, getRequestsOrgEnvelopeFromDb } = await import(
      "@/app/lib/requests-config-db"
    );
    const etabId = await requestsConfigDbReady();
    if (!etabId) return undefined;
    const env = await getRequestsOrgEnvelopeFromDb(etabId);
    return env ? { data: env as T, key } : null;
  }
  if (relativePath === "personnel-ogec/leave-requests.json") {
    const { getPersonnelLeaveRequests } = await import("@/app/lib/personnel-leave-storage");
    try {
      return { data: (await getPersonnelLeaveRequests()) as T, key };
    } catch {
      return undefined;
    }
  }
  if (relativePath === "personnel-ogec/shared-documents.json") {
    const { getSharedPersonnelDocuments } = await import("@/app/lib/personnel-storage");
    try {
      return { data: (await getSharedPersonnelDocuments()) as T, key };
    } catch {
      return undefined;
    }
  }
  if (relativePath === "reservation-rooms/rooms.json") {
    const { listReservationRooms } = await import("@/app/lib/reservation-rooms-storage");
    return { data: { rooms: await listReservationRooms() } as T, key };
  }
  if (relativePath === "reservation-rooms/reservations.json") {
    const { listReservationBookings } = await import("@/app/lib/reservation-rooms-storage");
    return { data: (await listReservationBookings()) as T, key };
  }
  return undefined;
}

async function tryTypedPut(relativePath: string, data: unknown): Promise<string | null> {
  const key = s3Key(relativePath);
  if (/^absences\//i.test(key)) {
    throw new Error(
      "[ent] Absences Postgres uniquement — ne pas écrire absences/*.json via putJson",
    );
  }
  if (/^requests\//i.test(key)) {
    const { requestsDbReady, upsertRequestInDb } = await import("@/app/lib/request-db");
    const etabId = await requestsDbReady();
    if (!etabId) throw new Error("[ent] Postgres requis");
    if (key === "requests/index.json" && Array.isArray(data)) {
      // Comme travels : jamais de replace wipe — upsert unitaire uniquement.
      for (const row of data) {
        const rec = row as { id?: string };
        if (rec?.id) {
          await upsertRequestInDb(
            etabId,
            row as import("@/app/lib/requests").RequestRecord,
          );
        }
      }
      return key;
    }
    const match = /^requests\/([^/]+)\.json$/i.exec(key);
    if (match && match[1] !== "index") {
      await upsertRequestInDb(etabId, data as import("@/app/lib/requests").RequestRecord);
      return key;
    }
    throw new Error("[ent] Demandes Postgres uniquement — chemin requests/*.json invalide");
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
    // Index legacy : la liste lit `travel` directement. Ne pas remplacer toute la table
    // (sinon suppression de dossiers non finalisés lors d'une simple mise à jour).
    return key;
  }
  if (relativePath === "settings/requests-routing.json") {
    const { requestsConfigDbReady, saveRequestsRoutingEnvelopeToDb } = await import(
      "@/app/lib/requests-config-db"
    );
    const etabId = await requestsConfigDbReady();
    if (!etabId) return null;
    await saveRequestsRoutingEnvelopeToDb(etabId, data as { data: unknown });
    return key;
  }
  if (relativePath === "settings/requests-org.json") {
    const { requestsConfigDbReady, saveRequestsOrgEnvelopeToDb } = await import(
      "@/app/lib/requests-config-db"
    );
    const etabId = await requestsConfigDbReady();
    if (!etabId) return null;
    await saveRequestsOrgEnvelopeToDb(etabId, data as { data: unknown });
    return key;
  }
  if (relativePath === "personnel-ogec/leave-requests.json" && Array.isArray(data)) {
    const { upsertPersonnelLeaveRequest } = await import("@/app/lib/personnel-leave-storage");
    for (const row of data) {
      const leave = row as { id?: string };
      if (leave?.id) {
        await upsertPersonnelLeaveRequest(
          row as import("@/app/lib/personnel-types").PersonnelLeaveRequest,
        );
      }
    }
    return key;
  }
  if (relativePath === "personnel-ogec/shared-documents.json" && Array.isArray(data)) {
    const { saveSharedPersonnelDocuments } = await import("@/app/lib/personnel-storage");
    await saveSharedPersonnelDocuments(
      data as import("@/app/lib/personnel-types").SharedPersonnelDocument[],
    );
    return key;
  }
  if (relativePath === "reservation-rooms/rooms.json") {
    const { saveReservationRooms } = await import("@/app/lib/reservation-rooms-storage");
    const rooms = Array.isArray(data)
      ? data
      : Array.isArray((data as { rooms?: unknown[] })?.rooms)
        ? (data as { rooms: unknown[] }).rooms
        : [];
    await saveReservationRooms(rooms as import("@/app/lib/reservation-rooms-db").ReservationRoomRow[]);
    return key;
  }
  if (relativePath === "reservation-rooms/reservations.json" && Array.isArray(data)) {
    const { saveReservationBookings } = await import("@/app/lib/reservation-rooms-storage");
    await saveReservationBookings(
      data as import("@/app/lib/prof-room-reservations-normalize").RoomReservationRow[],
    );
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
