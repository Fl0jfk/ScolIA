/**
 * Backfill scolarité : pas de réactivation des sortis, idempotence des écritures.
 * Usage : TEST_DATABASE_URL=… npm run test:eleve-scolarite-backfill:db
 */
import assert from "node:assert/strict";
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
import { beginTestDatabase, endTestDatabase } from "@/app/lib/test-database-harness";

function countEleveTableSelects(queries: string[]): number {
  return queries.filter((q) => {
    const lower = q.toLowerCase();
    if (!lower.includes("select")) return false;
    if (!/\beleve\b/.test(lower)) return false;
    if (
      lower.includes("eleve_scolarite") ||
      lower.includes("eleve_regime") ||
      lower.includes("eleve_foyer")
    ) {
      return false;
    }
    return true;
  }).length;
}

test("backfill — élève sorti (date passée) non réactivé", async (t) => {
  if (!beginTestDatabase(t)) return;

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
    await endTestDatabase(closeDb);
  }
});

test("applyClasseCourante — second passage sans changement ne met pas à jour", async (t) => {
  if (!beginTestDatabase(t)) return;

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
    await endTestDatabase(closeDb);
  }
});

test("backfillElevesScolariteCouranteOnce — marqueur persistant, second appel no-op", async (t) => {
  if (!beginTestDatabase(t)) return;

  const { getDb, closeDb } = await import("@/db/index");
  const {
    backfillElevesScolariteCouranteOnce,
    resetScolariteBackfillCacheForTests,
  } = await import("@/app/lib/ent-core-db");
  const { isEleveScolariteBackfillDone } = await import("@/app/lib/eleve-scolarite-backfill-marker");

  resetScolariteBackfillCacheForTests();

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
    await endTestDatabase(closeDb);
  }
});

test("backfillElevesScolariteCouranteOnce — marqueur déjà posé, aucune lecture élève", async (t) => {
  if (!beginTestDatabase(t)) return;

  const { getDb, closeDb, drainTestSqlLog, resetTestSqlLog } = await import("@/db/index");
  const {
    backfillElevesScolariteCouranteOnce,
    resetScolariteBackfillCacheForTests,
  } = await import("@/app/lib/ent-core-db");
  const { markEleveScolariteBackfillDone } = await import("@/app/lib/eleve-scolarite-backfill-marker");

  resetScolariteBackfillCacheForTests();
  resetTestSqlLog();

  const db = getDb();
  const slug = `test-backfill-marker-${randomUUID().slice(0, 8)}`;
  const [etab] = await db
    .insert(etablissement)
    .values({ slug, name: "Backfill marker", dataBucket: "scola-dev" })
    .returning({ id: etablissement.id });

  try {
    await markEleveScolariteBackfillDone(etab.id);
    resetTestSqlLog();
    const n = await backfillElevesScolariteCouranteOnce(etab.id);
    assert.equal(n, 0);
    assert.equal(countEleveTableSelects(drainTestSqlLog()), 0);
  } finally {
    await db.delete(tenantSettingAttr).where(eq(tenantSettingAttr.etablissementId, etab.id));
    await db.delete(etablissement).where(eq(etablissement.id, etab.id));
    await endTestDatabase(closeDb);
  }
});

test("backfillElevesScolariteCouranteOnce — appels concurrents, une seule passe liste", async (t) => {
  if (!beginTestDatabase(t)) return;

  const { getDb, closeDb, resetTestSqlLog } = await import("@/db/index");
  const {
    backfillElevesScolariteCouranteOnce,
    getScolariteBackfillRunCountForTests,
    resetScolariteBackfillCacheForTests,
  } = await import("@/app/lib/ent-core-db");

  resetScolariteBackfillCacheForTests();
  resetTestSqlLog();

  const db = getDb();
  const slug = `test-backfill-concurrent-${randomUUID().slice(0, 8)}`;
  const [etab] = await db
    .insert(etablissement)
    .values({ slug, name: "Backfill concurrent", dataBucket: "scola-dev" })
    .returning({ id: etablissement.id });

  await db.insert(anneeScolaire).values({
    etablissementId: etab.id,
    label: "2025-2026",
    isCurrent: true,
    startsOn: "2025-09-01",
    endsOn: "2026-08-31",
  });

  await db.insert(eleve).values({
    etablissementId: etab.id,
    sourceKey: `test:${randomUUID()}`,
    nom: "CONC",
    prenom: "Backfill",
    folderName: "CONC Backfill",
    classe: "5A",
    status: "inscrit",
  });

  try {
    resetTestSqlLog();
    const [n1, n2] = await Promise.all([
      backfillElevesScolariteCouranteOnce(etab.id),
      backfillElevesScolariteCouranteOnce(etab.id),
    ]);
    assert.equal(n1, n2);
    assert.equal(n1, 1);
    assert.equal(getScolariteBackfillRunCountForTests(), 1);
  } finally {
    await db.delete(eleveScolarite).where(eq(eleveScolarite.etablissementId, etab.id));
    await db.delete(eleve).where(eq(eleve.etablissementId, etab.id));
    await db.delete(anneeScolaire).where(eq(anneeScolaire.etablissementId, etab.id));
    await db.delete(tenantSettingAttr).where(eq(tenantSettingAttr.etablissementId, etab.id));
    await db.delete(etablissement).where(eq(etablissement.id, etab.id));
    resetScolariteBackfillCacheForTests();
    await endTestDatabase(closeDb);
  }
});

test("syncEleveScolariteFromEleveRow — pas de SELECT eleve redondant (snapshot backfill)", async (t) => {
  if (!beginTestDatabase(t)) return;

  const { getDb, closeDb, drainTestSqlLog, resetTestSqlLog } = await import("@/db/index");
  const { syncEleveScolariteFromEleveRow } = await import("@/app/lib/ent-core-db");

  resetTestSqlLog();

  const db = getDb();
  const slug = `test-backfill-n1-${randomUUID().slice(0, 8)}`;
  const [etab] = await db
    .insert(etablissement)
    .values({ slug, name: "Backfill N+1", dataBucket: "scola-dev" })
    .returning({ id: etablissement.id });

  await db.insert(anneeScolaire).values({
    etablissementId: etab.id,
    label: "2025-2026",
    isCurrent: true,
    startsOn: "2025-09-01",
    endsOn: "2026-08-31",
  });

  const [row] = await db
    .insert(eleve)
    .values({
      etablissementId: etab.id,
      sourceKey: `test:${randomUUID()}`,
      nom: "N1",
      prenom: "Test",
      folderName: "N1 Test",
      classe: "6B",
      status: "inscrit",
    })
    .returning({
      id: eleve.id,
      classe: eleve.classe,
      regime: eleve.regime,
      status: eleve.status,
      dateSortie: eleve.dateSortie,
    });

  try {
    resetTestSqlLog();
    await syncEleveScolariteFromEleveRow(etab.id, row);
    assert.equal(
      countEleveTableSelects(drainTestSqlLog()),
      0,
      "le registre plat est réutilisé sans relecture eleve",
    );
  } finally {
    await db.delete(eleveScolarite).where(eq(eleveScolarite.eleveId, row.id));
    await db.delete(eleve).where(eq(eleve.id, row.id));
    await db.delete(anneeScolaire).where(eq(anneeScolaire.etablissementId, etab.id));
    await db.delete(etablissement).where(eq(etablissement.id, etab.id));
    await endTestDatabase(closeDb);
  }
});
