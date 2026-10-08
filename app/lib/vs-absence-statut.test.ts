import assert from "node:assert/strict";
import test from "node:test";
import {
  assertAccueilEleveAbsenceCancelable,
  filterVsAbsencesVisiblesEtComptees,
  isVsAbsenceAccueilSlotActive,
  isVsAbsenceAnnulee,
  isVsAbsenceVisibleEtComptee,
  VS_ABSENCE_CANCEL_ACCUEIL_DEJA_TRAITEE,
  VS_ABSENCE_STATUT_ANNULEE,
} from "@/app/lib/vs-absence-statut";

test("annulee — invisible et non comptée", () => {
  assert.equal(isVsAbsenceAnnulee(VS_ABSENCE_STATUT_ANNULEE), true);
  assert.equal(isVsAbsenceVisibleEtComptee(VS_ABSENCE_STATUT_ANNULEE), false);
  assert.equal(isVsAbsenceVisibleEtComptee("a_traiter"), true);
  assert.equal(isVsAbsenceVisibleEtComptee("classee"), true);
  assert.equal(isVsAbsenceVisibleEtComptee("justifiee"), true);
});

test("filterVsAbsencesVisiblesEtComptees — retire les annulées", () => {
  const rows = [
    { id: "1", statut: "a_traiter" },
    { id: "2", statut: VS_ABSENCE_STATUT_ANNULEE },
    { id: "3", statut: "classee" },
  ];
  const out = filterVsAbsencesVisiblesEtComptees(rows);
  assert.deepEqual(out.map((r) => r.id), ["1", "3"]);
});

test("accueil — annulee et classee ne bloquent plus un créneau", () => {
  assert.equal(isVsAbsenceAccueilSlotActive("a_traiter"), true);
  assert.equal(isVsAbsenceAccueilSlotActive("classee"), false);
  assert.equal(isVsAbsenceAccueilSlotActive(VS_ABSENCE_STATUT_ANNULEE), false);
});

test("annulation accueil — refuse si déjà traitée par la vie scolaire", () => {
  assert.doesNotThrow(() => assertAccueilEleveAbsenceCancelable("a_traiter", "accueil"));
  for (const statut of ["justifiee", "non_justifiee", "classee"] as const) {
    assert.throws(
      () => assertAccueilEleveAbsenceCancelable(statut, "accueil"),
      (err: unknown) =>
        err instanceof Error && err.message === VS_ABSENCE_CANCEL_ACCUEIL_DEJA_TRAITEE,
    );
  }
  assert.throws(() => assertAccueilEleveAbsenceCancelable("a_traiter", "appel"));
});
