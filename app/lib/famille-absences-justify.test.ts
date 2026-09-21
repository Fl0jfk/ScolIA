/**
 * Tests purs — mapping famille absences (justif / canJustify).
 */

import { test } from "node:test";
import assert from "node:assert/strict";

/** Miroir de la logique UI FamilleAbsencesClient.canJustify — garder aligné. */
function canJustify(a: {
  justifie: boolean;
  statut: string;
  motifEnAttente?: boolean;
}): boolean {
  if (
    a.justifie ||
    a.statut === "justifiee" ||
    a.statut === "non_justifiee" ||
    a.statut === "classee"
  ) {
    return false;
  }
  return (
    a.statut === "en_cours" ||
    a.statut === "justif_recue" ||
    a.statut === "a_traiter" ||
    Boolean(a.motifEnAttente)
  );
}

test("famille canJustify — a_traiter / en_cours autorisés", () => {
  assert.equal(canJustify({ justifie: false, statut: "en_cours" }), true);
  assert.equal(canJustify({ justifie: false, statut: "a_traiter" }), true);
  assert.equal(canJustify({ justifie: false, statut: "justif_recue" }), true);
});

test("famille canJustify — déjà traitée refusée", () => {
  assert.equal(canJustify({ justifie: true, statut: "justifiee" }), false);
  assert.equal(canJustify({ justifie: false, statut: "non_justifiee" }), false);
  assert.equal(canJustify({ justifie: false, statut: "classee" }), false);
});
