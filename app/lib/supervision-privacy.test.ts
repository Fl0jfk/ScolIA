import { test } from "node:test";
import assert from "node:assert/strict";
import { isSupervisionPrivacyBlockedPath } from "./supervision-paths";

test("messagerie et APIs liées bloquées en supervision", () => {
  assert.equal(isSupervisionPrivacyBlockedPath("/messagerie"), true);
  assert.equal(isSupervisionPrivacyBlockedPath("/messagerie/abc"), true);
  assert.equal(isSupervisionPrivacyBlockedPath("/api/messaging"), true);
  assert.equal(isSupervisionPrivacyBlockedPath("/api/messaging/conversations"), true);
  assert.equal(isSupervisionPrivacyBlockedPath("/channels"), true);
  assert.equal(isSupervisionPrivacyBlockedPath("/api/channels/messages"), true);
});

test("annuaire channels/users/list reste disponible (sélecteurs RH)", () => {
  assert.equal(isSupervisionPrivacyBlockedPath("/api/channels/users/list"), false);
});

test("autres modules restent hors blocage privacy", () => {
  assert.equal(isSupervisionPrivacyBlockedPath("/rh"), false);
  assert.equal(isSupervisionPrivacyBlockedPath("/api/absences"), false);
  assert.equal(isSupervisionPrivacyBlockedPath("/dashboard"), false);
  assert.equal(isSupervisionPrivacyBlockedPath("/api/supervision/status"), false);
});
