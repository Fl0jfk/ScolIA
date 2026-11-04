import { test } from "node:test";
import assert from "node:assert/strict";
import {
  PRECONVENTION_MEMORY_MAX_AGE_MS,
  PRECONVENTION_MEMORY_MAX_CHILDREN,
  findPreconventionMemoryIn,
  migratePreconventionMemoryV2,
  parsePreconventionMemoryList,
  removePreconventionMemory,
  upsertPreconventionMemory,
  type StagePreconventionDeviceMemory,
} from "./stage-preconvention-device-memory";

const NOW = Date.parse("2026-11-04T10:00:00.000Z");
const DAY = 24 * 60 * 60 * 1000;

function child(prenom: string, dateNaissance = "2012-05-01", proof?: string) {
  return { nom: "DUPONT", prenom, dateNaissance, classe: "4A", identityProof: proof };
}

test("migration v2 : une entrée devient une liste à un élément v3", () => {
  const raw = JSON.stringify({
    version: 2,
    nom: "Dupont",
    prenom: "Léa",
    dateNaissance: "2012-05-01",
    classe: "4A",
    identityProof: "proof-a",
    savedAt: new Date(NOW - DAY).toISOString(),
  });
  const list = migratePreconventionMemoryV2(raw, NOW);
  assert.equal(list.length, 1);
  assert.equal(list[0].version, 3);
  assert.equal(list[0].prenom, "Léa");
  assert.equal(list[0].identityProof, "proof-a");
});

test("migration v2 : entrée invalide, expirée ou d'une autre version ignorée", () => {
  assert.deepEqual(migratePreconventionMemoryV2(null, NOW), []);
  assert.deepEqual(migratePreconventionMemoryV2("pas du json", NOW), []);
  const old = JSON.stringify({
    version: 2,
    nom: "A",
    prenom: "B",
    dateNaissance: "2012-01-01",
    savedAt: new Date(NOW - PRECONVENTION_MEMORY_MAX_AGE_MS - DAY).toISOString(),
  });
  assert.deepEqual(migratePreconventionMemoryV2(old, NOW), []);
  assert.deepEqual(migratePreconventionMemoryV2(JSON.stringify({ version: 3 }), NOW), []);
});

test("upsert : deux enfants, le plus récent en tête", () => {
  let list: StagePreconventionDeviceMemory[] = [];
  list = upsertPreconventionMemory(list, child("Léa", "2012-05-01", "pa"), NOW - 2 * DAY);
  list = upsertPreconventionMemory(list, child("Tom", "2012-05-01", "pt"), NOW - DAY);
  assert.deepEqual(
    list.map((e) => e.prenom),
    ["Tom", "Léa"],
  );
  assert.equal(list[1].identityProof, "pa");
});

test("upsert : déduplication insensible à la casse, remplace et remonte en tête", () => {
  let list: StagePreconventionDeviceMemory[] = [];
  list = upsertPreconventionMemory(list, child("Léa", "2012-05-01", "old"), NOW - 3 * DAY);
  list = upsertPreconventionMemory(list, child("Tom"), NOW - 2 * DAY);
  list = upsertPreconventionMemory(
    list,
    { nom: " dupont ", prenom: "LÉA", dateNaissance: "2012-05-01", identityProof: "new" },
    NOW,
  );
  assert.equal(list.length, 2);
  assert.equal(list[0].prenom, "LÉA");
  assert.equal(list[0].identityProof, "new");
  assert.equal(list[0].classe, undefined);
});

test("upsert : jumeaux même nom/prénom de famille, dates ou prénoms distincts conservés", () => {
  let list: StagePreconventionDeviceMemory[] = [];
  list = upsertPreconventionMemory(list, child("Léa", "2012-05-01"), NOW - DAY);
  list = upsertPreconventionMemory(list, child("Léa", "2011-03-02"), NOW);
  assert.equal(list.length, 2);
});

test("upsert : limite à 6 enfants, le plus ancien est écarté", () => {
  let list: StagePreconventionDeviceMemory[] = [];
  for (let i = 0; i < 8; i++) {
    list = upsertPreconventionMemory(list, child(`Enfant${i}`), NOW + i * 1000);
  }
  assert.equal(list.length, PRECONVENTION_MEMORY_MAX_CHILDREN);
  assert.equal(list[0].prenom, "Enfant7");
  assert.ok(!list.some((e) => e.prenom === "Enfant0" || e.prenom === "Enfant1"));
});

test("upsert : identité incomplète ignorée", () => {
  const list = upsertPreconventionMemory([], { nom: "A", prenom: "", dateNaissance: "2012-01-01" });
  assert.deepEqual(list, []);
});

test("parse : expiration par entrée, les autres sont conservées", () => {
  const fresh = upsertPreconventionMemory([], child("Léa"), NOW - DAY)[0];
  const stale = {
    ...upsertPreconventionMemory([], child("Tom"), NOW)[0],
    savedAt: new Date(NOW - PRECONVENTION_MEMORY_MAX_AGE_MS - DAY).toISOString(),
  };
  const list = parsePreconventionMemoryList(JSON.stringify([stale, fresh]), NOW);
  assert.deepEqual(
    list.map((e) => e.prenom),
    ["Léa"],
  );
});

test("parse : JSON invalide ou non-liste donne une liste vide", () => {
  assert.deepEqual(parsePreconventionMemoryList("{", NOW), []);
  assert.deepEqual(parsePreconventionMemoryList('{"a":1}', NOW), []);
  assert.deepEqual(parsePreconventionMemoryList(null, NOW), []);
});

test("find et remove : la preuve reste propre à chaque enfant", () => {
  let list: StagePreconventionDeviceMemory[] = [];
  list = upsertPreconventionMemory(list, child("Léa", "2012-05-01", "pa"), NOW - DAY);
  list = upsertPreconventionMemory(list, child("Tom", "2012-05-01", "pt"), NOW);
  assert.equal(findPreconventionMemoryIn(list, { nom: "dupont", prenom: "tom" })?.identityProof, "pt");
  assert.equal(
    findPreconventionMemoryIn(list, { nom: "DUPONT", prenom: "Léa", dateNaissance: "2012-05-01" })
      ?.identityProof,
    "pa",
  );
  assert.equal(findPreconventionMemoryIn(list, { nom: "DUPONT", prenom: "Zoé" }), null);
  const after = removePreconventionMemory(list, child("Tom"));
  assert.deepEqual(
    after.map((e) => e.prenom),
    ["Léa"],
  );
});

test("find sans date : ambigu (jumeaux homonymes) donne null", () => {
  let list: StagePreconventionDeviceMemory[] = [];
  list = upsertPreconventionMemory(list, child("Léa", "2012-05-01"), NOW - DAY);
  list = upsertPreconventionMemory(list, child("Léa", "2011-03-02"), NOW);
  assert.equal(findPreconventionMemoryIn(list, { nom: "DUPONT", prenom: "Léa" }), null);
});
