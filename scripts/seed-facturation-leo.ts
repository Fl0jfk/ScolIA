/**
 * Seed facturation familles pour Leo JUSTIF (Value Gate).
 * Idempotent : tarif FORFAIT_DP, foyer responsable parent@, foyer_facturation.
 *
 * Usage :
 *   NODE_TLS_REJECT_UNAUTHORIZED=0 node --require ./scripts/stub-server-only.cjs --import tsx scripts/seed-facturation-leo.ts
 */
import { existsSync, readFileSync } from "node:fs";
import { and, eq } from "drizzle-orm";
import { closeDb, getDb, isDatabaseConfigured } from "../db/index";
import {
  eleve,
  eleveFoyerLink,
  etablissement,
  foyer,
  foyerFacturation,
  foyerResponsable,
  tarif,
} from "../db/schema";
import { upsertTarif } from "../app/lib/facturation-db";

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
const PARENT_EMAIL = "parent@localhost.dev";
const TARIF_CODE = "FORFAIT_DP";

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

  const [leo] = await db
    .select({ id: eleve.id, nom: eleve.nom, prenom: eleve.prenom, regime: eleve.regime })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etab.id), eq(eleve.id, ELEVE_ID)))
    .limit(1);
  if (!leo) throw new Error("Leo JUSTIF introuvable — lancer seed:dev");

  if (!leo.regime?.trim()) {
    await db
      .update(eleve)
      .set({ regime: "Demi-pension", updatedAt: new Date() })
      .where(eq(eleve.id, ELEVE_ID));
  }

  const [link] = await db
    .select({ foyerId: eleveFoyerLink.foyerId })
    .from(eleveFoyerLink)
    .where(
      and(eq(eleveFoyerLink.etablissementId, etab.id), eq(eleveFoyerLink.eleveId, ELEVE_ID)),
    )
    .limit(1);

  let foyerId = link?.foyerId ?? null;
  if (!foyerId) {
    const [created] = await db
      .insert(foyer)
      .values({
        etablissementId: etab.id,
        label: `Foyer ${leo.nom}`,
        payeurEstFoyer: true,
      })
      .returning({ id: foyer.id });
    foyerId = created!.id;
    await db.insert(eleveFoyerLink).values({
      etablissementId: etab.id,
      eleveId: ELEVE_ID,
      foyerId,
    });
  }

  const [resp] = await db
    .select({ id: foyerResponsable.id })
    .from(foyerResponsable)
    .where(
      and(
        eq(foyerResponsable.etablissementId, etab.id),
        eq(foyerResponsable.foyerId, foyerId),
        eq(foyerResponsable.email, PARENT_EMAIL),
      ),
    )
    .limit(1);

  if (!resp) {
    await db.insert(foyerResponsable).values({
      etablissementId: etab.id,
      foyerId,
      nom: "Local",
      prenom: "Parent",
      email: PARENT_EMAIL,
      autoriteParentale: true,
      contactUrgence: true,
      payeur: true,
      recupereEnfant: true,
    });
  } else {
    await db
      .update(foyerResponsable)
      .set({ payeur: true })
      .where(eq(foyerResponsable.id, resp.id));
  }

  const [ff] = await db
    .select({ id: foyerFacturation.id })
    .from(foyerFacturation)
    .where(
      and(
        eq(foyerFacturation.etablissementId, etab.id),
        eq(foyerFacturation.foyerId, foyerId),
      ),
    )
    .limit(1);
  if (!ff) {
    await db.insert(foyerFacturation).values({
      etablissementId: etab.id,
      foyerId,
      codeAuxiliaire: "JUSTIF-LEO",
      acceptePrelevement: true,
      iban: "FR7630006000011234567890189",
      bic: "AGRIFRPP",
      rum: "RUM-JUSTIF-LEO-001",
      mandatDate: new Date().toISOString().slice(0, 10),
    });
  } else {
    await db
      .update(foyerFacturation)
      .set({
        acceptePrelevement: true,
        iban: "FR7630006000011234567890189",
        bic: "AGRIFRPP",
        rum: "RUM-JUSTIF-LEO-001",
        mandatDate: new Date().toISOString().slice(0, 10),
        updatedAt: new Date(),
      })
      .where(eq(foyerFacturation.id, ff.id));
  }

  const existingTarifs = await db
    .select({ id: tarif.id })
    .from(tarif)
    .where(and(eq(tarif.etablissementId, etab.id), eq(tarif.code, TARIF_CODE)))
    .limit(1);

  const tarifRow = await upsertTarif(etab.id, {
    id: existingTarifs[0]?.id,
    code: TARIF_CODE,
    libelle: "Forfait demi-pension (mensuel)",
    prixUnitaire: "120.00",
    periodicite: "mensuel",
    portee: "regime",
    porteeValeur: "demi",
    actif: true,
  });

  console.log(
    JSON.stringify(
      {
        ok: true,
        etablissementId: etab.id,
        eleveId: ELEVE_ID,
        foyerId,
        tarifId: tarifRow.id,
        tarifCode: TARIF_CODE,
        parentEmail: PARENT_EMAIL,
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
