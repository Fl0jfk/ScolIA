import assert from "node:assert/strict";
import { test } from "node:test";
import { canAccessIntranetPath } from "@/app/lib/intranet-modules";

// Régression : /api/chatbot (les questions à ScolIA) n'était rattachée à aucun module,
// le proxy fail-closed la refusait donc à tout le monde, admin compris.

test("un admin peut parler à ScolIA", () => {
  assert.equal(canAccessIntranetPath("/api/chatbot", ["admin"], true), true);
});

test("le personnel peut parler à ScolIA et lire ses suggestions", () => {
  assert.equal(canAccessIntranetPath("/api/chatbot", ["professeur"], false), true);
  assert.equal(canAccessIntranetPath("/api/chatbot/capabilities", ["professeur"], false), true);
  assert.equal(canAccessIntranetPath("/api/chatbot/upload", ["professeur"], false), true);
});

test("l'alimentation de la base de connaissances reste réservée", () => {
  assert.equal(canAccessIntranetPath("/api/chatbot/ingest", ["professeur"], false), false);
  assert.equal(canAccessIntranetPath("/api/chatbot/ingest", ["admin"], true), true);
});

test("un parent n'accède pas à l'assistant institutionnel", () => {
  assert.equal(canAccessIntranetPath("/api/chatbot", ["parent"], false), false);
});
