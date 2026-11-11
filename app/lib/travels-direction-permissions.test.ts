import assert from "node:assert/strict";
import test from "node:test";
import {
  canEditExistingTravelTrip,
  isForbiddenTravelStatusJump,
} from "@/app/lib/travels-direction-permissions";

test("NJ-07 : un professeur non porteur ne modifie pas le dossier d'un autre", () => {
  assert.equal(canEditExistingTravelTrip({ roles: ["professeur"], isOwner: false }), false);
  assert.equal(canEditExistingTravelTrip({ roles: ["surveillant"], isOwner: false }), false);
  assert.equal(canEditExistingTravelTrip({ roles: ["professeur"], isOwner: true }), true);
  assert.equal(canEditExistingTravelTrip({ roles: ["comptabilite"], isOwner: false }), true);
  assert.equal(canEditExistingTravelTrip({ roles: ["direction_college"], isOwner: false }), true);
});

test("NJ-07 : seul un rôle habilité passe un dossier à « validé »", () => {
  assert.equal(isForbiddenTravelStatusJump("EN_ATTENTE_DIR_INITIAL", "VALIDE", false), true);
  assert.equal(isForbiddenTravelStatusJump("VALIDE", "VALIDE", false), false);
  assert.equal(isForbiddenTravelStatusJump("EN_ATTENTE_DIR_INITIAL", "VALIDE", true), false);
  assert.equal(isForbiddenTravelStatusJump("PROF_LOGISTICS", "EN_ATTENTE_COMPTA", false), false);
});
