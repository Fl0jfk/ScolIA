/**
 * Applique 0025 (tables salles) + 0030 (absence_ingest_job) + 0031 (booked_by_user_id).
 * Idempotent. Retry réseau.
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

const url = process.env.DATABASE_URL;
if (!url) {
  console.error("DATABASE_URL manquant");
  process.exit(1);
}

const sqlFiles = [
  "../drizzle/0025_reservation_rooms_typed.sql",
  "../drizzle/0030_absence_ingest_job.sql",
  "../drizzle/0031_reservation_booked_by_user.sql",
].map((rel) => path.resolve(import.meta.dirname, rel));

function stripBreakpoints(sqlText) {
  return sqlText.replace(/-->\s*statement-breakpoint\s*/g, "\n");
}

async function once() {
  const sql = postgres(url, {
    max: 1,
    prepare: false,
    connect_timeout: 60,
    ssl: { rejectUnauthorized: false },
  });
  try {
    for (const sqlFile of sqlFiles) {
      const sqlText = stripBreakpoints(readFileSync(sqlFile, "utf8"));
      console.log("apply", path.basename(sqlFile));
      await sql.unsafe(sqlText);
    }
    const ingestCols = await sql`
      SELECT count(*)::int AS n
      FROM information_schema.columns
      WHERE table_name = 'absence_ingest_job'
    `;
    const bookedCol = await sql`
      SELECT column_name
      FROM information_schema.columns
      WHERE table_name = 'reservation_room_booking'
        AND column_name = 'booked_by_user_id'
    `;
    const tables = await sql`
      SELECT table_name
      FROM information_schema.tables
      WHERE table_schema = 'public'
        AND table_name IN ('absence_ingest_job', 'reservation_room', 'reservation_room_booking')
      ORDER BY table_name
    `;
    const nAbs = await sql`SELECT count(*)::int AS n FROM absence`;
    const nJobs = await sql`SELECT count(*)::int AS n FROM absence_ingest_job`;
    const nRooms = await sql`SELECT count(*)::int AS n FROM reservation_room`;
    const nBook = await sql`SELECT count(*)::int AS n FROM reservation_room_booking`;
    console.log(
      JSON.stringify(
        {
          tables: tables.map((t) => t.table_name),
          absence_ingest_job_columns: ingestCols[0]?.n,
          booked_by_user_id: bookedCol.length === 1,
          absence_rows: nAbs[0]?.n,
          ingest_jobs: nJobs[0]?.n,
          reservation_room_rows: nRooms[0]?.n,
          reservation_room_booking_rows: nBook[0]?.n,
        },
        null,
        2,
      ),
    );
  } finally {
    await sql.end({ timeout: 5 });
  }
}

let lastErr;
for (let i = 1; i <= 4; i++) {
  try {
    console.log(`tentative ${i}/4…`);
    await once();
    process.exit(0);
  } catch (e) {
    lastErr = e;
    console.error(`échec ${i}:`, e instanceof Error ? e.message : e);
    await new Promise((r) => setTimeout(r, 3000 * i));
  }
}
console.error("Impossible d’atteindre Postgres:", lastErr);
process.exit(1);
