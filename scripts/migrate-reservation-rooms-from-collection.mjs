/**
 * Migre salles/réservations depuis ent_collection_* → tables typées (upsert unitaire).
 */
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import postgres from "postgres";

function loadEnvFile(filePath) {
  if (!existsSync(filePath)) return;
  for (const line of readFileSync(filePath, "utf8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eqIdx = trimmed.indexOf("=");
    if (eqIdx <= 0) continue;
    const key = trimmed.slice(0, eqIdx);
    let value = trimmed.slice(eqIdx + 1);
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    if (!(key in process.env)) process.env[key] = value;
  }
}

loadEnvFile(path.resolve(import.meta.dirname, "../.env.local"));
loadEnvFile(path.resolve(import.meta.dirname, "../.env"));
process.env.NODE_TLS_REJECT_UNAUTHORIZED = "0";

const { inflateFromAttrs } = await import("../app/lib/ent-attr-codec.ts");

const sql = postgres(process.env.DATABASE_URL, {
  max: 1,
  prepare: false,
  connect_timeout: 60,
  ssl: { rejectUnauthorized: false },
});

async function loadRecord(etabId, collection, recordId) {
  const attrs = await sql`
    SELECT path, value
    FROM ent_collection_attr
    WHERE etablissement_id = ${etabId}
      AND collection = ${collection}
      AND record_id = ${recordId}
  `;
  if (attrs.length === 0) return null;
  return inflateFromAttrs(attrs.map((a) => ({ path: a.path, value: a.value })));
}

async function loadSingleton(etabId, collection) {
  return loadRecord(etabId, collection, "_");
}

function asRooms(payload) {
  if (!payload) return [];
  if (Array.isArray(payload)) return payload;
  if (Array.isArray(payload.rooms)) return payload.rooms;
  if (payload.__root != null) return asRooms(payload.__root);
  return [];
}

function asBookings(payload) {
  if (!payload) return [];
  if (Array.isArray(payload)) return payload;
  if (Array.isArray(payload.__root)) return payload.__root;
  if (Array.isArray(payload.reservations)) return payload.reservations;
  return [];
}

try {
  const etabs = await sql`SELECT id, slug FROM etablissement ORDER BY slug`;
  for (const etab of etabs) {
    const roomPayload =
      (await loadSingleton(etab.id, "reservation-rooms__rooms")) ||
      (await loadSingleton(etab.id, "reservation-rooms"));
    const bookPayload =
      (await loadSingleton(etab.id, "reservation-rooms__reservations")) ||
      (await loadRecord(etab.id, "reservation-rooms", "reservations"));

    const rooms = asRooms(roomPayload);
    const bookings = asBookings(bookPayload);

    let roomsUpserted = 0;
    for (const [i, r] of rooms.entries()) {
      const id = String(r?.id || "").trim();
      const name = String(r?.name || r?.label || "").trim();
      if (!id || !name) continue;
      await sql`
        INSERT INTO reservation_room (
          etablissement_id, id, name, building, sort_order, created_at, updated_at
        ) VALUES (
          ${etab.id}, ${id}, ${name}, ${r.building ? String(r.building) : null},
          ${typeof r.sortOrder === "number" ? r.sortOrder : i}, now(), now()
        )
        ON CONFLICT (etablissement_id, id) DO UPDATE SET
          name = EXCLUDED.name,
          building = EXCLUDED.building,
          sort_order = EXCLUDED.sort_order,
          updated_at = now()
      `;
      roomsUpserted += 1;
    }

    let bookingsUpserted = 0;
    for (const b of bookings) {
      const id = String(b?.id || "").trim();
      const roomId = String(b?.roomId || "").trim();
      const startsAt = String(b?.startsAt || "").trim();
      if (!id || !roomId || !startsAt) continue;
      await sql`
        INSERT INTO reservation_room_booking (
          etablissement_id, id, room_id, group_id, user_id,
          first_name, last_name, booked_by_first_name, booked_by_last_name,
          booked_by_user_id, booked_for_other, email, subject, class_name, comment,
          starts_at, ends_at, status, cancelled_at, cancelled_by, cancel_reason,
          created_at, updated_at
        ) VALUES (
          ${etab.id}, ${id}, ${roomId},
          ${b.groupId ? String(b.groupId) : null},
          ${String(b.userId || "")},
          ${String(b.firstName || "")},
          ${String(b.lastName || "")},
          ${String(b.bookedByFirstName || "")},
          ${String(b.bookedByLastName || "")},
          ${b.bookedByUserId ? String(b.bookedByUserId) : null},
          ${Boolean(b.bookedForOther)},
          ${b.email ? String(b.email) : null},
          ${b.subject ? String(b.subject) : null},
          ${b.className ? String(b.className) : null},
          ${b.comment ? String(b.comment) : null},
          ${startsAt},
          ${String(b.endsAt || startsAt)},
          ${String(b.status || "CONFIRMED")},
          ${b.cancelledAt ? String(b.cancelledAt) : null},
          ${b.cancelledBy ? String(b.cancelledBy) : null},
          ${b.cancelReason ? String(b.cancelReason) : null},
          ${b.createdAt ? new Date(String(b.createdAt)) : new Date()},
          now()
        )
        ON CONFLICT (etablissement_id, id) DO UPDATE SET
          room_id = EXCLUDED.room_id,
          group_id = EXCLUDED.group_id,
          user_id = EXCLUDED.user_id,
          first_name = EXCLUDED.first_name,
          last_name = EXCLUDED.last_name,
          booked_by_first_name = EXCLUDED.booked_by_first_name,
          booked_by_last_name = EXCLUDED.booked_by_last_name,
          booked_by_user_id = EXCLUDED.booked_by_user_id,
          booked_for_other = EXCLUDED.booked_for_other,
          email = EXCLUDED.email,
          subject = EXCLUDED.subject,
          class_name = EXCLUDED.class_name,
          comment = EXCLUDED.comment,
          starts_at = EXCLUDED.starts_at,
          ends_at = EXCLUDED.ends_at,
          status = EXCLUDED.status,
          cancelled_at = EXCLUDED.cancelled_at,
          cancelled_by = EXCLUDED.cancelled_by,
          cancel_reason = EXCLUDED.cancel_reason,
          updated_at = now()
      `;
      bookingsUpserted += 1;
    }

    console.log({
      etab: etab.slug,
      roomsFound: rooms.length,
      bookingsFound: bookings.length,
      roomsUpserted,
      bookingsUpserted,
    });
  }

  const totals = await sql`
    SELECT
      (SELECT count(*)::int FROM reservation_room) AS rooms,
      (SELECT count(*)::int FROM reservation_room_booking) AS bookings
  `;
  console.log("totals", totals[0]);
} finally {
  await sql.end({ timeout: 5 });
}
