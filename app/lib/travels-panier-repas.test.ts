import assert from "node:assert/strict";
import test from "node:test";
import {
  clampPanierRepasAssignments,
  clampPanierRepasWhoEats,
  countPanierRepasAssigned,
  countPanierRepasTotalAssigned,
  isCuisineWhoEatsComplete,
} from "./travels-eleves-list";
import type { TravelsPanierRepasAdulte, TravelsParticipantEleve } from "./travels-types";

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

function a(id: string, name = "Adulte"): TravelsPanierRepasAdulte {
  return { id, name, source: "autre" };
}

test("countPanierRepasAssigned", () => {
  assert.equal(countPanierRepasAssigned([p("a", true), p("b"), p("c", true)]), 2);
});

test("countPanierRepasTotalAssigned — élèves + adultes", () => {
  assert.equal(
    countPanierRepasTotalAssigned([p("a", true), p("b")], [a("1"), a("2")]),
    3,
  );
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

test("clampPanierRepasWhoEats — élèves d’abord, puis adultes", () => {
  const out = clampPanierRepasWhoEats(
    [p("a", true), p("b", true)],
    [a("1"), a("2"), a("3")],
    3,
  );
  assert.equal(countPanierRepasAssigned(out.participants), 2);
  assert.equal(out.adultes.length, 1);
});

test("isCuisineWhoEatsComplete — adultes comblent le total", () => {
  const details = {
    active: true,
    daysSelection: { lundi: true },
    orders: { lundi: { picnicTotal: "2" } },
  };
  assert.equal(
    isCuisineWhoEatsComplete({
      piqueNiqueDetails: details as never,
      participantEleves: [p("a", true)],
      panierRepasAdultes: [a("1")],
    }),
    true,
  );
});
