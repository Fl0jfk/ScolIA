import assert from "node:assert/strict";
import test from "node:test";
import {
  clampPanierRepasAssignments,
  countPanierRepasAssigned,
} from "./travels-eleves-list";
import type { TravelsParticipantEleve } from "./travels-types";

function p(
  ine: string,
  panierRepas?: boolean,
): TravelsParticipantEleve {
  return {
    ine,
    nom: ine,
    prenom: "X",
    droitImageOk: true,
    panierRepas,
  };
}

test("countPanierRepasAssigned", () => {
  assert.equal(countPanierRepasAssigned([p("a", true), p("b"), p("c", true)]), 2);
});

test("clampPanierRepasAssignments — respecte le plafond", () => {
  const out = clampPanierRepasAssignments(
    [p("a", true), p("b", true), p("c", true), p("d")],
    2,
  );
  assert.equal(countPanierRepasAssigned(out), 2);
  assert.equal(out[0]?.panierRepas, true);
  assert.equal(out[1]?.panierRepas, true);
  assert.equal(out[2]?.panierRepas, false);
});

test("clampPanierRepasAssignments — zéro commande → aucun panier", () => {
  const out = clampPanierRepasAssignments([p("a", true), p("b", true)], 0);
  assert.equal(countPanierRepasAssigned(out), 0);
});
