/**
 * Intégration locale du socle élève (deux tenants, régime daté, prevue).
 * Usage : npx tsx --test --require ./scripts/stub-server-only.cjs app/lib/eleve-core/port.integration.test.ts
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";
import { randomUUID } from "node:crypto";
import { eq } from "drizzle-orm";
import { eleve, etablissement, metierEvent } from "@/db/schema";

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

const hasDb = Boolean(process.env.DATABASE_URL?.trim());

test("socle élève — tenant, régime daté, prevue (intégration)", async (t) => {
  if (!hasDb) {
    t.skip("DATABASE_URL absente");
    return;
  }

  const { getDb, closeDb } = await import("@/db/index");
  const {
    applyClasseCourante,
    applyRegimeChange,
    getEleve,
    getRegimeAtDate,
    openPrevueScolarite,
  } = await import("@/app/lib/eleve-core/port");
  const { EleveCoreError } = await import("@/app/lib/eleve-core/invariants");

  const db = getDb();
  const slugA = `test-eleve-a-${randomUUID().slice(0, 8)}`;
  const slugB = `test-eleve-b-${randomUUID().slice(0, 8)}`;
  const [etabA] = await db
    .insert(etablissement)
    .values({ slug: slugA, name: "Tenant A test socle", dataBucket: "scola-dev" })
    .returning({ id: etablissement.id });
  const [etabB] = await db
    .insert(etablissement)
    .values({ slug: slugB, name: "Tenant B test socle", dataBucket: "scola-dev" })
    .returning({ id: etablissement.id });

  const [thomas] = await db
    .insert(eleve)
    .values({
      etablissementId: etabA.id,
      sourceKey: `test:${randomUUID()}`,
      nom: "TESTCORE",
      prenom: "Thomas",
      folderName: "TESTCORE Thomas",
      status: "inscrit",
    })
    .returning({ id: eleve.id });

  try {
    await applyClasseCourante(
      { etablissementId: etabA.id, eleveId: thomas.id, classe: "4B", siteId: "college" },
      { skipHooks: true },
    );
    await applyRegimeChange(
      {
        etablissementId: etabA.id,
        eleveId: thomas.id,
        regime: "Demi-pension",
        effectiveOn: "2026-09-01",
      },
      { skipHooks: true },
    );
    await applyRegimeChange(
      {
        etablissementId: etabA.id,
        eleveId: thomas.id,
        regime: "Interne",
        effectiveOn: "2027-01-15",
      },
      { skipHooks: true },
    );

    assert.equal(await getRegimeAtDate({ etablissementId: etabA.id, eleveId: thomas.id, on: "2027-01-14" }), "Demi-pension");
    assert.equal(await getRegimeAtDate({ etablissementId: etabA.id, eleveId: thomas.id, on: "2027-01-15" }), "Interne");

    const snap = await getEleve({ etablissementId: etabA.id, eleveId: thomas.id });
    assert.equal(snap?.classe, "4B");
    assert.equal(snap?.regime, "Demi-pension");

    const ghost = await getEleve({ etablissementId: etabB.id, eleveId: thomas.id });
    assert.equal(ghost, null);

    await openPrevueScolarite(
      {
        etablissementId: etabA.id,
        eleveId: thomas.id,
        classe: "3A",
        siteId: "college",
      },
      { skipHooks: true },
    );
    const afterPrevue = await getEleve({ etablissementId: etabA.id, eleveId: thomas.id });
    assert.equal(afterPrevue?.classe, "4B");
    assert.equal(afterPrevue?.prevue?.classe, "3A");

    await assert.rejects(
      () =>
        openPrevueScolarite(
          {
            etablissementId: etabA.id,
            eleveId: thomas.id,
            classe: "2nde",
            anneeScolaireId: afterPrevue?.anneeScolaireId ?? undefined,
          },
          { skipHooks: true },
        ),
      (err: unknown) => err instanceof EleveCoreError && err.code === "SAME_YEAR",
    );

    // Correction régime le même jour (import) — pas d’erreur d’ordre.
    await applyRegimeChange(
      {
        etablissementId: etabA.id,
        eleveId: thomas.id,
        regime: "Externe",
        effectiveOn: "2027-01-15",
      },
      { skipHooks: true },
    );
    assert.equal(
      await getRegimeAtDate({ etablissementId: etabA.id, eleveId: thomas.id, on: "2027-01-15" }),
      "Externe",
    );

    const { applyEleveStatus } = await import("@/app/lib/eleve-core/port");
    await applyEleveStatus(
      { etablissementId: etabA.id, eleveId: thomas.id, status: "ancien" },
      { skipHooks: true },
    );
    const afterAncien = await getEleve({ etablissementId: etabA.id, eleveId: thomas.id });
    assert.equal(afterAncien?.status, "ancien");
    assert.equal(afterAncien?.scolariteId, null);
    assert.ok(afterAncien?.prevue, "la scolarité prévue N+1 reste");

    const events = await db
      .select({ type: metierEvent.type })
      .from(metierEvent)
      .where(eq(metierEvent.eleveId, thomas.id));
    const types = new Set(events.map((e) => e.type));
    assert.ok(types.has("scolarite.opened") || types.has("scolarite.classe_changed"));
    assert.ok(types.has("eleve.regime_changed"));
    assert.ok(types.has("eleve.status_changed"));
  } finally {
    await db.delete(etablissement).where(eq(etablissement.id, etabA.id));
    await db.delete(etablissement).where(eq(etablissement.id, etabB.id));
    await closeDb();
  }
});
