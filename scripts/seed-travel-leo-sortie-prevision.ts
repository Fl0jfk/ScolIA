/**
 * Seed voyage VALIDE pour Leo JUSTIF (Value Gate prévision cantine × en_sortie).
 * Idempotent : travel id `trip_seed_leo_sortie_prevision`.
 *
 * Usage :
 *   NODE_TLS_REJECT_UNAUTHORIZED=0 node --require ./scripts/stub-server-only.cjs --import tsx scripts/seed-travel-leo-sortie-prevision.ts
 */
import { existsSync, readFileSync } from "node:fs";
import { and, eq } from "drizzle-orm";
import { closeDb, getDb, isDatabaseConfigured } from "../db/index";
import { etablissement, travel, travelParticipant } from "../db/schema";

function loadEnvFile(path: string) {
  if (!existsSync(path)) return;
  for (const line of readFileSync(path, "utf8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const i = trimmed.indexOf("=");
    if (i < 0) continue;
    const key = trimmed.slice(0, i).trim();
    let val = trimmed.slice(i + 1).trim();
    if (
      (val.startsWith('"') && val.endsWith('"')) ||
      (val.startsWith("'") && val.endsWith("'"))
    ) {
      val = val.slice(1, -1);
    }
    if (!(key in process.env)) process.env[key] = val;
  }
}

loadEnvFile(".env.local");

const TENANT_SLUG = "default";
const ELEVE_ID = "872a645d-1db9-4f1c-98cf-9d7a89ed81d8";
const TRAVEL_ID = "trip_seed_leo_sortie_prevision";
const INE = "INEJUSTIF001";

function parisDateOffset(daysFromToday: number): string {
  const today = new Date().toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
  const d = new Date(`${today}T12:00:00+02:00`);
  d.setDate(d.getDate() + daysFromToday);
  return d.toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
}

async function main() {
  if (!isDatabaseConfigured()) throw new Error("DATABASE_URL manquant");
  process.env.ENT_CORE_DB = process.env.ENT_CORE_DB || "1";

  const db = getDb();
  const [etab] = await db
    .select()
    .from(etablissement)
    .where(eq(etablissement.slug, TENANT_SLUG))
    .limit(1);
  if (!etab) throw new Error(`Établissement ${TENANT_SLUG} introuvable`);

  const startDate = parisDateOffset(0);
  const endDate = parisDateOffset(1);

  await db
    .insert(travel)
    .values({
      id: TRAVEL_ID,
      etablissementId: etab.id,
      type: "sortie",
      status: "VALIDE",
      title: "Sortie seed prévision cantine",
      startDate,
      endDate,
      listeElevesStatus: "confirmed",
    })
    .onConflictDoUpdate({
      target: travel.id,
      set: {
        status: "VALIDE",
        title: "Sortie seed prévision cantine",
        startDate,
        endDate,
        listeElevesStatus: "confirmed",
      },
    });

  const existing = await db
    .select({ id: travelParticipant.id })
    .from(travelParticipant)
    .where(
      and(
        eq(travelParticipant.etablissementId, etab.id),
        eq(travelParticipant.travelId, TRAVEL_ID),
        eq(travelParticipant.eleveId, ELEVE_ID),
      ),
    )
    .limit(1);

  if (existing.length === 0) {
    await db.insert(travelParticipant).values({
      etablissementId: etab.id,
      travelId: TRAVEL_ID,
      eleveKey: INE,
      eleveId: ELEVE_ID,
      nom: "JUSTIF",
      prenom: "Leo",
      classe: "4B",
      sortOrder: 0,
    });
  }

  console.log(
    JSON.stringify(
      {
        ok: true,
        travelId: TRAVEL_ID,
        etablissementId: etab.id,
        eleveId: ELEVE_ID,
        startDate,
        endDate,
      },
      null,
      2,
    ),
  );
  await closeDb();
}

main().catch(async (e) => {
  console.error(e);
  try {
    await closeDb();
  } catch {
    /* ignore */
  }
  process.exit(1);
});
