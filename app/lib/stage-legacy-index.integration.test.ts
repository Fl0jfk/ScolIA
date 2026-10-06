/**
 * Intégration stages (PGlite, SCOLA_TEST_DB — pas de DATABASE_URL distante).
 * Usage : npm run test:stages-perf
 */
import assert from "node:assert/strict";
import { test } from "node:test";
import { flattenToAttrs } from "@/app/lib/ent-attr-codec";
import { upsertCollectionRecord } from "@/app/lib/ent-collection-db";
import { closeDb, setIntegrationTestDb } from "@/db/index";
import { stageConvention, stageConventionAttr } from "@/db/schema";
import type { StageConvention } from "@/app/lib/stage-types";
import { sample } from "@/app/lib/stage-index.test-fixtures";
import {
  createStagePgliteFixture,
  destroyStagePgliteFixture,
} from "@/app/lib/test/stage-pglite";

process.env.SCOLA_TEST_DB = "1";
process.env.ENT_CORE_DB = "1";

const TOM_LEGACY_ID = "stg_conv_tom_legacy";

function tomLegacyConvention(): StageConvention {
  return {
    ...sample,
    id: TOM_LEGACY_ID,
    status: "signatures_pending",
    updatedAt: "2026-03-21T10:00:00.000Z",
    student: {
      ...sample.student,
      firstName: "Tom",
      lastName: "Legacy",
      className: "4B",
      level: "Seconde",
    },
  };
}

test("legacy ent_collection — Tom Legacy visible (index, hub, recherche, dossier)", async () => {
  const fixture = await createStagePgliteFixture();
  setIntegrationTestDb(fixture.db);
  process.env.SCOLA_TEST_ETAB_ID = fixture.etablissementId;

  const legacy = tomLegacyConvention();
  await upsertCollectionRecord(
    fixture.etablissementId,
    "stages__conventions",
    TOM_LEGACY_ID,
    legacy as unknown as Record<string, unknown>,
  );

  const { listConventionIndexFromDb } = await import("@/app/lib/stage-db");
  const index = await listConventionIndexFromDb(fixture.etablissementId);

  const tomEntry = index.find((e) => e.id === TOM_LEGACY_ID);
  assert.ok(tomEntry, "Tom Legacy doit apparaître dans l’index après migration lazy");
  assert.match(tomEntry!.studentName, /Tom/i);
  assert.match(tomEntry!.studentName, /Legacy/i);
  assert.equal(tomEntry!.className, "4B");

  const { searchStageConventionsGlobal } = await import("@/app/lib/stage-class-roster");
  const searchHits = await searchStageConventionsGlobal("Tom Legacy");
  assert.ok(
    searchHits.some((h) => h.conventionId === TOM_LEGACY_ID),
    "recherche globale",
  );

  const { loadHubBoardStageConventions } = await import("@/app/lib/stage-convention-load");
  const hub = await loadHubBoardStageConventions();
  assert.ok(
    hub.some((c) => c.id === TOM_LEGACY_ID),
    "hub board",
  );

  const { listConventionsForDossier } = await import("@/app/lib/stage-storage");
  const dossier = await listConventionsForDossier({
    firstName: "Tom",
    lastName: "Legacy",
    className: "4B",
  });
  assert.ok(
    dossier.some((c) => c.id === TOM_LEGACY_ID),
    "listConventionsForDossier",
  );

  const { loadStageConventionsByIds } = await import("@/app/lib/stage-convention-load");
  const classLoads = await loadStageConventionsByIds(
    index.filter((e) => e.className === "4B").map((e) => e.id),
  );
  assert.ok(
    classLoads.some((c) => c.id === TOM_LEGACY_ID),
    "chargement liste classe",
  );

  delete process.env.SCOLA_TEST_ETAB_ID;
  await closeDb();
  await destroyStagePgliteFixture(fixture);
});

test("convention incomplète — absente de l’index et loadStageConventionsByIds ne plante pas", async () => {
  const fixture = await createStagePgliteFixture();
  setIntegrationTestDb(fixture.db);
  process.env.SCOLA_TEST_ETAB_ID = fixture.etablissementId;

  const brokenId = "stg_conv_broken_incomplete";
  const partial = {
    student: {
      firstName: "Zoé",
      lastName: "Incomplete",
      className: "3A",
      level: "3ème",
    },
    schoolYear: "2025-2026",
    internshipKind: "pfmp",
    teacherReferent: { name: "Prof", email: "prof@test.dev" },
    signatures: [],
    history: [],
    createdBy: { role: "eleve", name: "Zoé" },
  };
  const attrs = flattenToAttrs(partial);
  await fixture.db.insert(stageConvention).values({
    id: brokenId,
    etablissementId: fixture.etablissementId,
    status: "signatures_pending",
    updatedAt: new Date("2026-03-20T12:00:00.000Z"),
  });
  if (attrs.length > 0) {
    await fixture.db.insert(stageConventionAttr).values(
      attrs.map((a) => ({
        etablissementId: fixture.etablissementId,
        conventionId: brokenId,
        path: a.path,
        value: a.value,
      })),
    );
  }

  const { listConventionIndexFromDb, getConventionsFromDb } = await import("@/app/lib/stage-db");
  const index = await listConventionIndexFromDb(fixture.etablissementId);
  assert.equal(
    index.some((e) => e.id === brokenId),
    false,
    "index ne doit pas lister la convention illisible",
  );

  const { loadStageConventionsByIds } = await import("@/app/lib/stage-convention-load");
  const loaded = await loadStageConventionsByIds([brokenId]);
  assert.equal(loaded.some((c) => c.id === brokenId), false);
  assert.equal(loaded.length, 0);

  const fromDb = await getConventionsFromDb(fixture.etablissementId, [brokenId]);
  assert.equal(fromDb.length, 0);

  delete process.env.SCOLA_TEST_ETAB_ID;
  await closeDb();
  await destroyStagePgliteFixture(fixture);
});
