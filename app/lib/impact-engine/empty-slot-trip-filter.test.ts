import assert from "node:assert/strict";
import test from "node:test";
import { filterTripsActiveOnParisDate } from "@/app/lib/impact-engine/empty-slot-trip-filter";
import type { TravelsTrip } from "@/app/lib/travels-types";

function trip(partial: Partial<TravelsTrip> & { id: string }): TravelsTrip {
  return {
    type: "SIMPLE",
    status: "VALIDE",
    data: {},
    ...partial,
  };
}

test("filterTripsActiveOnParisDate — status, listeElevesStatus et dates (contrat forListIndex)", () => {
  const active = filterTripsActiveOnParisDate(
    [
      trip({
        id: "a",
        status: "VALIDE",
        data: {
          startDate: "2026-10-14",
          endDate: "2026-10-16",
          listeElevesStatus: "confirmed",
        },
      }),
      trip({
        id: "b",
        status: "VALIDE",
        data: { startDate: "2026-10-20", listeElevesStatus: "confirmed" },
      }),
      trip({
        id: "c",
        status: "ANNULE",
        data: { startDate: "2026-10-14", listeElevesStatus: "confirmed" },
      }),
      trip({
        id: "d",
        status: "VALIDE",
        data: { startDate: "2026-10-14", listeElevesStatus: "draft" },
      }),
    ],
    "2026-10-15",
  );
  assert.deepEqual(active.map((t) => t.id), ["a"]);
});
