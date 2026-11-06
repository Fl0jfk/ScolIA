import assert from "node:assert/strict";
import test from "node:test";
import { buildTravelMonthRail, travelMonthRailKey } from "./travels-month-rail";
import { sortTripsActiveBeforePast } from "./travels-trip-helpers";
import type { TravelsTrip } from "./travels-types";

function simple(id: string, date?: string): TravelsTrip {
  return { id, type: "SIMPLE", status: "VALIDE", data: { title: id, date } };
}

function complex(id: string, startDate: string, endDate: string): TravelsTrip {
  return { id, type: "COMPLEX", status: "VALIDE", data: { title: id, startDate, endDate } };
}

test("tri — à venir du plus proche au plus lointain, sans date ensuite, terminés du plus récent au plus ancien", () => {
  const sorted = sortTripsActiveBeforePast([
    simple("old", "2020-01-10"),
    simple("far", "2099-06-01"),
    simple("nodate"),
    simple("near", "2099-01-15"),
    simple("recent", "2021-05-01"),
  ]);
  assert.deepEqual(
    sorted.map((t) => t.id),
    ["near", "far", "nodate", "recent", "old"],
  );
});

test("clé — mois de départ, section selon la date de fin", () => {
  assert.equal(travelMonthRailKey(simple("a", "2099-03-04")), "upcoming:2099-03");
  assert.equal(travelMonthRailKey(simple("b", "2020-12-31")), "past:2020-12");
  assert.equal(travelMonthRailKey(simple("c")), "upcoming:none");
  // Voyage commencé il y a longtemps mais pas fini : reste « à venir », mois de départ.
  assert.equal(travelMonthRailKey(complex("d", "2020-02-01", "2099-01-01")), "upcoming:2020-02");
});

test("frise — regroupe par mois dans l'ordre de la liste, compte et premier séjour", () => {
  const trips = sortTripsActiveBeforePast([
    simple("m1a", "2099-01-05"),
    simple("m1b", "2099-01-20"),
    simple("m3", "2099-03-01"),
    simple("p1", "2020-06-10"),
    simple("p2", "2020-06-02"),
    simple("x"),
  ]);
  const rail = buildTravelMonthRail(trips, new Date(2099, 0, 10));
  assert.deepEqual(
    rail.map((e) => [e.key, e.count, e.firstTripId, e.label]),
    [
      ["upcoming:2099-01", 2, "m1a", "Janvier"],
      ["upcoming:2099-03", 1, "m3", "Mars"],
      ["upcoming:none", 1, "x", "Date à préciser"],
      ["past:2020-06", 2, "p1", "Juin"],
    ],
  );
  assert.equal(rail[0]!.isCurrentMonth, true);
  assert.equal(rail[0]!.shortLabel, "janv. 2099");
  assert.equal(rail[1]!.isCurrentMonth, false);
});

test("frise — liste vide", () => {
  assert.deepEqual(buildTravelMonthRail([]), []);
});
