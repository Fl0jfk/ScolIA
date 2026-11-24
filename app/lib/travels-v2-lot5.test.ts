import { test } from "node:test";
import assert from "node:assert/strict";
import { MAX_INVITE_SEND_FAILURES, selectInvitationsToSend } from "@/app/lib/travels-v2-lot4";
import { mutTokenSendFailed, mutTokenSendOk } from "@/app/lib/travels-v2-invitation-mutations";
import { countCuisineDays } from "@/app/lib/travels-cuisine-form";
import type { FamilyInvitation } from "@/app/lib/travels-v2-registration-types";

function inv(over: Partial<FamilyInvitation> & { id: string }): FamilyInvitation {
  return {
    tripId: "trip12345",
    ine: `ine-${over.id}`,
    studentName: "Léa DUPONT",
    email: `${over.id}@x.fr`,
    createdAt: "2026-11-01T00:00:00Z",
    tokenHashes: [],
    sendCount: 0,
    answers: {},
    documents: {},
    exemptions: {},
    codeSends: [],
    ...over,
  };
}

test("L5-1 filet d'envoi : jamais envoyée = à envoyer ; en échec sous le plafond = à réessayer ; plafond = abandonnée", () => {
  const sel = selectInvitationsToSend([
    inv({ id: "neuve" }),
    inv({ id: "echec1", lastSendError: "SMTP", sendFailures: 1 }),
    inv({ id: "echec2", lastSendError: "SMTP", sendFailures: 2 }),
    inv({ id: "echec3", lastSendError: "SMTP", sendFailures: MAX_INVITE_SEND_FAILURES }),
    inv({ id: "ancienne", lastSendError: "SMTP" }), // échec antérieur au compteur : compte pour un
  ]);
  assert.deepEqual(sel.send, ["neuve"]);
  assert.deepEqual(sel.retry, ["echec1", "echec2", "ancienne"]);
  assert.deepEqual(sel.abandoned, ["echec3"]);
});

test("L5-2 filet d'envoi : ignore envoyées, signées, purgées, désistées et sans e-mail", () => {
  const sel = selectInvitationsToSend([
    inv({ id: "envoyee", firstSentAt: "2026-11-02T08:00:00Z", sendCount: 1 }),
    inv({ id: "signee", signature: {} as FamilyInvitation["signature"] }),
    inv({ id: "purgee", purgedAt: "2026-12-01T00:00:00Z" }),
    inv({ id: "desistee", withdrawal: { at: "2026-11-03T00:00:00Z", by: "x" } }),
    inv({ id: "sansmail", email: "" }),
  ]);
  assert.deepEqual(sel, { send: [], retry: [], abandoned: [] });
});

test("L5-3 compteur d'échecs : +1 par échec, remis à zéro par un envoi réussi", () => {
  const i = inv({ id: "a", tokenHashes: ["h1"] });
  mutTokenSendFailed(i, "h1", "SMTP");
  mutTokenSendFailed(i, "h2", "SMTP");
  assert.equal(i.sendFailures, 2);
  assert.equal(selectInvitationsToSend([i]).retry.length, 1);
  mutTokenSendOk(i, "2026-11-04T08:00:00Z", true);
  assert.equal(i.sendFailures, undefined);
  assert.equal(i.lastSendError, undefined);
  assert.equal(selectInvitationsToSend([i]).send.length, 0, "envoyée : plus rien à faire");
});

test("L5-4 jours de restauration : v2 (dates + miroir jour de semaine) ne compte que les dates réelles", () => {
  assert.equal(countCuisineDays({ "2027-01-29": true, vendredi: true }), 1, "sortie d'un jour : 1 jour, pas 2");
  assert.equal(countCuisineDays({ "2027-01-28": true, "2027-01-29": true, jeudi: true, vendredi: true, lundi: false }), 2);
  assert.equal(countCuisineDays({ "2027-01-29": false, vendredi: true }), 1, "date décochée + miroir : retombe sur les clés cochées");
});

test("L5-5 jours de restauration : v1 inchangé (jours de semaine cochés)", () => {
  assert.equal(countCuisineDays({ lundi: true, mardi: true, mercredi: false }), 2);
  assert.equal(countCuisineDays({}), 0);
  assert.equal(countCuisineDays(undefined), 0);
});
