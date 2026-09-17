import assert from "node:assert/strict";
import test from "node:test";
import {
  eventTitleMatchesPattern,
  isValidDirectionSlug,
  normalizeRdvEventTitle,
  splitRdvTitlePatterns,
} from "@/app/lib/rdv-inscription-types";

test("normalizeRdvEventTitle — accents et casse", () => {
  assert.equal(normalizeRdvEventTitle("Rendez-vous Inscription"), "rendez vous inscription");
  assert.equal(normalizeRdvEventTitle("  Rendez-vous  d'inscription "), "rendez vous d'inscription");
});

test("eventTitleMatchesPattern — motif exact (sans synonymie)", () => {
  assert.equal(
    eventTitleMatchesPattern("Rendez-vous inscription collège", "rendez-vous inscription"),
    true,
  );
  assert.equal(eventTitleMatchesPattern("Réunion parents", "rendez-vous inscription"), false);
  assert.equal(eventTitleMatchesPattern("RDV INSCRIPTION", "rdv inscription"), true);
  // Pas d’équivalence automatique RDV ↔ rendez-vous
  assert.equal(eventTitleMatchesPattern("RDV inscription", "rendez-vous inscription"), false);
  assert.equal(
    eventTitleMatchesPattern("Inscription direction lycée", "rdv inscription | inscription direction"),
    true,
  );
});

test("splitRdvTitlePatterns", () => {
  assert.deepEqual(splitRdvTitlePatterns("rdv inscription | inscription direction"), [
    "rdv inscription",
    "inscription direction",
  ]);
  assert.deepEqual(splitRdvTitlePatterns("a;b\nc"), ["a", "b", "c"]);
});

test("isValidDirectionSlug", () => {
  assert.equal(isValidDirectionSlug("college"), true);
  assert.equal(isValidDirectionSlug("ecole-primaire"), true);
  assert.equal(isValidDirectionSlug("École"), false);
  assert.equal(isValidDirectionSlug(""), false);
});
