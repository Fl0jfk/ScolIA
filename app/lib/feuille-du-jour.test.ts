import { test } from "node:test";
import assert from "node:assert/strict";
import {
  assertSortieNotBulletin,
  buildFeuilleDuJourRow,
  sortFeuilleDuJourRows,
  summarizeFeuilleDuJour,
} from "./feuille-du-jour";

test("feuille du jour — en_sortie ≠ bulletin", () => {
  const row = buildFeuilleDuJourRow({
    eleveId: "e1",
    nom: "CERVEAU",
    prenom: "Alice",
    classe: "4B",
    fact: {
      eleveId: "e1",
      tag: "en_sortie",
      source: "travel:t1",
      detail: { travelId: "t1" },
    },
  });
  assert.equal(row.tag, "en_sortie");
  assert.equal(row.excludeFromBulletin, true);
  assert.match(row.explanationFr, /pas une absence bulletin/i);
  assert.equal(row.travelId, "t1");
  assert.ok(assertSortieNotBulletin("en_sortie"));
});

test("feuille du jour — absent_vs ≠ en_sortie", () => {
  const row = buildFeuilleDuJourRow({
    eleveId: "e2",
    nom: "MALADE",
    prenom: "Bob",
    fact: { eleveId: "e2", tag: "absent_vs", source: "vs:1", detail: { motif: "Fièvre" } },
  });
  assert.equal(row.tag, "absent_vs");
  assert.equal(row.excludeFromBulletin, false);
  assert.match(row.labelFr, /absent/i);
});

test("feuille du jour — résumé + tri", () => {
  const rows = sortFeuilleDuJourRows([
    buildFeuilleDuJourRow({
      eleveId: "c",
      nom: "C",
      prenom: "C",
      fact: { eleveId: "c", tag: "en_cours", source: "default" },
    }),
    buildFeuilleDuJourRow({
      eleveId: "a",
      nom: "A",
      prenom: "A",
      fact: { eleveId: "a", tag: "en_sortie", source: "travel:x" },
    }),
    buildFeuilleDuJourRow({
      eleveId: "b",
      nom: "B",
      prenom: "B",
      fact: { eleveId: "b", tag: "absent_vs", source: "vs:y" },
    }),
  ]);
  assert.equal(rows[0]!.tag, "en_sortie");
  assert.equal(rows[1]!.tag, "absent_vs");
  assert.equal(rows[2]!.tag, "en_cours");
  const s = summarizeFeuilleDuJour(rows);
  assert.equal(s.total, 3);
  assert.equal(s.enSortie, 1);
  assert.equal(s.absentVs, 1);
});
