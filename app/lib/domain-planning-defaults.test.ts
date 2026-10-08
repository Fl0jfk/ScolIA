import assert from "node:assert/strict";
import test from "node:test";
import {
  buildEmptyLyceeSessions,
  classesForTransversalNiveau,
  DEFAULT_DOMAIN_ID,
  DEFAULT_EVARS_SESSIONS,
  isTransversalNiveau,
} from "./domain-planning-defaults";

test("DEFAULT_EVARS_SESSIONS are attached to the default college domain", () => {
  assert.ok(DEFAULT_EVARS_SESSIONS.length > 0);
  for (const session of DEFAULT_EVARS_SESSIONS) {
    assert.equal(session.domainId, DEFAULT_DOMAIN_ID);
    assert.ok(isTransversalNiveau(session.niveau));
  }
});

test("classesForTransversalNiveau matches college and lycée class codes", () => {
  const classesByPole = {
    COLLÈGE: ["6A", "5B", "3C"],
    LYCÉE: ["2A", "1B", "TA", "TB"],
  };
  assert.deepEqual(classesForTransversalNiveau("6e", classesByPole), ["6A"]);
  assert.deepEqual(classesForTransversalNiveau("2nde", classesByPole), ["2A"]);
  assert.deepEqual(classesForTransversalNiveau("1ere", classesByPole), ["1B"]);
  assert.deepEqual(classesForTransversalNiveau("tle", classesByPole), ["TA", "TB"]);
});

test("buildEmptyLyceeSessions creates a 3×3 grid for a domain", () => {
  const sessions = buildEmptyLyceeSessions("evars-lycee");
  assert.equal(sessions.length, 9);
  assert.ok(sessions.every((s) => s.domainId === "evars-lycee"));
  assert.deepEqual(
    [...new Set(sessions.map((s) => s.niveau))].sort(),
    ["1ere", "2nde", "tle"],
  );
});
