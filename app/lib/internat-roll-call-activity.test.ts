import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  rollCallCanValidate,
  rollCallPendingActivityStudents,
} from "@/app/lib/internat-stats";
import { emptyRollCall, type InternatStudent } from "@/app/lib/internat-types";

function student(partial: Partial<InternatStudent> & { id: string; sexe: "M" | "F" }): InternatStudent {
  return {
    eleveRef: { nom: "Test", prenom: "A", folderName: "TEST—A" },
    etablissement: "Lycée",
    classe: "1re A",
    actif: true,
    createdAt: "",
    updatedAt: "",
    ...partial,
  };
}

describe("roll call activité en attente", () => {
  it("autorise de terminer les sections avec activité, mais bloque l'envoi", () => {
    const students = [
      student({ id: "b1", sexe: "M" }),
      student({ id: "b2", sexe: "M" }),
      student({ id: "g1", sexe: "F" }),
    ];
    const rc = emptyRollCall("2026-09-15", "soir");
    rc.boys.marks = { b1: "present", b2: "activite" };
    rc.boys.completed = true;
    rc.girls.marks = { g1: "present" };
    rc.girls.completed = true;

    assert.equal(rollCallPendingActivityStudents(rc, students).length, 1);
    assert.equal(rollCallCanValidate(rc, students), false);

    rc.boys.marks.b2 = "present";
    assert.equal(rollCallPendingActivityStudents(rc, students).length, 0);
    assert.equal(rollCallCanValidate(rc, students), true);
  });
});
