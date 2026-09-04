import postgres from "postgres";

const url = process.env.DATABASE_URL;
if (!url) throw new Error("DATABASE_URL manquant");

const sql = postgres(url, {
  ssl: { rejectUnauthorized: false },
  connect_timeout: 30,
  max: 1,
});

async function run() {
  await sql`ALTER TABLE absence ADD COLUMN IF NOT EXISTS staff_preferred_treatment text`;
  await sql`ALTER TABLE absence ADD COLUMN IF NOT EXISTS staff_preferred_makeup_slots text`;
  await sql`ALTER TABLE absence ADD COLUMN IF NOT EXISTS direction_confirmed_makeup_slots text`;
  await sql`ALTER TABLE absence ADD COLUMN IF NOT EXISTS makeup_slots_relance_at timestamptz`;
  console.log("OK — colonnes rattrapage / relance créneaux sur absence");
  await sql.end();
}

run().catch((e) => {
  console.error("ERREUR:", e.message);
  process.exit(1);
});
