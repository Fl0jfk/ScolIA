import assert from "node:assert/strict";
import test from "node:test";
import { canDeclareAccueilAbsence } from "@/app/lib/accueil-absences-access";

test("NJ-34 : rôles autorisés à déclarer une absence à l'accueil", () => {
  for (const r of ["accueil", "cpe", "surveillant", "administratif", "direction_college", "admin"]) {
    assert.equal(canDeclareAccueilAbsence([r]), true, r);
  }
  for (const r of ["professeur", "comptabilite", "infirmerie"]) {
    assert.equal(canDeclareAccueilAbsence([r]), false, r);
  }
});
