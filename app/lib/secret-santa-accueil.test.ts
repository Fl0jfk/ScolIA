import assert from "node:assert/strict";
import { test } from "node:test";
import { santaHomeStatus, type SantaHomeSource } from "@/app/lib/secret-santa-accueil";

const base: SantaHomeSource = {
  revealAt: new Date("2026-12-04T11:00:00Z"),
  drawnAt: null,
  eventAt: null,
  response: "pending",
  openedAt: null,
  changedAt: null,
};
const before = new Date("2026-12-01T00:00:00Z");
const after = new Date("2026-12-04T12:00:00Z");
const drawn = { ...base, response: "accepted" as const, drawnAt: new Date("2026-12-04T11:00:01Z") };

test("l'encadré suit l'étape de la personne invitée", () => {
  assert.equal(santaHomeStatus(base, before), "answer");
  assert.equal(santaHomeStatus({ ...base, response: "accepted" }, before), "waiting");
  assert.equal(santaHomeStatus({ ...base, response: "accepted" }, after), "reveal");
  assert.equal(santaHomeStatus(drawn, after), "reveal");
  assert.equal(santaHomeStatus({ ...drawn, openedAt: after }, after), "opened");
  assert.equal(santaHomeStatus({ ...drawn, openedAt: after, changedAt: after }, after), "changed");
});

test("rien après un refus, un retrait ou des réponses closes sans « oui »", () => {
  assert.equal(santaHomeStatus({ ...base, response: "declined" }, before), null);
  assert.equal(santaHomeStatus({ ...base, response: "withdrawn" }, after), null);
  assert.equal(santaHomeStatus(base, after), null);
});

test("un Secret Santa terminé quitte l'accueil", () => {
  const withEvent = { ...drawn, eventAt: new Date("2026-12-18T11:30:00Z") };
  assert.equal(santaHomeStatus(withEvent, new Date("2026-12-18T20:00:00Z")), "reveal");
  assert.equal(santaHomeStatus(withEvent, new Date("2026-12-20T00:00:00Z")), null);
  assert.equal(santaHomeStatus(drawn, new Date("2027-01-10T00:00:00Z")), null);
});
