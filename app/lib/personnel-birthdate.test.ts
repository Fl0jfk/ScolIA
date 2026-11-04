import assert from "node:assert/strict";
import test from "node:test";
import { validatePersonnelBirthDate } from "@/app/lib/personnel-birthdate";

const TODAY = "2026-11-04";

test("date valide", () => {
  assert.deepEqual(validatePersonnelBirthDate("1985-03-12", TODAY), { ok: true, value: "1985-03-12" });
});
test("absente ou mal formée", () => {
  assert.equal(validatePersonnelBirthDate("", TODAY).ok, false);
  assert.equal(validatePersonnelBirthDate(null, TODAY).ok, false);
  assert.equal(validatePersonnelBirthDate("12/03/1985", TODAY).ok, false);
});
test("inexistante", () => {
  assert.equal(validatePersonnelBirthDate("1985-02-30", TODAY).ok, false);
  assert.equal(validatePersonnelBirthDate("1985-13-01", TODAY).ok, false);
});
test("futur, avant 1900, trop jeune", () => {
  assert.equal(validatePersonnelBirthDate("2030-01-01", TODAY).ok, false);
  assert.equal(validatePersonnelBirthDate("1899-12-31", TODAY).ok, false);
  assert.equal(validatePersonnelBirthDate("2015-01-01", TODAY).ok, false);
});
test("limite des 14 ans", () => {
  assert.equal(validatePersonnelBirthDate("2012-11-04", TODAY).ok, true);
  assert.equal(validatePersonnelBirthDate("2012-11-05", TODAY).ok, false);
});
