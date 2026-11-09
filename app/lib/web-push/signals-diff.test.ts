import assert from "node:assert/strict";
import { test } from "node:test";
import { diffSignals, safeRelativeHref } from "./signals-diff";

const sig = (id: string, count: number) => ({ id, label: id, count, href: "/x", detail: "" });

test("premier calcul : référence silencieuse", () => {
  const d = diffSignals(null, [sig("a", 3)]);
  assert.deepEqual(d.toNotify, []);
  assert.deepEqual(d.nextCounts, { a: 3 });
  assert.deepEqual(d.toDelete, []);
});

test("hausse notifiée, baisse et disparition enregistrées", () => {
  const d = diffSignals({ a: 1, b: 5, c: 2 }, [sig("a", 2), sig("b", 3)]);
  assert.deepEqual(d.toNotify.map((s) => s.id), ["a"]);
  assert.deepEqual(d.nextCounts, { a: 2, b: 3 });
  assert.deepEqual(d.toDelete, ["c"]);
});

test("aucune ligne stockée mais map vide : nouveau signal notifié", () => {
  assert.deepEqual(diffSignals({}, [sig("a", 1)]).toNotify.map((s) => s.id), ["a"]);
});

test("href relatif uniquement", () => {
  assert.equal(safeRelativeHref("/absences"), "/absences");
  assert.equal(safeRelativeHref("//evil.com"), "/");
  assert.equal(safeRelativeHref("https://evil.com"), "/");
});
