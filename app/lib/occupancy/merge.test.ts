import assert from "node:assert/strict";
import test from "node:test";
import {
  factsByEleveId,
  filterBulletinEligibleEleveIds,
  mergeCoverage,
  mergeOccupancySignals,
} from "./merge";
import { isExcludedFromBulletinAbsence, isStageAbsenceMotif } from "./types";

test("merge — sortie bat absence VS (jamais fusionnés)", () => {
  const fact = mergeOccupancySignals({
    eleveId: "e1",
    date: "2026-03-10",
    signals: [
      {
        eleveId: "e1",
        tag: "absent_vs",
        source: "vs_absence:a1",
        detail: { absenceId: "a1", motif: "malade" },
      },
      {
        eleveId: "e1",
        tag: "en_sortie",
        source: "travel:t1",
        detail: { travelId: "t1", snapshotClasse: "3eA", liveClasse: "3eB" },
      },
    ],
  });
  assert.equal(fact.tag, "en_sortie");
  assert.equal(fact.source, "travel:t1");
  assert.equal(fact.detail?.snapshotClasse, "3eA");
  assert.equal(fact.detail?.liveClasse, "3eB");
  assert.equal(isExcludedFromBulletinAbsence(fact.tag), true);
});

test("merge — stage bat absent_vs générique", () => {
  const fact = mergeOccupancySignals({
    eleveId: "e1",
    date: "2026-03-10",
    signals: [
      { eleveId: "e1", tag: "absent_vs", source: "vs_absence:a1" },
      { eleveId: "e1", tag: "en_stage", source: "vs_absence:a2" },
    ],
  });
  assert.equal(fact.tag, "en_stage");
  assert.equal(isExcludedFromBulletinAbsence(fact.tag), false);
});

test("merge — défaut en_cours si aucun signal", () => {
  const fact = mergeOccupancySignals({
    eleveId: "e1",
    date: "2026-03-10",
    signals: [],
    defaultTag: "en_cours",
  });
  assert.equal(fact.tag, "en_cours");
  assert.equal(fact.source, "scolarite");
});

test("filterBulletinEligible — élève en sortie exclu", () => {
  const facts = [
    mergeOccupancySignals({
      eleveId: "out",
      date: "2026-03-10",
      signals: [{ eleveId: "out", tag: "en_sortie", source: "travel:t" }],
    }),
    mergeOccupancySignals({
      eleveId: "sick",
      date: "2026-03-10",
      signals: [{ eleveId: "sick", tag: "absent_vs", source: "vs_absence:a" }],
    }),
    mergeOccupancySignals({
      eleveId: "here",
      date: "2026-03-10",
      signals: [],
      defaultTag: "en_cours",
    }),
  ];
  const eligible = filterBulletinEligibleEleveIds(
    ["out", "sick", "here"],
    factsByEleveId(facts),
  );
  assert.deepEqual(eligible, ["sick", "here"]);
});

test("isStageAbsenceMotif", () => {
  assert.equal(isStageAbsenceMotif("Stage — Acme [abc]"), true);
  assert.equal(isStageAbsenceMotif("stage - foo"), true);
  assert.equal(isStageAbsenceMotif("Maladie"), false);
});

test("mergeCoverage", () => {
  assert.equal(mergeCoverage(["complete", "partial"]), "partial");
  assert.equal(mergeCoverage(["complete", "unavailable"]), "unavailable");
  assert.equal(mergeCoverage(["complete"]), "complete");
});
