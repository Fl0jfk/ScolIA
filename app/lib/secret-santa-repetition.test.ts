import assert from "node:assert/strict";
import { randomInt } from "node:crypto";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import type { SantaExclusion } from "@/app/lib/secret-santa";
import { REHEARSAL_CHECKS, breaksRules, rehearseDraw } from "@/app/lib/secret-santa-repetition";

const rnd = (max: number) => randomInt(max);
const twenty = Array.from({ length: 20 }, (_, i) => `p${i}`);
const rules: SantaExclusion[] = [["p0", "p1"], ["p0", "p2"], ["p3", "p4"], ["p5", "absent"]];

test("le contrôle voit une séparation non respectée, dans les deux sens", () => {
  assert.equal(breaksRules([{ giver: "p1", receiver: "p0" }], rules), true);
  assert.equal(breaksRules([{ giver: "p0", receiver: "p1" }], rules), true);
  assert.equal(breaksRules([{ giver: "p0", receiver: "p3" }], rules), false);
});

test("répétition à vingt : mille vérifications, aucune séparation violée, absents ignorés", () => {
  const result = rehearseDraw(twenty, rules, rnd, () => 0);
  assert.ok(result.ok);
  assert.deepEqual([result.checked, result.violations, result.rules.length], [REHEARSAL_CHECKS, 0, 3]);
  assert.deepEqual(result.pairs.map((p) => p.receiver).sort(), [...twenty].sort());
  assert.equal(breaksRules(result.pairs, rules), false);
});

test("répétition bloquée : l'administrateur voit qui dépasse la limite", () => {
  const result = rehearseDraw(["p0", "p1", "p2", "p3"], rules, rnd);
  assert.deepEqual(result, { ok: false, reason: "blocked", maxPerPerson: 1, overLimit: [{ userId: "p0", count: 2 }] });
});

test("la répétition ne lit ni n'écrit aucun tirage", () => {
  const route = readFileSync("app/api/secret-santa/incompatibilites/repetition/route.ts", "utf8");
  assert.match(route, /requireSantaRulesAdmin\(\)/);
  assert.doesNotMatch(route, /readSantaFor|createSanta|getDb|sendSanta/);
});
