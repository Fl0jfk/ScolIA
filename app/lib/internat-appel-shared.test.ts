import assert from "node:assert/strict";
import test from "node:test";
import {
  INTERNAT_APPEL_MARQUE_LABELS,
  isInternatAppelMarque,
} from "@/app/lib/internat-appel-shared";

test("isInternatAppelMarque", () => {
  assert.equal(isInternatAppelMarque("present"), true);
  assert.equal(isInternatAppelMarque("absent"), true);
  assert.equal(isInternatAppelMarque("excuse"), true);
  assert.equal(isInternatAppelMarque("activite"), true);
  assert.equal(isInternatAppelMarque("retard"), false);
  assert.equal(INTERNAT_APPEL_MARQUE_LABELS.present, "Présent");
});
