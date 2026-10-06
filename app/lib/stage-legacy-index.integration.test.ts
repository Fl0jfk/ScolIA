/**
 * Intégration stages (PGlite, SCOLA_TEST_DB — pas de DATABASE_URL distante).
 * Usage : npm run test:stages-perf
 */
import assert from "node:assert/strict";
import { test } from "node:test";
import { flattenToAttrs } from "@/app/lib/ent-attr-codec";
import { upsertCollectionRecord } from "@/app/lib/ent-collection-db";
import { randomUUID } from "node:crypto";
import { and, eq } from "drizzle-orm";
import { closeDb, setIntegrationTestDb } from "@/db/index";
import {
  etablissement,
  stageConvention,
  stageConventionAttr,
  stageToken,
  stageTokenAttr,
} from "@/db/schema";
import type { StageConvention } from "@/app/lib/stage-types";
import { sample } from "@/app/lib/stage-index.test-fixtures";
import {
  createStagePgliteFixture,
  destroyStagePgliteFixture,
} from "@/app/lib/test/stage-pglite";

process.env.SCOLIA_TEST_MODE = "1";
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

test("deux établissements — marqueurs distincts, lectures alternées sans réécriture", async () => {
  const fixture = await createStagePgliteFixture();
  setIntegrationTestDb(fixture.db);

  const [etabB] = await fixture.db
    .insert(etablissement)
    .values({
      slug: `stage-b-${randomUUID().slice(0, 8)}`,
      name: "Tenant B",
      dataBucket: "scola-dev",
    })
    .returning({ id: etablissement.id });
  assert.ok(etabB);

  const idA = "stg_conv_etab_a";
  const idB = "stg_conv_etab_b";
  const legacyA = {
    ...tomLegacyConvention(),
    id: idA,
    student: { ...sample.student, firstName: "A", lastName: "One", className: "4A" },
  };
  const legacyB = {
    ...tomLegacyConvention(),
    id: idB,
    student: { ...sample.student, firstName: "B", lastName: "Two", className: "4B" },
  };

  await upsertCollectionRecord(
    fixture.etablissementId,
    "stages__conventions",
    idA,
    legacyA as unknown as Record<string, unknown>,
  );
  await upsertCollectionRecord(
    etabB.id,
    "stages__conventions",
    idB,
    legacyB as unknown as Record<string, unknown>,
  );

  const { listConventionIndexFromDb } = await import("@/app/lib/stage-db");
  await listConventionIndexFromDb(fixture.etablissementId);
  await listConventionIndexFromDb(etabB.id);

  const markers = await fixture.db
    .select({ token: stageToken.token, etab: stageToken.etablissementId })
    .from(stageToken);
  const syncMarkers = markers.filter((m) => m.token.includes("conventions_legacy_synced:"));
  assert.equal(syncMarkers.length, 2);
  assert.notEqual(syncMarkers[0]!.token, syncMarkers[1]!.token);

  const [rowA] = await fixture.db
    .select({ updatedAt: stageConvention.updatedAt, status: stageConvention.status })
    .from(stageConvention)
    .where(eq(stageConvention.id, idA))
    .limit(1);
  assert.ok(rowA);

  await listConventionIndexFromDb(etabB.id);
  await listConventionIndexFromDb(fixture.etablissementId);

  const [rowAAfter] = await fixture.db
    .select({ updatedAt: stageConvention.updatedAt, status: stageConvention.status })
    .from(stageConvention)
    .where(eq(stageConvention.id, idA))
    .limit(1);
  assert.equal(rowAAfter?.status, rowA.status);
  assert.equal(rowAAfter?.updatedAt.getTime(), rowA.updatedAt.getTime());

  await closeDb();
  await destroyStagePgliteFixture(fixture);
});

test("migration concurrente — pas de violation de clé unique", async () => {
  const fixture = await createStagePgliteFixture();
  setIntegrationTestDb(fixture.db);

  const legacy = tomLegacyConvention();
  await upsertCollectionRecord(
    fixture.etablissementId,
    "stages__conventions",
    TOM_LEGACY_ID,
    legacy as unknown as Record<string, unknown>,
  );

  const { listConventionIndexFromDb } = await import("@/app/lib/stage-db");
  await Promise.all(
    Array.from({ length: 8 }, () => listConventionIndexFromDb(fixture.etablissementId)),
  );

  const rows = await fixture.db
    .select({ id: stageConvention.id })
    .from(stageConvention)
    .where(eq(stageConvention.etablissementId, fixture.etablissementId));
  assert.equal(rows.filter((r) => r.id === TOM_LEGACY_ID).length, 1);

  await closeDb();
  await destroyStagePgliteFixture(fixture);
});

test("legacy — ne remplace pas une convention déjà signée en base", async () => {
  const fixture = await createStagePgliteFixture();
  setIntegrationTestDb(fixture.db);

  const convId = "stg_conv_signed_keep";
  const signed: StageConvention = {
    ...sample,
    id: convId,
    status: "signed",
    updatedAt: "2026-03-22T08:00:00.000Z",
    signatures: sample.signatures,
  };
  const { upsertConventionInDb, ensureConventionsMigratedFromCollection, getConventionFromDb } =
    await import("@/app/lib/stage-db");
  await upsertConventionInDb(fixture.etablissementId, signed);

  const legacyDraft = {
    ...signed,
    status: "signatures_pending",
    updatedAt: "2026-01-01T00:00:00.000Z",
  };
  await upsertCollectionRecord(
    fixture.etablissementId,
    "stages__conventions",
    convId,
    legacyDraft as unknown as Record<string, unknown>,
  );

  await ensureConventionsMigratedFromCollection(fixture.etablissementId);
  const hit = await getConventionFromDb(fixture.etablissementId, convId);
  assert.equal(hit?.status, "signed");

  await closeDb();
  await destroyStagePgliteFixture(fixture);
});

test("convention illisible en base — legacy ne l’écrase pas (getConventionFromDbOrMigrate)", async () => {
  const fixture = await createStagePgliteFixture();
  setIntegrationTestDb(fixture.db);

  const convId = "stg_conv_unreadable_keep";
  const partial = {
    student: { firstName: "Léa", lastName: "Partial", className: "2A", level: "2nde" },
    schoolYear: "2025-2026",
    internshipKind: "pfmp",
    teacherReferent: { name: "Prof", email: "prof@test.dev" },
    signatures: [],
    history: [],
    createdBy: { role: "eleve", name: "Léa" },
  };
  await fixture.db.insert(stageConvention).values({
    id: convId,
    etablissementId: fixture.etablissementId,
    status: "signatures_pending",
    updatedAt: new Date("2026-03-19T09:00:00.000Z"),
  });
  const attrs = flattenToAttrs(partial);
  await fixture.db.insert(stageConventionAttr).values(
    attrs.map((a) => ({
      etablissementId: fixture.etablissementId,
      conventionId: convId,
      path: a.path,
      value: a.value,
    })),
  );

  const fullLegacy = { ...sample, id: convId, status: "signed", student: partial.student };
  await upsertCollectionRecord(
    fixture.etablissementId,
    "stages__conventions",
    convId,
    fullLegacy as unknown as Record<string, unknown>,
  );

  const { getConventionFromDbOrMigrate } = await import("@/app/lib/stage-db");
  const hit = await getConventionFromDbOrMigrate(fixture.etablissementId, convId);
  assert.equal(hit, null);

  const [main] = await fixture.db
    .select({ status: stageConvention.status })
    .from(stageConvention)
    .where(eq(stageConvention.id, convId))
    .limit(1);
  assert.equal(main?.status, "signatures_pending");

  await closeDb();
  await destroyStagePgliteFixture(fixture);
});

test("index typé — legacy non migrable : marqueur posé, seconde lecture sans rescan collection", async () => {
  const fixture = await createStagePgliteFixture();
  setIntegrationTestDb(fixture.db);

  const goodId = "stg_conv_index_ok";
  const {
    upsertConventionInDb,
    listConventionIndexFromDb,
    isConventionsLegacyMigrationComplete,
    resetConventionsLegacyMigrationMemoryForTests,
    getLegacyCollectConventionIdsCallsForTests,
  } = await import("@/app/lib/stage-db");
  resetConventionsLegacyMigrationMemoryForTests();

  await upsertConventionInDb(fixture.etablissementId, {
    ...sample,
    id: goodId,
    status: "signatures_pending",
  });

  await upsertCollectionRecord(fixture.etablissementId, "stages__conventions", "stg_legacy_unmigratable", {
    id: "stg_legacy_unmigratable",
    status: "signatures_pending",
    student: { firstName: "X", lastName: "Y", className: "1A", level: "6ème" },
  });

  const index = await listConventionIndexFromDb(fixture.etablissementId);
  assert.ok(index.some((e) => e.id === goodId));

  const complete = await isConventionsLegacyMigrationComplete(fixture.etablissementId);
  assert.equal(complete, true, "marqueur complete doit être décodé (b:1), pas comparé à 'true'");

  const tokenPk = `auto_purge:conventions_legacy_synced:${fixture.etablissementId}`;
  const [atRow] = await fixture.db
    .select({ value: stageTokenAttr.value })
    .from(stageTokenAttr)
    .where(
      and(
        eq(stageTokenAttr.etablissementId, fixture.etablissementId),
        eq(stageTokenAttr.token, tokenPk),
        eq(stageTokenAttr.path, "at"),
      ),
    )
    .limit(1);
  assert.ok(atRow?.value, "champ at du marqueur");

  const callsAfterFirst = getLegacyCollectConventionIdsCallsForTests();
  assert.ok(callsAfterFirst >= 1);

  resetConventionsLegacyMigrationMemoryForTests();
  const index2 = await listConventionIndexFromDb(fixture.etablissementId);
  assert.ok(index2.some((e) => e.id === goodId));
  assert.equal(
    getLegacyCollectConventionIdsCallsForTests(),
    0,
    "seconde lecture : cache mémoire, pas de rescan ent_collection",
  );

  const [atRowAfter] = await fixture.db
    .select({ value: stageTokenAttr.value })
    .from(stageTokenAttr)
    .where(
      and(
        eq(stageTokenAttr.etablissementId, fixture.etablissementId),
        eq(stageTokenAttr.token, tokenPk),
        eq(stageTokenAttr.path, "at"),
      ),
    )
    .limit(1);
  assert.equal(atRowAfter?.value, atRow?.value, "marqueur at inchangé après seconde lecture");

  await closeDb();
  await destroyStagePgliteFixture(fixture);
});

test("id convention global autre établissement — marqueur posé, pas de boucle", async () => {
  const fixture = await createStagePgliteFixture();
  setIntegrationTestDb(fixture.db);

  const sharedId = "stg_conv_global_pk_clone";
  const [etabB] = await fixture.db
    .insert(etablissement)
    .values({
      slug: `stage-global-${randomUUID().slice(0, 8)}`,
      name: "Tenant clone",
      dataBucket: "scola-dev",
    })
    .returning({ id: etablissement.id });
  assert.ok(etabB);

  await fixture.db.insert(stageConvention).values({
    id: sharedId,
    etablissementId: fixture.etablissementId,
    status: "signatures_pending",
    updatedAt: new Date("2026-03-21T10:00:00.000Z"),
  });

  const cloneLegacy = {
    ...tomLegacyConvention(),
    id: sharedId,
    student: { ...sample.student, firstName: "Clone", lastName: "Tenant", className: "5A" },
  };
  await upsertCollectionRecord(
    etabB.id,
    "stages__conventions",
    sharedId,
    cloneLegacy as unknown as Record<string, unknown>,
  );

  const { listConventionIndexFromDb, isConventionsLegacyMigrationComplete } = await import(
    "@/app/lib/stage-db"
  );
  await listConventionIndexFromDb(etabB.id);
  assert.equal(await isConventionsLegacyMigrationComplete(etabB.id), true);

  const rowsB = await fixture.db
    .select({ id: stageConvention.id })
    .from(stageConvention)
    .where(eq(stageConvention.etablissementId, etabB.id));
  assert.equal(rowsB.length, 0, "insert ignoré (PK globale)");

  await listConventionIndexFromDb(etabB.id);
  assert.equal(await isConventionsLegacyMigrationComplete(etabB.id), true);

  await closeDb();
  await destroyStagePgliteFixture(fixture);
});
