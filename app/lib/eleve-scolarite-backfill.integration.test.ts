/**
 * Backfill scolarité : pas de réactivation des sortis, idempotence des écritures.
 * Usage : TEST_DATABASE_URL=… npx tsx --test --require ./scripts/stub-server-only.cjs app/lib/eleve-scolarite-backfill.integration.test.ts
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";
import { randomUUID } from "node:crypto";
import { and, eq } from "drizzle-orm";
import {
  anneeScolaire,
  eleve,
  eleveScolarite,
  etablissement,
  tenantSettingAttr,
} from "@/db/schema";

function loadEnvFile(path: string) {
  if (!existsSync(path)) return;
  for (const line of readFileSync(path, "utf8").split("\n")) {
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

loadEnvFile(".env.local");
loadEnvFile(".env");

const dbUrl = process.env.TEST_DATABASE_URL?.trim() || process.env.DATABASE_URL?.trim();
const hasDb = Boolean(dbUrl);

test("backfill — élève sorti (date passée) non réactivé", async (t) => {
  if (!hasDb) {
    t.skip("TEST_DATABASE_URL / DATABASE_URL absente");
    return;
  }

  const { getDb, closeDb } = await import("@/db/index");
  const { syncScolariteCouranteFromPlat } = await import("@/app/lib/eleve-core/port");
  const { syncEleveScolariteFromEleveRow } = await import("@/app/lib/ent-core-db");

  const db = getDb();
  const slug = `test-backfill-sorti-${randomUUID().slice(0, 8)}`;
  const [etab] = await db
    .insert(etablissement)
    .values({ slug, name: "Backfill sorti", dataBucket: "scola-dev" })
    .returning({ id: etablissement.id });

  const past = "2025-06-30";
  const [row] = await db
    .insert(eleve)
    .values({
      etablissementId: etab.id,
      sourceKey: `test:${randomUUID()}`,
      nom: "SORTI",
      prenom: "Backfill",
      folderName: "SORTI Backfill",
      classe: "3A",
      status: "inscrit",
      dateSortie: past,
    })
    .returning({ id: eleve.id });

  const [annee] = await db
    .insert(anneeScolaire)
    .values({
      etablissementId: etab.id,
      label: "2025-2026",
      isCurrent: true,
      startsOn: "2025-09-01",
      endsOn: "2026-08-31",
    })
    .returning({ id: anneeScolaire.id });

  const [scol] = await db
    .insert(eleveScolarite)
    .values({
      etablissementId: etab.id,
      eleveId: row.id,
      anneeScolaireId: annee.id,
      classe: "3A",
      statut: "terminee",
    })
    .returning({ id: eleveScolarite.id, updatedAt: eleveScolarite.updatedAt });

  const [beforeEleve] = await db
    .select({ updatedAt: eleve.updatedAt })
    .from(eleve)
    .where(eq(eleve.id, row.id))
    .limit(1);

  try {
    await syncEleveScolariteFromEleveRow(etab.id, {
      id: row.id,
      classe: "3A",
      regime: null,
      status: "inscrit",
      dateSortie: past,
    });
    await syncScolariteCouranteFromPlat(
      {
        etablissementId: etab.id,
        eleveId: row.id,
        classe: "3A",
        status: "inscrit",
      },
      { skipHooks: true },
    );

    const [afterEleve] = await db
      .select({
        status: eleve.status,
        dateSortie: eleve.dateSortie,
        updatedAt: eleve.updatedAt,
      })
      .from(eleve)
      .where(eq(eleve.id, row.id))
      .limit(1);
    const [afterScol] = await db
      .select({ statut: eleveScolarite.statut, updatedAt: eleveScolarite.updatedAt })
      .from(eleveScolarite)
      .where(eq(eleveScolarite.id, scol.id))
      .limit(1);

    assert.equal(String(afterEleve?.dateSortie).slice(0, 10), past);
    assert.equal(afterScol?.statut, "terminee");
    assert.equal(afterEleve?.updatedAt?.getTime(), beforeEleve?.updatedAt?.getTime());
    assert.equal(afterScol?.updatedAt?.getTime(), scol.updatedAt?.getTime());
  } finally {
    await db.delete(eleveScolarite).where(eq(eleveScolarite.eleveId, row.id));
    await db.delete(eleve).where(eq(eleve.id, row.id));
    await db.delete(anneeScolaire).where(eq(anneeScolaire.id, annee.id));
    await db.delete(etablissement).where(eq(etablissement.id, etab.id));
    await closeDb();
  }
});

test("applyClasseCourante — second passage sans changement ne met pas à jour", async (t) => {
  if (!hasDb) {
    t.skip("TEST_DATABASE_URL / DATABASE_URL absente");
    return;
  }

  const { getDb, closeDb } = await import("@/db/index");
  const { applyClasseCourante } = await import("@/app/lib/eleve-core/port");

  const db = getDb();
  const slug = `test-backfill-idem-${randomUUID().slice(0, 8)}`;
  const [etab] = await db
    .insert(etablissement)
    .values({ slug, name: "Backfill idem", dataBucket: "scola-dev" })
    .returning({ id: etablissement.id });

  const [row] = await db
    .insert(eleve)
    .values({
      etablissementId: etab.id,
      sourceKey: `test:${randomUUID()}`,
      nom: "ACTIF",
      prenom: "Idem",
      folderName: "ACTIF Idem",
      classe: "4B",
      status: "inscrit",
    })
    .returning({ id: eleve.id });

  try {
    await applyClasseCourante(
      { etablissementId: etab.id, eleveId: row.id, classe: "4B" },
      { skipHooks: true },
    );

    const [before] = await db
      .select({ updatedAt: eleve.updatedAt })
      .from(eleve)
      .where(eq(eleve.id, row.id))
      .limit(1);

    await applyClasseCourante(
      { etablissementId: etab.id, eleveId: row.id, classe: "4B" },
      { skipHooks: true },
    );

    const [after] = await db
      .select({ updatedAt: eleve.updatedAt })
      .from(eleve)
      .where(eq(eleve.id, row.id))
      .limit(1);

    assert.equal(after?.updatedAt?.getTime(), before?.updatedAt?.getTime());
  } finally {
    await db.delete(eleveScolarite).where(eq(eleveScolarite.eleveId, row.id));
    await db.delete(eleve).where(eq(eleve.id, row.id));
    await db.delete(etablissement).where(eq(etablissement.id, etab.id));
    await closeDb();
  }
});

test("backfillElevesScolariteCouranteOnce — marqueur persistant, second appel no-op", async (t) => {
  if (!hasDb) {
    t.skip("TEST_DATABASE_URL / DATABASE_URL absente");
    return;
  }

  const { getDb, closeDb } = await import("@/db/index");
  const { backfillElevesScolariteCouranteOnce } = await import("@/app/lib/ent-core-db");
  const { isEleveScolariteBackfillDone } = await import("@/app/lib/eleve-scolarite-backfill-marker");

  const db = getDb();
  const slug = `test-backfill-once-${randomUUID().slice(0, 8)}`;
  const [etab] = await db
    .insert(etablissement)
    .values({ slug, name: "Backfill once", dataBucket: "scola-dev" })
    .returning({ id: etablissement.id });

  try {
    const n1 = await backfillElevesScolariteCouranteOnce(etab.id);
    assert.ok(n1 >= 0);
    assert.equal(await isEleveScolariteBackfillDone(etab.id), true);
    const n2 = await backfillElevesScolariteCouranteOnce(etab.id);
    assert.equal(n2, 0);
  } finally {
    await db.delete(tenantSettingAttr).where(eq(tenantSettingAttr.etablissementId, etab.id));
    await db.delete(etablissement).where(eq(etablissement.id, etab.id));
    await closeDb();
  }
});
