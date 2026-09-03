import { test } from "node:test";
import assert from "node:assert/strict";
import {
  toAbsenceDateOnly,
  toAbsenceIsoTimestamp,
  toAbsenceIsoTimestampOrNull,
} from "./absence-db-dates";

test("toAbsenceDateOnly accepte string et Date UTC minuit", () => {
  assert.equal(toAbsenceDateOnly("2026-09-03"), "2026-09-03");
  assert.equal(toAbsenceDateOnly("2026-09-03T12:00:00.000Z"), "2026-09-03");
  assert.equal(toAbsenceDateOnly(new Date(Date.UTC(2026, 8, 3))), "2026-09-03");
});

test("toAbsenceIsoTimestamp n’exige pas .toISOString() sur une string", () => {
  assert.equal(toAbsenceIsoTimestamp("2026-09-03T10:00:00.000Z"), "2026-09-03T10:00:00.000Z");
  assert.equal(toAbsenceIsoTimestamp("2026-09-03"), "2026-09-03T00:00:00.000Z");
  const d = new Date("2026-09-03T10:00:00.000Z");
  assert.equal(toAbsenceIsoTimestamp(d), "2026-09-03T10:00:00.000Z");
});

test("toAbsenceIsoTimestampOrNull gère null / vide", () => {
  assert.equal(toAbsenceIsoTimestampOrNull(null), null);
  assert.equal(toAbsenceIsoTimestampOrNull(""), null);
  assert.equal(toAbsenceIsoTimestampOrNull("2026-09-03T08:00:00.000Z"), "2026-09-03T08:00:00.000Z");
});

test("régression : string timestamp ne plante pas (ex. driver PG)", () => {
  // Avant : main.createdAt.toISOString() → TypeError si string
  const createdAt = "2026-09-01T08:15:00.123Z";
  const startAt = "2026-09-03T07:00:00.000Z";
  assert.doesNotThrow(() => {
    const out = {
      createdAt: toAbsenceIsoTimestamp(createdAt),
      startAt: toAbsenceIsoTimestamp(startAt, createdAt),
      startDate: toAbsenceDateOnly("2026-09-03"),
    };
    assert.equal(out.createdAt, createdAt);
    assert.equal(out.startAt, startAt);
    assert.equal(out.startDate, "2026-09-03");
  });
});
