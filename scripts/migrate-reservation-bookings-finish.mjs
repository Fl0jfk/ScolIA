/**
 * Reprend la migration des bookings (skip déjà présents, reconnecte si reset).
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

function connect() {
  return postgres(process.env.DATABASE_URL, {
    max: 1,
    prepare: false,
    connect_timeout: 60,
    idle_timeout: 5,
    max_lifetime: 60 * 5,
    ssl: { rejectUnauthorized: false },
  });
}

function asBookings(payload) {
  if (!payload) return [];
  if (Array.isArray(payload)) return payload;
  if (Array.isArray(payload.__root)) return payload.__root;
  if (Array.isArray(payload.reservations)) return payload.reservations;
  return [];
}

function rowValues(etabId, b) {
  const id = String(b?.id || "").trim();
  const roomId = String(b?.roomId || "").trim();
  const startsAt = String(b?.startsAt || "").trim();
  if (!id || !roomId || !startsAt) return null;
  return {
    etablissement_id: etabId,
    id,
    room_id: roomId,
    group_id: b.groupId ? String(b.groupId) : null,
    user_id: String(b.userId || ""),
    first_name: String(b.firstName || ""),
    last_name: String(b.lastName || ""),
    booked_by_first_name: String(b.bookedByFirstName || ""),
    booked_by_last_name: String(b.bookedByLastName || ""),
    booked_by_user_id: b.bookedByUserId ? String(b.bookedByUserId) : null,
    booked_for_other: Boolean(b.bookedForOther),
    email: b.email ? String(b.email) : null,
    subject: b.subject ? String(b.subject) : null,
    class_name: b.className ? String(b.className) : null,
    comment: b.comment ? String(b.comment) : null,
    starts_at: startsAt,
    ends_at: String(b.endsAt || startsAt),
    status: String(b.status || "CONFIRMED"),
    cancelled_at: b.cancelledAt ? String(b.cancelledAt) : null,
    cancelled_by: b.cancelledBy ? String(b.cancelledBy) : null,
    cancel_reason: b.cancelReason ? String(b.cancelReason) : null,
    created_at: b.createdAt ? new Date(String(b.createdAt)) : new Date(),
    updated_at: new Date(),
  };
}

let sql = connect();
try {
  const [etab] = await sql`
    SELECT id, slug FROM etablissement WHERE slug = 'la-providence-nicolas-barre'
  `;
  const attrs = await sql`
    SELECT path, value FROM ent_collection_attr
    WHERE etablissement_id = ${etab.id}
      AND collection = 'reservation-rooms'
      AND record_id = 'reservations'
  `;
  const payload = inflateFromAttrs(attrs.map((a) => ({ path: a.path, value: a.value })));
  const bookings = asBookings(payload)
    .map((b) => rowValues(etab.id, b))
    .filter(Boolean);
  console.log("legacy valid", bookings.length);

  const existing = await sql`
    SELECT id FROM reservation_room_booking WHERE etablissement_id = ${etab.id}
  `;
  const have = new Set(existing.map((r) => r.id));
  const missing = bookings.filter((b) => !have.has(b.id));
  console.log("already", have.size, "missing", missing.length);

  for (let i = 0; i < missing.length; i += 100) {
    const chunk = missing.slice(i, i + 100);
    for (let attempt = 1; attempt <= 4; attempt++) {
      try {
        await sql`INSERT INTO reservation_room_booking ${sql(chunk)} ON CONFLICT (etablissement_id, id) DO NOTHING`;
        console.log(`inserted ${i}-${i + chunk.length - 1}`);
        break;
      } catch (e) {
        console.error(`retry ${attempt}`, e instanceof Error ? e.message : e);
        try {
          await sql.end({ timeout: 1 });
        } catch {
          /* ignore */
        }
        await new Promise((r) => setTimeout(r, 2000 * attempt));
        sql = connect();
        if (attempt === 4) throw e;
      }
    }
  }

  const n = await sql`
    SELECT count(*)::int AS n FROM reservation_room_booking WHERE etablissement_id = ${etab.id}
  `;
  console.log("final bookings", n[0].n, "/", bookings.length);
} finally {
  await sql.end({ timeout: 5 });
}
