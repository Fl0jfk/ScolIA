import assert from "node:assert/strict";
import { randomInt } from "node:crypto";
import { test } from "node:test";
import {
  buildSantaView,
  drawBlocker,
  drawSecretSanta,
  parseSantaInput,
  parseSantaTextUpdate,
  repairAfterWithdrawal,
  type SantaExclusion,
  type SantaPair,
  type SantaParticipantRecord,
  type SantaRecord,
  type SantaResponse,
} from "@/app/lib/secret-santa";
import { santaMail } from "@/app/lib/secret-santa-mail-text";

const rnd = (max: number) => randomInt(max);

test("le tirage attend trois « oui » ; les règles des absents ne comptent pas", () => {
  assert.equal(drawBlocker(["a", "b"], []), "not_enough");
  assert.equal(drawBlocker(["a", "b", "c"], [["d", "e"]]), null);
  assert.equal(drawBlocker(["a", "b", "c"], [["a", "b"]]), "too_many_exclusions");
});

function assertValid(pairs: SantaPair[], ids: string[]) {
  assert.deepEqual(pairs.map((p) => p.giver).sort(), [...ids].sort());
  assert.deepEqual(pairs.map((p) => p.receiver).sort(), [...ids].sort());
  assert.ok(pairs.every((p) => p.giver !== p.receiver));
}
const changes = (before: SantaPair[], after: SantaPair[]) =>
  after.filter((a) => before.find((b) => b.giver === a.giver)!.receiver !== a.receiver).length;

test("désistement dans une chaîne : une seule personne change de nom", () => {
  const before = [
    { giver: "a", receiver: "b" },
    { giver: "b", receiver: "c" },
    { giver: "c", receiver: "d" },
    { giver: "d", receiver: "a" },
  ];
  const after = repairAfterWithdrawal(before, "b", [], rnd);
  assertValid(after, ["a", "c", "d"]);
  assert.equal(after.find((p) => p.giver === "a")!.receiver, "c");
  assert.equal(changes(before, after), 1);
});

test("deux personnes qui s'offraient l'une à l'autre : deux noms changent", () => {
  const before = [
    { giver: "a", receiver: "b" },
    { giver: "b", receiver: "a" },
    { giver: "c", receiver: "d" },
    { giver: "d", receiver: "e" },
    { giver: "e", receiver: "c" },
  ];
  const after = repairAfterWithdrawal(before, "b", [], rnd);
  assertValid(after, ["a", "c", "d", "e"]);
  assert.equal(changes(before, after), 2);
});

test("désistements au hasard : résultat toujours valide et paires interdites respectées", () => {
  const ids = ["a", "b", "c", "d", "e", "f", "g"];
  const exclusions: SantaExclusion[] = [["a", "b"], ["c", "d"], ["e", "f"]];
  for (let i = 0; i < 500; i++) {
    const before = drawSecretSanta(ids, exclusions, rnd);
    const leaver = ids[rnd(ids.length)]!;
    const after = repairAfterWithdrawal(before, leaver, exclusions, rnd);
    assertValid(after, ids.filter((x) => x !== leaver));
    assert.ok(after.every((p) => !exclusions.some((e) => e.includes(p.giver) && e.includes(p.receiver))));
    assert.ok(changes(before, after) <= 2);
  }
});

test("des retraits ne peuvent pas réduire le tirage à une seule possibilité", () => {
  // À 7, a-b et a-c sont permis ; à 4 ils forceraient a et d à s'offrir l'un à l'autre.
  const exclusions: SantaExclusion[] = [["a", "b"], ["a", "c"]];
  let pairs = drawSecretSanta(["a", "b", "c", "d", "e", "f", "g"], exclusions, rnd);
  assert.throws(() => {
    for (const leaver of ["e", "f", "g"]) pairs = repairAfterWithdrawal(pairs, leaver, exclusions, rnd);
  }, /plus secret/);
  assert.ok(pairs.length > 4);
});

test("pas de désistement s'il restait moins de trois personnes", () => {
  const before = drawSecretSanta(["a", "b", "c"], [], rnd);
  assert.throws(() => repairAfterWithdrawal(before, "a", [], rnd), /moins de 3/);
});

const NOW = new Date("2026-11-27T12:00:00Z");
const base = { title: "Profs", revealAt: "2026-12-04T11:00:00Z", participantIds: ["a", "b", "c"] };

test("rendez-vous facultatif, mais forcément après le tirage", () => {
  assert.equal(parseSantaInput(base, NOW).eventAt, null);
  const input = parseSantaInput({ ...base, eventPlace: " Salle ", eventAt: "2026-12-18T11:30:00Z" }, NOW);
  assert.equal(input.eventPlace, "Salle");
  assert.throws(() => parseSantaInput({ ...base, eventAt: "2026-12-01T10:00:00Z" }, NOW), /après le tirage/);
  assert.equal(parseSantaTextUpdate({ eventAt: "" }).eventAt, null);
});

const santa: SantaRecord = {
  id: "s",
  organizerUserId: "org",
  organizerName: "Dominique",
  title: "Personnel",
  budget: "",
  message: "",
  eventPlace: "",
  version: 0,
  eventAt: null,
  revealAt: new Date("2026-12-04T11:00:00Z"),
  drawnAt: null,
  exclusions: [],
};

test("l'invitation n'évoque le budget et le rendez-vous que s'ils sont saisis", () => {
  const empty = santaMail("invitation", santa, "https://x.test/s", NOW).lines.join("\n");
  assert.match(empty, /invité\(e\) par Dominique/);
  assert.match(empty, /Je participe/);
  assert.doesNotMatch(empty, /budget/i);
  assert.doesNotMatch(empty, /Rendez-vous/);
  const full = santaMail(
    "invitation",
    { ...santa, budget: "15 €", eventPlace: "Salle des professeurs", eventAt: new Date("2026-12-18T11:30:00Z") },
    "https://x.test/s",
    NOW,
  ).lines.join("\n");
  assert.match(full, /Budget : 15 €/);
  assert.match(full, /Rendez-vous : vendredi 18 décembre.*12:30, Salle des professeurs/);
});

test("l'avertissement après désistement ne nomme personne", () => {
  const mail = santaMail("changed", { ...santa, drawnAt: NOW }, "https://x.test/s", NOW);
  assert.doesNotMatch(mail.lines.join("\n") + mail.subject, /Dominique|retir|désist/i);
});

const person = (userId: string, response: SantaResponse): SantaParticipantRecord => ({
  userId,
  displayName: `Nom ${userId}`,
  receiverUserId: null,
  invitedAt: null,
  openedAt: null,
  response,
  changedAt: null,
});
const people = [person("a", "accepted"), person("b", "declined"), person("c", "pending")];

test("les invités ne voient que les participants ; seul l'organisateur voit refus et absences", () => {
  const guest = buildSantaView(santa, people, "c", NOW);
  assert.deepEqual(guest.participants.map((p) => p.userId), ["a"]);
  assert.equal(guest.organizer, null);
  assert.ok(!JSON.stringify(guest).includes("Nom b"));
  assert.equal(guest.canRespond, true);
  assert.equal(guest.isParticipant, false);
  assert.deepEqual(
    buildSantaView(santa, people, "org", NOW).organizer!.invitees.map((i) => i.response),
    ["accepted", "declined", "pending"],
  );
});

test("la liste de l'organisateur le désigne, pour qu'il ne s'invite pas lui-même", () => {
  const view = buildSantaView({ ...santa, organizerUserId: "a" }, people, "a", NOW);
  assert.deepEqual(view.organizer!.invitees.map((i) => i.isOrganizer), [true, false, false]);
});

test("à l'heure dite, les réponses sont closes et un tirage impossible s'explique", () => {
  const late = new Date("2026-12-04T11:00:00Z");
  const view = buildSantaView(santa, people, "c", late);
  assert.equal(view.canRespond, false);
  assert.equal(view.blocker, "not_enough");
  assert.equal(buildSantaView(santa, people, "c", NOW).blocker, null);
});

test("après un changement, la personne est invitée à rouvrir son cadeau", () => {
  const drawn = [
    { ...person("a", "accepted"), receiverUserId: "b", openedAt: NOW, changedAt: NOW },
    { ...person("b", "accepted"), receiverUserId: "a" },
  ];
  const view = buildSantaView({ ...santa, drawnAt: NOW }, drawn, "a", new Date("2026-12-05T00:00:00Z"));
  assert.deepEqual([view.mine!.changed, view.mine!.alreadyOpened], [true, false]);
});
