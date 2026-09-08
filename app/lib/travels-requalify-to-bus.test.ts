import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  mergeTransportRequestForRequalify,
  normalizeDatesForComplexTrip,
  resolveRequalifyToBusStatus,
} from "@/app/lib/travels-requalify-to-bus";

describe("resolveRequalifyToBusStatus", () => {
  it("garde la pédagogie initiale", () => {
    assert.deepEqual(resolveRequalifyToBusStatus("EN_ATTENTE_DIR_INITIAL"), {
      status: "EN_ATTENTE_DIR_INITIAL",
    });
  });

  it("ramène finances / validé vers PROF_LOGISTICS", () => {
    assert.deepEqual(resolveRequalifyToBusStatus("EN_ATTENTE_COMPTA"), {
      status: "PROF_LOGISTICS",
    });
    assert.deepEqual(resolveRequalifyToBusStatus("VALIDE"), {
      status: "PROF_LOGISTICS",
    });
  });

  it("ajuste previousStatus en BESOIN_MODIFICATION", () => {
    assert.deepEqual(resolveRequalifyToBusStatus("BESOIN_MODIFICATION", "EN_ATTENTE_COMPTA"), {
      status: "BESOIN_MODIFICATION",
      previousStatus: "PROF_LOGISTICS",
    });
  });
});

describe("normalizeDatesForComplexTrip", () => {
  it("dérive start/end depuis date SIMPLE", () => {
    const out = normalizeDatesForComplexTrip({ date: "2026-10-15", title: "Test" });
    assert.equal(out.startDate, "2026-10-15");
    assert.equal(out.endDate, "2026-10-15");
    assert.equal(out.date, "2026-10-15");
  });
});

describe("mergeTransportRequestForRequalify", () => {
  it("fusionne les champs bus", () => {
    const out = mergeTransportRequestForRequalify(
      { pickupPoint: "Ancien", stayOnSite: false },
      { pickupPoint: "  Gymnase  ", stayOnSite: true, freeText: "note" },
    );
    assert.equal(out.pickupPoint, "Gymnase");
    assert.equal(out.stayOnSite, true);
    assert.equal(out.freeText, "note");
  });
});
