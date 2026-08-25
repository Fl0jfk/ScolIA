/**
 * Smoke test BDD post-migration — compteurs tables Charlemagne VS/Notes.
 * Usage : node scripts/smoke-charlemagne-db.mjs
 */
import postgres from "postgres";

const url = process.env.DATABASE_URL;
if (!url) {
  console.error("DATABASE_URL manquant.");
  process.exit(1);
}

const sql = postgres(url, {
  ssl: process.env.NODE_TLS_REJECT_UNAUTHORIZED === "0" ? { rejectUnauthorized: false } : undefined,
  max: 1,
});

const tables = [
  "eleve",
  "edt_creneau",
  "calendrier_scolaire",
  "note_devoir",
  "note_competence_item",
  "vs_appel",
  "vs_absence_eleve",
  "vs_sanction",
  "vs_carnet_entree",
  "facture",
];

for (const t of tables) {
  const [row] = await sql.unsafe(`SELECT count(*)::int AS n FROM "${t}"`);
  console.log(`${t}: ${row.n}`);
}

const [etab] = await sql`SELECT id, nom FROM etablissement LIMIT 1`;
if (etab) {
  console.log(`\nÉtablissement: ${etab.nom} (${etab.id})`);
  const [eleves] = await sql`SELECT count(*)::int AS n FROM eleve WHERE etablissement_id = ${etab.id}`;
  console.log(`Élèves tenant: ${eleves.n}`);
}

console.log("\nSmoke OK.");
await sql.end();
