import assert from "node:assert/strict";
import test from "node:test";
import {
  canBroadcastFromMatrix,
  canInitiateFromMatrix,
  DEFAULT_FAMILLE_MESSAGING_SETTINGS,
  filterFoyersForProfClasses,
  isProfesseurOnly,
  roleMatchesMatrixEntry,
} from "./famille-messaging-matrix";

test("matrice — admin / cpe peuvent initier", () => {
  const s = DEFAULT_FAMILLE_MESSAGING_SETTINGS;
  assert.equal(canInitiateFromMatrix(["admin"], s), true);
  assert.equal(canInitiateFromMatrix(["cpe"], s), true);
  assert.equal(canInitiateFromMatrix(["parent"], s), false);
});

test("matrice — professeur hors liste par défaut", () => {
  const s = DEFAULT_FAMILLE_MESSAGING_SETTINGS;
  assert.equal(canInitiateFromMatrix(["professeur"], s), false);
  assert.equal(
    canInitiateFromMatrix(["professeur"], { ...s, rolesCanInitiate: [...s.rolesCanInitiate, "professeur"] }),
    true,
  );
});

test("matrice — broadcast interdit au prof seul", () => {
  const s = {
    ...DEFAULT_FAMILLE_MESSAGING_SETTINGS,
    rolesCanInitiate: ["admin", "cpe", "professeur"],
    allowBroadcast: true,
  };
  assert.equal(canBroadcastFromMatrix(["professeur"], s), false);
  assert.equal(canBroadcastFromMatrix(["cpe"], s), true);
  assert.equal(canBroadcastFromMatrix(["professeur"], { ...s, allowBroadcast: false }), false);
});

test("matrice — isProfesseurOnly", () => {
  assert.equal(isProfesseurOnly(["professeur"]), true);
  assert.equal(isProfesseurOnly(["professeur", "cpe"]), false);
  assert.equal(isProfesseurOnly(["admin"]), false);
});

test("matrice — roleMatches direction", () => {
  assert.equal(roleMatchesMatrixEntry(["direction_college"], "direction"), true);
  assert.equal(roleMatchesMatrixEntry(["professeur"], "direction"), false);
});

test("matrice — filtre foyers par classes prof", () => {
  const foyers = [
    { id: "1", eleves: [{ classe: "4B" }] },
    { id: "2", eleves: [{ classe: "3A" }] },
  ];
  const filtered = filterFoyersForProfClasses(foyers, ["4B"]);
  assert.equal(filtered.length, 1);
  assert.equal(filtered[0].id, "1");
});
