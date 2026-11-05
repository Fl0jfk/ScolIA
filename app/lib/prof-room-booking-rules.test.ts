import assert from "node:assert/strict";
import { test } from "node:test";
import { isBeyondBookingHorizon } from "@/app/lib/prof-room-booking-rules";

const now = new Date("2026-11-05T10:00:00");

test("horizon : dernier jour inclus, lendemain refusé", () => {
  assert.equal(isBeyondBookingHorizon("2026-11-05", 56, now), false);
  assert.equal(isBeyondBookingHorizon("2026-12-31", 56, now), false);
  assert.equal(isBeyondBookingHorizon("2027-01-01", 56, now), true);
});

test("horizon : date invalide refusée, 0 jour = défaut 56", () => {
  assert.equal(isBeyondBookingHorizon("pas-une-date", 56, now), true);
  assert.equal(isBeyondBookingHorizon("2026-12-31", 0, now), false);
});
