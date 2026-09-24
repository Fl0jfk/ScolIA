/**
 * Rattache Séverine COLAS → Mme PLANTEC (validation absences OGEC).
 * Usage : node scripts/fix-ogec-absence-validator-colas.mjs
 */
import postgres from "postgres";
import { readFileSync, existsSync } from "fs";

function loadEnv() {
  for (const f of [".env.local", ".env", ".cursor/mcp.local.env"]) {
    if (!existsSync(f)) continue;
    for (const line of readFileSync(f, "utf8").split(/\r?\n/)) {
      const m = line.match(/^([A-Z0-9_]+)=(.*)$/);
      if (!m) continue;
      if (!process.env[m[1]]) process.env[m[1]] = m[2].replace(/^["']|["']$/g, "");
    }
  }
}

loadEnv();
const url = process.env.MCP_DATABASE_URL || process.env.DATABASE_URL;
if (!url) {
  console.error("DATABASE_URL manquant");
  process.exit(1);
}

const sql = postgres(url, {
  ssl: url.includes("127.0.0.1") || url.includes("localhost") ? false : "require",
  max: 1,
  connect_timeout: 25,
});

function serializeManager(person) {
  return JSON.stringify({
    email: String(person.email || "").trim().toLowerCase(),
    ...(person.userId ? { userId: String(person.userId) } : {}),
    ...(person.label ? { label: String(person.label) } : {}),
  });
}

try {
  console.error("target=", url.includes("127.0.0.1") || url.includes("localhost") ? "local" : "remote");

  const colas = await sql`
    SELECT id, first_name, last_name, email, external_user_id, manager_id, display_name
    FROM personnel
    WHERE last_name ILIKE '%colas%'
    ORDER BY updated_at DESC
    LIMIT 10
  `;
  console.log("personnel Colas:", colas);
  if (colas.length === 0) {
    console.error("Séverine Colas introuvable");
    process.exit(1);
  }

  const plantecUsers = await sql`
    SELECT id, name, email FROM "user"
    WHERE name ILIKE '%plantec%' OR email ILIKE '%plantec%' OR email ILIKE '%0762041%'
    LIMIT 5
  `;
  console.log("users Plantec:", plantecUsers);

  const plantecPers = await sql`
    SELECT id, first_name, last_name, email, external_user_id, display_name
    FROM personnel
    WHERE last_name ILIKE '%plantec%'
    LIMIT 5
  `;
  console.log("personnel Plantec:", plantecPers);

  const plantec = {
    email: "",
    userId: "",
    label: "Mme Elise PLANTEC",
  };
  if (plantecUsers[0]) {
    plantec.email = String(plantecUsers[0].email || "").toLowerCase();
    plantec.userId = String(plantecUsers[0].id || "");
    plantec.label = String(plantecUsers[0].name || plantec.label);
  } else if (plantecPers[0]) {
    plantec.email = String(plantecPers[0].email || "").toLowerCase();
    plantec.userId = String(plantecPers[0].external_user_id || "");
    plantec.label = String(plantecPers[0].display_name || plantec.label);
  }
  if (!plantec.email) plantec.email = "0762041f@ac-normandie.fr";

  const managerJson = serializeManager(plantec);
  console.log("validateur:", plantec, managerJson);

  for (const row of colas) {
    await sql`
      UPDATE personnel
      SET manager_id = ${managerJson}, updated_at = NOW()
      WHERE id = ${row.id}
    `;
    console.log("OK fiche:", row.display_name || `${row.first_name} ${row.last_name}`, row.id);
  }

  console.log("Absences en attente : le routage se lit depuis la fiche (pas de backfill SQL nécessaire).");
  console.log("OK");
} catch (e) {
  console.error(e);
  process.exit(1);
} finally {
  await sql.end({ timeout: 5 });
}
