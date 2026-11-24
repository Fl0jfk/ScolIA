import { test } from "node:test";
import assert from "node:assert/strict";

import { busQuoteCancelWindow } from "./travels-bus-quote-cancel";

test("ouvert jusqu'à 14 jours avant, verrouillé ensuite", () => {
  const start = "2026-12-20";
  assert.equal(busQuoteCancelWindow(start, new Date("2026-12-06T08:00:00Z")).open, true);
  const locked = busQuoteCancelWindow(start, new Date("2026-12-07T08:00:00Z"));
  assert.equal(locked.open, false);
  assert.equal(locked.lastDay, "2026-12-06");
});

test("départ passé ou date inconnue : verrouillé", () => {
  assert.equal(busQuoteCancelWindow("2026-11-01", new Date("2026-11-23T10:00:00Z")).open, false);
  assert.equal(busQuoteCancelWindow(undefined).open, false);
});
