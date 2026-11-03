import assert from "node:assert/strict";
import test from "node:test";
import {
  SCOLA_BOOKED_PROP,
  SCOLA_BOOKING_ID_PROP,
  SCOLA_PENDING_PROP,
  hasScolaHoldMarker,
  privatePatchClearing,
  privatePatchClearingAllScola,
} from "@/app/lib/rdv-inscription-gcal-props";
import { parseManualRdvSlot } from "@/app/lib/rdv-inscription-manual-slot";

test("libération : les marques ScolIA sont envoyées à null (Google conserve une clé omise)", () => {
  const patch = privatePatchClearingAllScola({
    [SCOLA_BOOKED_PROP]: "true",
    [SCOLA_PENDING_PROP]: "true",
    [SCOLA_BOOKING_ID_PROP]: "b1",
    autre: "garde",
  });
  assert.deepEqual(patch, {
    [SCOLA_BOOKED_PROP]: null,
    [SCOLA_PENDING_PROP]: null,
    [SCOLA_BOOKING_ID_PROP]: null,
    autre: "garde",
  });
  assert.ok(SCOLA_BOOKED_PROP in patch, "la clé doit être présente, pas omise");
});

test("libération : marque absente envoyée quand même à null", () => {
  assert.deepEqual(privatePatchClearingAllScola(undefined), {
    [SCOLA_BOOKED_PROP]: null,
    [SCOLA_PENDING_PROP]: null,
    [SCOLA_BOOKING_ID_PROP]: null,
  });
});

test("libération partielle : seules les clés demandées sont effacées", () => {
  assert.deepEqual(
    privatePatchClearing({ [SCOLA_PENDING_PROP]: "true", [SCOLA_BOOKED_PROP]: "true" }, [
      SCOLA_PENDING_PROP,
    ]),
    { [SCOLA_PENDING_PROP]: null, [SCOLA_BOOKED_PROP]: "true" },
  );
});

test("détection de marque", () => {
  assert.equal(hasScolaHoldMarker(undefined), false);
  assert.equal(hasScolaHoldMarker({ [SCOLA_BOOKED_PROP]: "true" }), true);
  assert.equal(hasScolaHoldMarker({ [SCOLA_PENDING_PROP]: "true" }), true);
  assert.equal(hasScolaHoldMarker({ [SCOLA_BOOKING_ID_PROP]: "b1" }), false);
});

const NOW = new Date("2026-11-03T12:00:00Z");

test("saisie manuelle : horaire valide, heure de Paris", () => {
  const r = parseManualRdvSlot({ date: "2026-11-19", startTime: "17:00", endTime: "17:30", now: NOW });
  assert.equal(r.ok, true);
  if (!r.ok) return;
  // 19 novembre : heure d'hiver, Paris = UTC+1.
  assert.equal(r.startAt.toISOString(), "2026-11-19T16:00:00.000Z");
  assert.equal(r.endAt.toISOString(), "2026-11-19T16:30:00.000Z");
});

test("saisie manuelle : refus des cas invalides", () => {
  const bad = [
    { date: "19/11/2026", startTime: "17:00", endTime: "17:30" },
    { date: "2026-11-19", startTime: "17h", endTime: "17:30" },
    { date: "2026-11-19", startTime: "17:30", endTime: "17:00" },
    { date: "2026-11-19", startTime: "17:00", endTime: "17:00" },
    { date: "2026-11-19", startTime: "08:00", endTime: "13:00" },
    { date: "2026-11-02", startTime: "17:00", endTime: "17:30" },
    { date: "2027-02-31", startTime: "17:00", endTime: "17:30" },
    { date: "2026-11-19", startTime: "25:00", endTime: "25:30" },
  ];
  for (const b of bad) {
    assert.equal(parseManualRdvSlot({ ...b, now: NOW }).ok, false, JSON.stringify(b));
  }
});
