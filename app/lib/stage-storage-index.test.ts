import assert from "node:assert/strict";
import test from "node:test";
import { patchConventionIndexEntry } from "@/app/lib/stage-storage";
import type { StageConventionIndexEntry } from "@/app/lib/stage-types";

const baseEntry = (id: string): StageConventionIndexEntry => ({
  id,
  status: "signed",
  studentName: "Jean Dupont",
  className: "3A",
  level: "3e",
  companyName: "ACME",
  internshipKind: "stage_observation",
  periodStart: "2026-03-01",
  periodEnd: "2026-03-14",
  schoolYear: "2025-2026",
  updatedAt: "2026-01-01T00:00:00.000Z",
});

test("patchConventionIndexEntry — remplace une entrée existante", () => {
  const index = [baseEntry("a"), baseEntry("b")];
  const updated = { ...baseEntry("b"), companyName: "Nova", updatedAt: "2026-02-01T00:00:00.000Z" };
  const next = patchConventionIndexEntry(index, updated);
  assert.equal(next.length, 2);
  assert.equal(next[1]?.companyName, "Nova");
  assert.notEqual(next, index);
});

test("patchConventionIndexEntry — insère en tête si id inconnu", () => {
  const index = [baseEntry("a")];
  const fresh = baseEntry("z");
  const next = patchConventionIndexEntry(index, fresh);
  assert.equal(next.length, 2);
  assert.equal(next[0]?.id, "z");
});
