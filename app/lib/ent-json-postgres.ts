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
    try {
      const { requestsConfigDbReady, getRequestsRoutingEnvelopeFromDb } = await import(
        "@/app/lib/requests-config-db"
      );
      const etabId = await requestsConfigDbReady();
      if (!etabId) return undefined;
      const env = await getRequestsRoutingEnvelopeFromDb(etabId);
      // undefined (pas null) → repli ent_collection si tables vides / non migrées
      return env ? { data: env as T, key } : undefined;
    } catch (e) {
      console.error("[ent-json-postgres] requests-routing typed get", e);
      return undefined;
    }
  }
  if (relativePath === "settings/requests-org.json") {
    try {
      const { requestsConfigDbReady, getRequestsOrgEnvelopeFromDb } = await import(
        "@/app/lib/requests-config-db"
      );
      const etabId = await requestsConfigDbReady();
      if (!etabId) return undefined;
      const env = await getRequestsOrgEnvelopeFromDb(etabId);
      return env ? { data: env as T, key } : undefined;
    } catch (e) {
      console.error("[ent-json-postgres] requests-org typed get", e);
      return undefined;
    }
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

  // --- Stages : tables typées stage_* (plus de ent_collection pour ces chemins) ---
  if (/^stages\//i.test(key)) {
    const stagesHit = await tryStagesTypedGet<T>(key);
    if (stagesHit !== undefined) return stagesHit;
  }

  return undefined;
}

async function tryStagesTypedGet<T>(
  key: string,
): Promise<{ data: T; key: string } | null | undefined> {
  const {
    stagesDbReady,
    listConventionIndexFromDb,
    getConventionFromDbOrMigrate,
    listOfferIndexFromDb,
    getOfferFromDbOrMigrate,
    listApplicationsForOfferFromDb,
    getSignTokenTyped,
    getStudentTokenTyped,
    getOfferCandidatureTokenTyped,
    getSignCodeLookupTyped,
    getStageTokenFromDb,
  } = await import("@/app/lib/stage-db");
  const etabId = await stagesDbReady();
  if (!etabId) return undefined;

  if (key === "stages/conventions-index.json") {
    return { data: (await listConventionIndexFromDb(etabId)) as T, key };
  }
  if (key === "stages/offers-index.json") {
    return { data: (await listOfferIndexFromDb(etabId)) as T, key };
  }

  const conventionMatch = /^stages\/conventions\/([^/]+)\.json$/i.exec(key);
  if (conventionMatch) {
    const row = await getConventionFromDbOrMigrate(etabId, conventionMatch[1]);
    return row ? { data: row as T, key } : null;
  }

  const offerMatch = /^stages\/offers\/([^/]+)\.json$/i.exec(key);
  if (offerMatch) {
    const row = await getOfferFromDbOrMigrate(etabId, offerMatch[1]);
    return row ? { data: row as T, key } : null;
  }

  const appsMatch = /^stages\/offer-applications\/([^/]+)\.json$/i.exec(key);
  if (appsMatch) {
    return {
      data: (await listApplicationsForOfferFromDb(etabId, appsMatch[1])) as T,
      key,
    };
  }

  const signTok = /^stages\/sign-tokens\/([^/]+)\.json$/i.exec(key);
  if (signTok) {
    const row = await getSignTokenTyped(etabId, signTok[1]);
    return row ? { data: row as T, key } : null;
  }

  const studentTok = /^stages\/student-tokens\/([^/]+)\.json$/i.exec(key);
  if (studentTok) {
    const row = await getStudentTokenTyped(etabId, studentTok[1]);
    return row ? { data: row as T, key } : null;
  }

  const candTok = /^stages\/offer-candidature-tokens\/([^/]+)\.json$/i.exec(key);
  if (candTok) {
    const row = await getOfferCandidatureTokenTyped(etabId, candTok[1]);
    return row ? { data: row as T, key } : null;
  }

  const signCode = /^stages\/sign-code-lookup\/([^/]+)\.json$/i.exec(key);
  if (signCode) {
    const row = await getSignCodeLookupTyped(etabId, signCode[1]);
    return row ? { data: row as T, key } : null;
  }

  const periods = /^stages\/periods\/([^/]+)\.json$/i.exec(key);
  if (periods) {
    const row = await getStageTokenFromDb(etabId, "periods", periods[1]);
    return row ? { data: row as T, key } : null;
  }
  const referents = /^stages\/referents\/([^/]+)\.json$/i.exec(key);
  if (referents) {
    const row = await getStageTokenFromDb(etabId, "referents", referents[1]);
    return row ? { data: row as T, key } : null;
  }
  const constraints = /^stages\/constraints\/([^/]+)\.json$/i.exec(key);
  if (constraints) {
    const row = await getStageTokenFromDb(etabId, "constraints", constraints[1]);
    return row ? { data: row as T, key } : null;
  }
  const watchers = /^stages\/watchers\/([^/]+)\.json$/i.exec(key);
  if (watchers) {
    const row = await getStageTokenFromDb(etabId, "watchers", watchers[1]);
    return row ? { data: row as T, key } : null;
  }

  if (key === "stages/auto-purge-state.json") {
    const row = await getStageTokenFromDb(etabId, "auto_purge", "state");
    return row ? { data: row as T, key } : null;
  }

  const idOtp = /^stages\/identity-otp\/([^/]+)\.json$/i.exec(key);
  if (idOtp) {
    const row = await getStageTokenFromDb(etabId, "identity_otp", idOtp[1]);
    return row ? { data: row as T, key } : null;
  }
  const idChoice = /^stages\/identity-recipient-choice\/([^/]+)\.json$/i.exec(key);
  if (idChoice) {
    const row = await getStageTokenFromDb(etabId, "identity_recipient_choice", idChoice[1]);
    return row ? { data: row as T, key } : null;
  }
  const idProof = /^stages\/identity-proof\/([^/]+)\.json$/i.exec(key);
  if (idProof) {
    const row = await getStageTokenFromDb(etabId, "identity_proof", idProof[1]);
    return row ? { data: row as T, key } : null;
  }

  // Binaires / chemins inconnus : laisser le repli collection (ne devrait pas arriver pour JSON métier).
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
    try {
      const { requestsConfigDbReady, saveRequestsRoutingEnvelopeToDb } = await import(
        "@/app/lib/requests-config-db"
      );
      const etabId = await requestsConfigDbReady();
      if (!etabId) return null;
      await saveRequestsRoutingEnvelopeToDb(etabId, data as { data: unknown });
      return key;
    } catch (e) {
      console.error("[ent-json-postgres] requests-routing typed put", e);
      return null;
    }
  }
  if (relativePath === "settings/requests-org.json") {
    try {
      const { requestsConfigDbReady, saveRequestsOrgEnvelopeToDb } = await import(
        "@/app/lib/requests-config-db"
      );
      const etabId = await requestsConfigDbReady();
      if (!etabId) return null;
      await saveRequestsOrgEnvelopeToDb(etabId, data as { data: unknown });
      return key;
    } catch (e) {
      console.error("[ent-json-postgres] requests-org typed put", e);
      return null;
    }
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

  if (/^stages\//i.test(key)) {
    const stagesPut = await tryStagesTypedPut(key, data);
    if (stagesPut) return stagesPut;
  }

  return null;
}

async function tryStagesTypedPut(key: string, data: unknown): Promise<string | null> {
  const {
    stagesDbReady,
    upsertConventionInDb,
    upsertOfferInDb,
    replaceOfferApplicationsInDb,
    saveSignTokenTyped,
    saveStudentTokenTyped,
    saveOfferCandidatureTokenTyped,
    saveSignCodeLookupTyped,
    upsertStageTokenInDb,
  } = await import("@/app/lib/stage-db");
  const etabId = await stagesDbReady();
  if (!etabId) throw new Error("[ent] Postgres requis pour stages");

  // Indexes dérivés des tables typées — jamais de replace wipe.
  if (key === "stages/conventions-index.json" || key === "stages/offers-index.json") {
    return key;
  }

  const conventionMatch = /^stages\/conventions\/([^/]+)\.json$/i.exec(key);
  if (conventionMatch) {
    const c = data as import("@/app/lib/stage-types").StageConvention;
    if (!c?.id) throw new Error("[ent] Convention stages sans id");
    await upsertConventionInDb(etabId, c);
    return key;
  }

  const offerMatch = /^stages\/offers\/([^/]+)\.json$/i.exec(key);
  if (offerMatch) {
    const o = data as import("@/app/lib/stage-types").StageOffer;
    if (!o?.id) throw new Error("[ent] Offre stages sans id");
    await upsertOfferInDb(etabId, o);
    return key;
  }

  const appsMatch = /^stages\/offer-applications\/([^/]+)\.json$/i.exec(key);
  if (appsMatch) {
    const list = Array.isArray(data)
      ? (data as import("@/app/lib/stage-types").StageOfferApplication[])
      : [];
    await replaceOfferApplicationsInDb(etabId, appsMatch[1], list);
    return key;
  }

  const signTok = /^stages\/sign-tokens\/([^/]+)\.json$/i.exec(key);
  if (signTok) {
    await saveSignTokenTyped(
      etabId,
      signTok[1],
      data as import("@/app/lib/stage-types").StageSignTokenRef,
    );
    return key;
  }
  const studentTok = /^stages\/student-tokens\/([^/]+)\.json$/i.exec(key);
  if (studentTok) {
    await saveStudentTokenTyped(
      etabId,
      studentTok[1],
      data as import("@/app/lib/stage-types").StageStudentTokenRef,
    );
    return key;
  }
  const candTok = /^stages\/offer-candidature-tokens\/([^/]+)\.json$/i.exec(key);
  if (candTok) {
    await saveOfferCandidatureTokenTyped(
      etabId,
      candTok[1],
      data as import("@/app/lib/stage-types").StageOfferCandidatureTokenRef,
    );
    return key;
  }
  const signCode = /^stages\/sign-code-lookup\/([^/]+)\.json$/i.exec(key);
  if (signCode) {
    await saveSignCodeLookupTyped(
      etabId,
      signCode[1],
      data as import("@/app/lib/stage-types").StageSignCodeLookupRef,
    );
    return key;
  }

  const periods = /^stages\/periods\/([^/]+)\.json$/i.exec(key);
  if (periods && data && typeof data === "object" && !Array.isArray(data)) {
    await upsertStageTokenInDb(etabId, "periods", periods[1], data as Record<string, unknown>);
    return key;
  }
  const referents = /^stages\/referents\/([^/]+)\.json$/i.exec(key);
  if (referents && data && typeof data === "object" && !Array.isArray(data)) {
    await upsertStageTokenInDb(etabId, "referents", referents[1], data as Record<string, unknown>);
    return key;
  }
  const constraints = /^stages\/constraints\/([^/]+)\.json$/i.exec(key);
  if (constraints && data && typeof data === "object" && !Array.isArray(data)) {
    await upsertStageTokenInDb(
      etabId,
      "constraints",
      constraints[1],
      data as Record<string, unknown>,
    );
    return key;
  }
  const watchers = /^stages\/watchers\/([^/]+)\.json$/i.exec(key);
  if (watchers && data && typeof data === "object" && !Array.isArray(data)) {
    await upsertStageTokenInDb(etabId, "watchers", watchers[1], data as Record<string, unknown>);
    return key;
  }

  if (key === "stages/auto-purge-state.json" && data && typeof data === "object") {
    await upsertStageTokenInDb(
      etabId,
      "auto_purge",
      "state",
      data as Record<string, unknown>,
    );
    return key;
  }

  const idOtp = /^stages\/identity-otp\/([^/]+)\.json$/i.exec(key);
  if (idOtp && data && typeof data === "object" && !Array.isArray(data)) {
    await upsertStageTokenInDb(etabId, "identity_otp", idOtp[1], data as Record<string, unknown>);
    return key;
  }
  const idChoice = /^stages\/identity-recipient-choice\/([^/]+)\.json$/i.exec(key);
  if (idChoice && data && typeof data === "object" && !Array.isArray(data)) {
    await upsertStageTokenInDb(
      etabId,
      "identity_recipient_choice",
      idChoice[1],
      data as Record<string, unknown>,
    );
    return key;
  }
  const idProof = /^stages\/identity-proof\/([^/]+)\.json$/i.exec(key);
  if (idProof && data && typeof data === "object" && !Array.isArray(data)) {
    await upsertStageTokenInDb(
      etabId,
      "identity_proof",
      idProof[1],
      data as Record<string, unknown>,
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
  const key = s3Key(relativePath);
  if (/^stages\//i.test(key)) {
    const deleted = await tryStagesTypedDelete(key);
    if (deleted) return;
  }

  const etabId = await collectionDbReady();
  if (!etabId) return;
  const { collection, recordId, singleton } = jsonPathToCollection(relativePath);
  const { deleteCollectionRecord } = await import("@/app/lib/ent-collection-db");
  await deleteCollectionRecord(etabId, collection, singleton ? "_" : recordId);
}

async function tryStagesTypedDelete(key: string): Promise<boolean> {
  const { stagesDbReady, deleteStageTokenFromDb } = await import("@/app/lib/stage-db");
  const etabId = await stagesDbReady();
  if (!etabId) return false;

  const signTok = /^stages\/sign-tokens\/([^/]+)\.json$/i.exec(key);
  if (signTok) {
    await deleteStageTokenFromDb(etabId, "sign", signTok[1]);
    return true;
  }
  const studentTok = /^stages\/student-tokens\/([^/]+)\.json$/i.exec(key);
  if (studentTok) {
    await deleteStageTokenFromDb(etabId, "student", studentTok[1]);
    return true;
  }
  const candTok = /^stages\/offer-candidature-tokens\/([^/]+)\.json$/i.exec(key);
  if (candTok) {
    await deleteStageTokenFromDb(etabId, "offer_candidature", candTok[1]);
    return true;
  }
  const signCode = /^stages\/sign-code-lookup\/([^/]+)\.json$/i.exec(key);
  if (signCode) {
    await deleteStageTokenFromDb(etabId, "sign_code", signCode[1]);
    return true;
  }
  const idOtp = /^stages\/identity-otp\/([^/]+)\.json$/i.exec(key);
  if (idOtp) {
    await deleteStageTokenFromDb(etabId, "identity_otp", idOtp[1]);
    return true;
  }
  const idChoice = /^stages\/identity-recipient-choice\/([^/]+)\.json$/i.exec(key);
  if (idChoice) {
    await deleteStageTokenFromDb(etabId, "identity_recipient_choice", idChoice[1]);
    return true;
  }
  const idProof = /^stages\/identity-proof\/([^/]+)\.json$/i.exec(key);
  if (idProof) {
    await deleteStageTokenFromDb(etabId, "identity_proof", idProof[1]);
    return true;
  }
  return false;
}
