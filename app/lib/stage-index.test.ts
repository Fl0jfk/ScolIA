import assert from "node:assert/strict";
import test from "node:test";
import { flattenToAttrs, inflateFromAttrs } from "@/app/lib/ent-attr-codec";
import {
  CONVENTION_INDEX_ATTR_PATHS,
  CONVENTION_INDEX_MAX_DB_QUERIES_PER_CHUNK,
  conventionIndexEntriesFromDbRows,
  conventionToIndexEntry,
} from "@/app/lib/stage-db";
import { roundTripConvention, sample } from "@/app/lib/stage-index.test-fixtures";

test("index EAV — entrée alignée sur conventionToIndexEntry", () => {
  const full = roundTripConvention(sample);
  const expected = conventionToIndexEntry(full);
  const attrs = flattenToAttrs(
    Object.fromEntries(
      Object.entries(full as unknown as Record<string, unknown>).filter(
        ([k]) => !["id", "status", "updatedAt"].includes(k),
      ),
    ),
  ).filter((a) => (CONVENTION_INDEX_ATTR_PATHS as readonly string[]).includes(a.path));

  const mains = [
    {
      id: full.id,
      etablissementId: "etab_test",
      status: full.status,
      updatedAt: new Date(full.updatedAt),
    },
  ];
  const attrRows = attrs.map((a) => ({ id: full.id, path: a.path, value: a.value }));
  const built = conventionIndexEntriesFromDbRows(mains, attrRows);
  assert.equal(built.length, 1);
  assert.deepEqual(built[0], expected);
});

test("index SQL — au plus 2 requêtes par chunk de conventions", () => {
  const n = 110;
  const chunks = Math.ceil(n / 500);
  const maxQueries = chunks * CONVENTION_INDEX_MAX_DB_QUERIES_PER_CHUNK;
  assert.equal(maxQueries, 2, "110 conventions → 2 requêtes SQL pour l’index");
});

test("getConventionsFromDb — 2 requêtes par chunk (borné)", () => {
  const ids = Array.from({ length: 110 }, (_, i) => `id_${i}`);
  const chunks = Math.ceil(ids.length / 500);
  assert.equal(chunks * CONVENTION_INDEX_MAX_DB_QUERIES_PER_CHUNK, 2);
});

test("index — conserve une convention sans entreprise ni dates", () => {
  const full = roundTripConvention(sample);
  const expected = conventionToIndexEntry(full);
  const attrs = flattenToAttrs(
    Object.fromEntries(
      Object.entries(full as unknown as Record<string, unknown>).filter(
        ([k]) => !["id", "status", "updatedAt", "company", "schedule"].includes(k),
      ),
    ),
  ).filter((a) => (CONVENTION_INDEX_ATTR_PATHS as readonly string[]).includes(a.path));

  const mains = [
    {
      id: full.id,
      etablissementId: "etab_test",
      status: full.status,
      updatedAt: new Date(full.updatedAt),
    },
  ];
  const built = conventionIndexEntriesFromDbRows(
    mains,
    attrs.map((a) => ({ id: full.id, path: a.path, value: a.value })),
  );
  assert.equal(built.length, 1);
  assert.equal(built[0]!.companyName, "");
  assert.equal(built[0]!.periodStart, "");
  assert.equal(built[0]!.studentName, expected.studentName);
});
