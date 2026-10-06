import assert from "node:assert/strict";
import test from "node:test";
import {
  filterTripsForModuleList,
  isTripTravelDatePast,
  sortTripsActiveBeforePast,
} from "./travels-trip-helpers";
import type { TravelsTrip } from "./travels-types";

function trip(id: string, endDate: string, status = "VALIDE"): TravelsTrip {
  return {
    id,
    type: "SIMPLE",
    status,
    data: { date: endDate, title: id },
  };
}

test("filterTripsForModuleList — conserve les séjours passés (grisés en UI)", () => {
  const past = trip("past", "2020-01-01");
  const future = trip("future", "2099-06-01");
  const out = filterTripsForModuleList([past, future]);
  assert.equal(out.length, 2);
  assert.equal(out[0]!.id, "future");
  assert.equal(out[1]!.id, "past");
  assert.equal(isTripTravelDatePast(past), true);
});

test("filterTripsForModuleList — exclut annulés", () => {
  const out = filterTripsForModuleList([trip("x", "2099-01-01", "ANNULE")]);
  assert.equal(out.length, 0);
});

test("sortTripsActiveBeforePast", () => {
  const a = trip("a", "2099-01-01");
  const b = trip("b", "2020-01-01");
  const sorted = sortTripsActiveBeforePast([b, a]);
  assert.deepEqual(sorted.map((t) => t.id), ["a", "b"]);
});
