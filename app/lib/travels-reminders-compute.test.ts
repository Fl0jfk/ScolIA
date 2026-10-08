import assert from "node:assert/strict";
import test from "node:test";
import { computeTripReminders } from "./travels-trip-helpers";
import type { TravelsTrip } from "./travels-types";

test("computeTripReminders — trip sans data ne lève pas", () => {
  const trip = {
    id: "t-empty",
    type: "SIMPLE",
    status: "VALIDE",
  } as TravelsTrip;
  const out = computeTripReminders(trip);
  assert.ok(Array.isArray(out));
});
