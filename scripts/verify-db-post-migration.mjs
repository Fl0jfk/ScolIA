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

const sql = postgres(process.env.DATABASE_URL, {
  max: 1,
  prepare: false,
  ssl: { rejectUnauthorized: false },
});

try {
  const row = await sql`
    SELECT
      (SELECT count(*)::int FROM absence) AS absences,
      (SELECT count(*)::int FROM vs_absence_eleve) AS vs_absences,
      (SELECT count(*)::int FROM absence_ingest_job) AS ingest_jobs,
      (SELECT count(*)::int FROM reservation_room) AS rooms,
      (SELECT count(*)::int FROM reservation_room_booking) AS bookings,
      (SELECT count(*)::int FROM reservation_room_booking WHERE status = 'CONFIRMED') AS bookings_confirmed,
      (SELECT EXISTS(
         SELECT 1 FROM information_schema.columns
         WHERE table_name = 'reservation_room_booking' AND column_name = 'booked_by_user_id'
       )) AS has_booked_by_user_id,
      (SELECT EXISTS(
         SELECT 1 FROM information_schema.tables WHERE table_name = 'absence_ingest_job'
       )) AS has_ingest_job,
      (SELECT EXISTS(
         SELECT 1 FROM information_schema.tables WHERE table_name = 'request'
       )) AS has_request
  `;
  console.log(JSON.stringify(row[0], null, 2));
} finally {
  await sql.end({ timeout: 5 });
}
