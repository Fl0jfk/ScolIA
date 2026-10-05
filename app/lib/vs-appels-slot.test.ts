import assert from "node:assert/strict";
import test from "node:test";
import {
  appelAbsenceSlotsConflict,
  shouldPreserveAbsenceOnPresentLine,
} from "@/app/lib/vs-appels-slot";

test("shouldPreserveAbsenceOnPresentLine — motif conservé", () => {
  assert.equal(shouldPreserveAbsenceOnPresentLine({ motif: "Maladie", statut: "a_traiter" }), true);
  assert.equal(shouldPreserveAbsenceOnPresentLine({ statut: "justifiee" }), true);
  assert.equal(shouldPreserveAbsenceOnPresentLine({ statut: "a_traiter" }), false);
});

test("appelAbsenceSlotsConflict — autre appel même créneau horaire", () => {
  const slot = { dateAppel: "2026-10-06", heureDebut: "10:00", heureFin: "11:00", creneauId: "a" };
  const existing = {
    id: "x",
    appelId: "other",
    heureDebut: "10:00",
    heureFin: "11:00",
    statut: "a_traiter",
  };
  assert.equal(appelAbsenceSlotsConflict(slot, existing, "current"), true);
  assert.equal(
    appelAbsenceSlotsConflict(slot, { ...existing, appelId: "current" }, "current"),
    false,
  );
});
