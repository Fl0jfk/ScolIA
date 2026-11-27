import assert from "node:assert/strict";
import { test } from "node:test";
import {
  buildSantaView,
  drawBlockerWithRules,
  mergeExclusions,
  santaMaxExclusionsPerPerson,
  type SantaExclusion,
  type SantaParticipantRecord,
  type SantaRecord,
} from "@/app/lib/secret-santa";
import { canManageSantaRules } from "@/app/lib/secret-santa-rules";

test("seul l'administrateur de l'établissement gère les incompatibilités", () => {
  const actor = (roles: string[], platformAdmin = false) => ({ roles, platformAdmin });
  assert.equal(canManageSantaRules(actor(["admin"])), true);
  assert.equal(canManageSantaRules(actor([], true)), true);
  for (const role of ["professeur", "direction", "direction_lycee", "administratif"]) {
    assert.equal(canManageSantaRules(actor([role])), false, role);
  }
});

const twenty = Array.from({ length: 20 }, (_, i) => `p${String(i).padStart(2, "0")}`);

test("jusqu'à cinq personnes à éviter à partir de seize participants", () => {
  assert.deepEqual([3, 4, 7, 10, 13, 15, 16, 40].map(santaMaxExclusionsPerPerson), [0, 1, 2, 3, 4, 4, 5, 5]);
  const five: SantaExclusion[] = ["p01", "p02", "p03", "p04", "p05"].map((other) => ["p00", other]);
  assert.equal(drawBlockerWithRules(twenty, [], five), null);
  assert.equal(drawBlockerWithRules(twenty.slice(0, 15), [], five), "confidential_rules");
  assert.equal(drawBlockerWithRules(twenty, [], [...five, ["p00", "p06"]]), "confidential_rules");
});

test("à vingt, trois personnes à éviter passent ; dans un petit groupe le tirage se bloque", () => {
  const three: SantaExclusion[] = [["p00", "p01"], ["p00", "p02"], ["p00", "p03"]];
  assert.equal(drawBlockerWithRules(twenty, [], three), null);
  assert.equal(drawBlockerWithRules(["p00", "p01", "p02"], [], [["p00", "p01"]]), "confidential_rules");
  // La limite compte les deux sources ensemble ; une même paire n'est comptée qu'une fois.
  assert.equal(drawBlockerWithRules(twenty.slice(0, 12), [["p00", "p04"]], three), "confidential_rules");
  assert.equal(drawBlockerWithRules(twenty.slice(0, 12), [["p01", "p00"]], three), null);
  assert.equal(mergeExclusions([["b", "a"]], [["a", "b"]]).length, 1);
});

test("la vue ne contient ni les incompatibilités ni leur nombre", () => {
  const santa: SantaRecord = {
    id: "s",
    organizerUserId: "org",
    organizerName: "Org",
    title: "T",
    budget: "",
    message: "",
    eventPlace: "",
    eventAt: null,
    version: 0,
    revealAt: new Date("2026-12-04T11:00:00Z"),
    drawnAt: null,
    exclusions: [],
  };
  const people: SantaParticipantRecord[] = ["a", "b", "c"].map((userId) => ({
    userId,
    displayName: userId,
    receiverUserId: null,
    invitedAt: null,
    openedAt: null,
    response: "accepted",
    changedAt: null,
  }));
  for (const viewer of ["org", "a"]) {
    const before = new Date("2026-12-01T00:00:00Z");
    assert.deepEqual(
      buildSantaView(santa, people, viewer, before, [["a", "b"]]),
      buildSantaView(santa, people, viewer, before),
    );
    const view = buildSantaView(santa, people, viewer, new Date("2026-12-05T00:00:00Z"), [["a", "b"]]);
    assert.equal(view.blocker, "confidential_rules");
    assert.equal(view.exclusionCount, 0);
    assert.doesNotMatch(JSON.stringify(view), /\["a","b"\]/);
  }
});
