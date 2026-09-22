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

test("feuille du jour — a_infirmerie ≠ bulletin", () => {
  const row = buildFeuilleDuJourRow({
    eleveId: "e3",
    nom: "SOIN",
    prenom: "Clara",
    fact: {
      eleveId: "e3",
      tag: "a_infirmerie",
      source: "infirmerie_passage:i1",
      detail: { motif: "Maux de tête" },
    },
  });
  assert.equal(row.tag, "a_infirmerie");
  assert.equal(row.excludeFromBulletin, true);
  assert.match(row.explanationFr, /infirmerie/i);
  assert.ok(assertSortieNotBulletin("a_infirmerie"));
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
    buildFeuilleDuJourRow({
      eleveId: "d",
      nom: "D",
      prenom: "D",
      fact: { eleveId: "d", tag: "a_infirmerie", source: "infirmerie:z" },
    }),
    buildFeuilleDuJourRow({
      eleveId: "e",
      nom: "E",
      prenom: "E",
      fact: { eleveId: "e", tag: "hors_etablissement", source: "passage:p" },
    }),
  ]);
  assert.equal(rows[0]!.tag, "en_sortie");
  assert.equal(rows[1]!.tag, "a_infirmerie");
  assert.equal(rows[2]!.tag, "absent_vs");
  assert.equal(rows[3]!.tag, "hors_etablissement");
  assert.equal(rows[4]!.tag, "en_cours");
  const s = summarizeFeuilleDuJour(rows);
  assert.equal(s.total, 5);
  assert.equal(s.enSortie, 1);
  assert.equal(s.absentVs, 1);
  assert.equal(s.aInfirmerie, 1);
  assert.equal(s.horsEtablissement, 1);
});
