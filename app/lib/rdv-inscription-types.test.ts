import assert from "node:assert/strict";
import test from "node:test";
import {
  eventTitleMatchesPattern,
  isValidDirectionSlug,
  normalizeRdvEventTitle,
} from "@/app/lib/rdv-inscription-types";

test("normalizeRdvEventTitle — accents et casse", () => {
  assert.equal(normalizeRdvEventTitle("Rendez-vous Inscription"), "rendez vous inscription");
  assert.equal(normalizeRdvEventTitle("  Rendez-vous  d'inscription "), "rendez vous d'inscription");
});

test("eventTitleMatchesPattern", () => {
  assert.equal(
    eventTitleMatchesPattern("Rendez-vous inscription collège", "rendez-vous inscription"),
    true,
  );
  assert.equal(eventTitleMatchesPattern("Réunion parents", "rendez-vous inscription"), false);
  assert.equal(eventTitleMatchesPattern("RDV INSCRIPTION", "rdv inscription"), true);
});

test("isValidDirectionSlug", () => {
  assert.equal(isValidDirectionSlug("college"), true);
  assert.equal(isValidDirectionSlug("ecole-primaire"), true);
  assert.equal(isValidDirectionSlug("École"), false);
  assert.equal(isValidDirectionSlug(""), false);
});
