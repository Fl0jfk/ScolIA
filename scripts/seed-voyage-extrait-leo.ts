/**
 * Seed extrait santé portee=voyage pour Leo (Value Gate PAI voyage).
 * Idempotent : libellé fixe.
 *
 * Usage :
 *   NODE_TLS_REJECT_UNAUTHORIZED=0 node --require ./scripts/stub-server-only.cjs --import tsx scripts/seed-voyage-extrait-leo.ts
 */
import { existsSync, readFileSync } from "node:fs";
import { and, eq } from "drizzle-orm";
import { closeDb, getDb, isDatabaseConfigured } from "../db/index";
import { etablissement, santeExtrait } from "../db/schema";

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
const LIBELLE = "PAI — risque anaphylaxie (adrénaline à emporter)";

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

  const existing = await db
    .select({ id: santeExtrait.id, actif: santeExtrait.actif })
    .from(santeExtrait)
    .where(
      and(
        eq(santeExtrait.etablissementId, etab.id),
        eq(santeExtrait.eleveId, ELEVE_ID),
        eq(santeExtrait.portee, "voyage"),
        eq(santeExtrait.libelle, LIBELLE),
      ),
    )
    .limit(1);

  let id: string;
  if (existing[0]) {
    id = existing[0].id;
    if (!existing[0].actif) {
      await db
        .update(santeExtrait)
        .set({ actif: true, updatedAt: new Date() })
        .where(eq(santeExtrait.id, id));
    }
  } else {
    const [row] = await db
      .insert(santeExtrait)
      .values({
        etablissementId: etab.id,
        eleveId: ELEVE_ID,
        portee: "voyage",
        libelle: LIBELLE,
        actif: true,
      })
      .returning({ id: santeExtrait.id });
    id = row!.id;
  }

  console.log(
    JSON.stringify(
      {
        ok: true,
        extraitId: id,
        eleveId: ELEVE_ID,
        portee: "voyage",
        libelle: LIBELLE,
        travelId: "trip_seed_leo_sortie_prevision",
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
