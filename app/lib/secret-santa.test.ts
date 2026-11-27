import assert from "node:assert/strict";
import { randomInt } from "node:crypto";
import { test } from "node:test";
import {
  SantaError,
  buildSantaView,
  drawSecretSanta,
  isDrawFeasible,
  parseSantaInput,
  type SantaExclusion,
  type SantaParticipantRecord,
  type SantaRecord,
} from "@/app/lib/secret-santa";
import { canAccessIntranetPath } from "@/app/lib/intranet-modules";

const rnd = (max: number) => randomInt(max);

function assertValidDraw(ids: string[], exclusions: SantaExclusion[]) {
  const pairs = drawSecretSanta(ids, exclusions, rnd);
  assert.deepEqual(pairs.map((p) => p.giver).sort(), [...ids].sort());
  assert.deepEqual(pairs.map((p) => p.receiver).sort(), [...ids].sort());
  for (const { giver, receiver } of pairs) {
    assert.notEqual(giver, receiver);
    assert.ok(!exclusions.some(([a, b]) => (a === giver && b === receiver) || (b === giver && a === receiver)));
  }
}

test("chacun offre à une autre personne, exclusions respectées dans les deux sens", () => {
  for (let i = 0; i < 300; i++) assertValidDraw(["a", "b", "c", "d", "e", "f"], [["a", "b"], ["c", "d"]]);
});

test("le tirage est équitable : les 9 résultats possibles à 4 sortent aussi souvent", () => {
  const counts = new Map<string, number>();
  const runs = 18000;
  for (let i = 0; i < runs; i++) {
    const key = drawSecretSanta(["a", "b", "c", "d"], [], rnd).map((p) => p.receiver).join("");
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  assert.equal(counts.size, 9);
  for (const n of counts.values()) assert.ok(Math.abs(n - runs / 9) < 250, `écart anormal : ${n}`);
});

test("des exclusions très serrées gardent une solution, trouvée par le repli", () => {
  // Une seule boucle autorisée dans chaque sens : le mélange au hasard échoue presque toujours.
  const ids = Array.from({ length: 14 }, (_, i) => `p${i}`);
  const exclusions: SantaExclusion[] = [];
  for (let i = 0; i < ids.length; i++) {
    for (let j = i + 2; j < ids.length; j++) {
      if (!(i === 0 && j === ids.length - 1)) exclusions.push([ids[i]!, ids[j]!]);
    }
  }
  assert.equal(isDrawFeasible(ids, exclusions), true);
  assertValidDraw(ids, exclusions);
});

test("tirage impossible ou trop petit : refus clair", () => {
  assert.throws(() => drawSecretSanta(["a", "b"], [], rnd), SantaError);
  const blocked: SantaExclusion[] = [["a", "b"], ["a", "c"]];
  assert.equal(isDrawFeasible(["a", "b", "c"], blocked), false);
  assert.throws(() => drawSecretSanta(["a", "b", "c"], blocked, rnd), /impossible/);
});

const NOW = new Date("2026-11-26T12:00:00Z");
const valid = { title: "Profs", revealAt: "2026-12-04T11:00:00Z", participantIds: ["a", "b", "c"] };

test("saisie : date future, trois personnes, exclusions nettoyées", () => {
  const input = parseSantaInput(
    { ...valid, participantIds: ["a", "b", "c", "c", 4], exclusions: [["a", "z"], ["a", "a"]] },
    NOW,
  );
  assert.deepEqual(input.participantIds, ["a", "b", "c"]);
  assert.deepEqual(input.exclusions, []);
  assert.throws(() => parseSantaInput({ ...valid, revealAt: "2026-11-26T11:59:00Z" }, NOW), /futur/);
  assert.throws(() => parseSantaInput({ ...valid, revealAt: "" }, NOW), /date/);
  assert.throws(() => parseSantaInput({ ...valid, participantIds: ["a", "b"] }, NOW), /3 participants/);
  assert.throws(() => parseSantaInput({ ...valid, title: "  " }, NOW), /titre/);
  assert.throws(() => parseSantaInput({ ...valid, exclusions: [["a", "b"]] }, NOW), /aucune paire/);
});

test("l'organisateur ne peut pas forcer des paires avec les exclusions", () => {
  const ids = ["a", "b", "c", "d", "e"];
  // Ne laisser qu'un cercle a-b-c-d-e : il suffirait de lire son nom pour connaître tous les autres.
  const circle: SantaExclusion[] = [["a", "c"], ["a", "d"], ["b", "d"], ["b", "e"], ["c", "e"]];
  assert.throws(() => parseSantaInput({ ...valid, participantIds: ids, exclusions: circle }, NOW), /reste secret/);
  const couples: SantaExclusion[] = [["a", "b"], ["c", "d"]];
  assert.equal(parseSantaInput({ ...valid, participantIds: ids, exclusions: couples }, NOW).exclusions.length, 2);
});

const santa: SantaRecord = {
  id: "s",
  organizerUserId: "org",
  organizerName: "Org",
  title: "T",
  budget: "",
  message: "",
  revealAt: new Date("2026-12-04T11:00:00Z"),
  drawnAt: new Date("2026-12-04T11:00:01Z"),
  exclusions: [],
};
const person = (userId: string, receiverUserId: string): SantaParticipantRecord => ({
  userId,
  displayName: `Nom ${userId}`,
  receiverUserId,
  invitedAt: null,
  openedAt: null,
});
const people = [person("a", "b"), person("b", "c"), person("c", "a")];
const after = new Date("2026-12-04T11:05:00Z");

test("chacun ne reçoit que sa propre paire", () => {
  const view = buildSantaView(santa, people, "a", after);
  assert.deepEqual(view.mine, { receiverName: "Nom b", alreadyOpened: false });
  assert.equal(view.organizer, null);
  assert.ok(!JSON.stringify(view).includes("receiverUserId"));
});

test("l'organisateur suit les ouvertures sans voir aucune paire", () => {
  const view = buildSantaView(santa, people, "org", after);
  assert.equal(view.mine, null);
  assert.equal(view.organizer?.status.length, 3);
  assert.ok(!JSON.stringify(view).includes("receiver"));
});

test("rien n'est révélé avant l'heure, même si la base contenait une paire", () => {
  assert.equal(buildSantaView(santa, people, "a", new Date("2026-12-04T10:59:59Z")).mine, null);
});

test("le module est ouvert au personnel, pas aux parents ni aux élèves", () => {
  for (const path of ["/secret-santa", "/api/secret-santa", "/api/secret-santa/x/ouvert"]) {
    assert.equal(canAccessIntranetPath(path, ["professeur"], false), true, path);
    assert.equal(canAccessIntranetPath(path, ["maintenance"], false), true, path);
    assert.equal(canAccessIntranetPath(path, ["parent"], false), false, path);
    assert.equal(canAccessIntranetPath(path, ["eleve"], false), false, path);
  }
});
