/**
 * Copie mémoire de getCollectionRecord (PGlite, SCOLA_TEST_DB).
 * Vérifie qu'une écriture est toujours vue, même rapprochée, et qu'un appelant
 * ne peut pas modifier la copie gardée en mémoire.
 */
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  deleteCollectionRecord,
  getCollectionRecord,
  upsertCollectionRecord,
} from "@/app/lib/ent-collection-db";
import { closeDb, setIntegrationTestDb } from "@/db/index";
import {
  createStagePgliteFixture,
  destroyStagePgliteFixture,
} from "@/app/lib/test/stage-pglite";

process.env.SCOLIA_TEST_MODE = "1";
process.env.SCOLA_TEST_DB = "1";
process.env.ENT_CORE_DB = "1";

function bigList(tag: string, n = 120) {
  // 120 éléments × 3 champs = 360 attributs (au-dessus du seuil de mise en mémoire).
  return { __root: Array.from({ length: n }, (_, i) => ({ id: `i${i}`, label: `${tag}-${i}`, n: i })) };
}

type Root = { __root: Array<{ id: string; label: string; n: number }> };

test("getCollectionRecord — copie mémoire toujours à jour", async () => {
  const fixture = await createStagePgliteFixture();
  setIntegrationTestDb(fixture.db);
  const etab = fixture.etablissementId;
  try {
    await upsertCollectionRecord(etab, "test__memo", "_", bigList("a"));
    const r1 = await getCollectionRecord<Root>(etab, "test__memo", "_");
    assert.equal(r1?.__root[5].label, "a-5");

    // Modifier l'objet rendu ne doit pas toucher la copie mémoire.
    r1!.__root[5].label = "modifié par l'appelant";
    r1!.__root.push({ id: "x", label: "x", n: -1 });
    const r2 = await getCollectionRecord<Root>(etab, "test__memo", "_");
    assert.equal(r2?.__root[5].label, "a-5");
    assert.equal(r2?.__root.length, 120);

    // Écritures successives rapprochées : chacune doit être vue.
    for (const tag of ["b", "c", "d"]) {
      await upsertCollectionRecord(etab, "test__memo", "_", bigList(tag));
      const r = await getCollectionRecord<Root>(etab, "test__memo", "_");
      assert.equal(r?.__root[7].label, `${tag}-7`);
    }

    // Liste raccourcie.
    await upsertCollectionRecord(etab, "test__memo", "_", bigList("e", 80));
    const r3 = await getCollectionRecord<Root>(etab, "test__memo", "_");
    assert.equal(r3?.__root.length, 80);

    // Suppression : plus rien.
    await deleteCollectionRecord(etab, "test__memo", "_");
    assert.equal(await getCollectionRecord(etab, "test__memo", "_"), null);

    // Petit document : toujours correct (non mis en mémoire).
    await upsertCollectionRecord(etab, "test__memo", "small", { a: 1 });
    await upsertCollectionRecord(etab, "test__memo", "small", { a: 2 });
    const s = await getCollectionRecord<{ a: number }>(etab, "test__memo", "small");
    assert.equal(s?.a, 2);
  } finally {
    await closeDb();
    await destroyStagePgliteFixture(fixture);
  }
});
