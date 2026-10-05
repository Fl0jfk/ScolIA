import assert from "node:assert/strict";
import test from "node:test";
import { assembleImpacts, buildTiroirA, buildTiroirC } from "./drawers";
import {
  countEnSortieOnCreneau,
  eachIsoDateInclusive,
  isCreneauVide,
  signalIdFor,
} from "./empty-slots";
import {
  clearCreneauVideSignalsForTravel,
  listCreneauVideSignals,
  replaceCreneauVideSignalsForTravel,
  resetCreneauVideSignalsStore,
} from "./signals";
import type { CreneauVideSignal } from "./types";

test("empty-slots — classe 100 % partie → créneau vidé", () => {
  const expected = ["e1", "e2", "e3"];
  const enSortie = new Set(["e1", "e2", "e3", "other"]);
  assert.equal(isCreneauVide({ expectedEleveIds: expected, enSortieEleveIds: enSortie }), true);
  const stats = countEnSortieOnCreneau({ expectedEleveIds: expected, enSortieEleveIds: enSortie });
  assert.equal(stats.vide, true);
  assert.equal(stats.expectedCount, 3);
  assert.equal(stats.enSortieCount, 3);
});

test("empty-slots — classe partielle → pas de signal vide", () => {
  const expected = ["e1", "e2", "e3"];
  const enSortie = new Set(["e1", "e2"]);
  assert.equal(isCreneauVide({ expectedEleveIds: expected, enSortieEleveIds: enSortie }), false);
  const stats = countEnSortieOnCreneau({ expectedEleveIds: expected, enSortieEleveIds: enSortie });
  assert.equal(stats.vide, false);
  assert.equal(stats.enSortieCount, 2);
});

test("empty-slots — population attendue vide → pas de signal", () => {
  assert.equal(
    isCreneauVide({ expectedEleveIds: [], enSortieEleveIds: new Set(["e1"]) }),
    false,
  );
});

test("empty-slots — dates inclusives", () => {
  assert.deepEqual(eachIsoDateInclusive("2026-03-10", "2026-03-12"), [
    "2026-03-10",
    "2026-03-11",
    "2026-03-12",
  ]);
  assert.deepEqual(eachIsoDateInclusive("2026-03-10", "2026-03-10"), ["2026-03-10"]);
  assert.deepEqual(eachIsoDateInclusive("bad", "2026-03-10"), []);
});

test("tiroirs — A+B présents, C = questions, pas d’écriture planning", () => {
  const impacts = assembleImpacts({
    participantCount: 12,
    participantLinkedCount: 12,
    panierRepasCount: 3,
    creneauxVidesCount: 2,
    status: "VALIDE",
    edtCoverage: "complete",
  });
  const byTiroir = { A: 0, B: 0, C: 0, D: 0 };
  for (const i of impacts) byTiroir[i.tiroir] += 1;
  assert.ok(byTiroir.A >= 3);
  assert.ok(byTiroir.B >= 2);
  assert.ok(byTiroir.C >= 3);
  assert.ok(byTiroir.D >= 2);

  const a = buildTiroirA();
  assert.ok(a.some((x) => /vs_absence_eleve/.test(x.constat)));
  assert.ok(a.some((x) => /teacher_planning_replacement/.test(x.constat)));

  const c = buildTiroirC();
  assert.ok(c.every((x) => typeof x.question === "string" && x.question.length > 0));
  assert.ok(
    !impacts.some((x) => /INSERT|écrire.*replacement|affecte automatiquement/i.test(x.constat)),
  );
});

test("tiroirs — ANNULE expose B annulation sans DELETE bulletin", () => {
  const impacts = assembleImpacts({
    participantCount: 5,
    participantLinkedCount: 5,
    panierRepasCount: 0,
    creneauxVidesCount: 0,
    status: "ANNULE",
  });
  assert.ok(impacts.some((i) => i.tiroir === "B" && /Annulation/.test(i.constat)));
});

test("signals — confirm pose, ANNULE efface ; jamais planning", () => {
  resetCreneauVideSignalsStore();
  const etab = "etab-1";
  const travelId = "trip-1";
  const signal: CreneauVideSignal = {
    id: signalIdFor({
      etablissementId: etab,
      travelId,
      date: "2026-03-10",
      creneauId: "c1",
    }),
    etablissementId: etab,
    travelId,
    date: "2026-03-10",
    creneauId: "c1",
    jourSemaine: 2,
    heureDebut: "08:00",
    heureFin: "09:00",
    classe: "3A",
    groupeId: null,
    enseignantNom: "Mme X",
    matiereLabel: "Maths",
    expectedCount: 28,
    enSortieCount: 28,
    kind: "creneau_vide",
    createdAt: new Date().toISOString(),
  };

  replaceCreneauVideSignalsForTravel(etab, travelId, [signal]);
  assert.equal(listCreneauVideSignals(etab).length, 1);

  clearCreneauVideSignalsForTravel(etab, travelId);
  assert.equal(listCreneauVideSignals(etab).length, 0);
  // Invariant §14.4 : ce module n’exporte aucune écriture planning.
  assert.equal(typeof (globalThis as { teacherPlanningReplacement?: unknown }).teacherPlanningReplacement, "undefined");
});
