import assert from "node:assert/strict";
import test from "node:test";
import { isOutingTokenExpired } from "@/app/lib/internat-storage";

test("NJ-22 : expiration du lien de décision de sortie", () => {
  const now = new Date("2026-11-11T12:00:00Z");
  assert.equal(isOutingTokenExpired("2026-11-20", now), false);
  assert.equal(isOutingTokenExpired("2026-11-11", now), false);
  assert.equal(isOutingTokenExpired("2026-11-01", now), true);
  assert.equal(isOutingTokenExpired("n/a", now), false);
});
