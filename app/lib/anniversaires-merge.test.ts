import assert from "node:assert/strict";
import test from "node:test";
import { findFicheForAccount, isFicheDuplicateByName, personNameKey } from "@/app/lib/anniversaires-merge";

test("personNameKey: casse, accents, tirets", () => {
  assert.equal(personNameKey("Jean-Pierre", "DUPONT"), personNameKey("jean pierre", "Dupont"));
  assert.equal(personNameKey("Éloïse", "Martin"), personNameKey("eloise", "martin"));
  assert.notEqual(personNameKey("Paul", "Durand"), personNameKey("Paul", "Dupont"));
});

test("isFicheDuplicateByName", () => {
  const names = new Set([personNameKey("Anne", "Leroy")]);
  assert.equal(isFicheDuplicateByName(personNameKey("ANNE", "leroy"), names), true);
  assert.equal(isFicheDuplicateByName(personNameKey("Paul", "Durand"), names), false);
  assert.equal(isFicheDuplicateByName("", names), false);
});

test("findFicheForAccount: clés puis repli par nom actif unique", () => {
  const f1 = { active: true, keys: ["mail:a@x.fr"], nameKey: personNameKey("Anne", "Leroy"), jobTitle: "Prof" };
  const f2 = { active: true, keys: [], nameKey: personNameKey("Paul", "Durand"), jobTitle: "CPE" };
  const f3 = { active: true, keys: [], nameKey: personNameKey("Paul", "Durand"), jobTitle: "X" };
  const f4 = { active: false, keys: [], nameKey: personNameKey("Zoé", "Old"), jobTitle: "Y" };
  assert.equal(findFicheForAccount({ keys: ["mail:a@x.fr"], nameKey: "zzz" }, [f1, f2]), f1);
  assert.equal(findFicheForAccount({ keys: [], nameKey: personNameKey("anne", "leroy") }, [f1, f2]), f1);
  assert.equal(findFicheForAccount({ keys: [], nameKey: personNameKey("Paul", "Durand") }, [f2]), f2);
  assert.equal(findFicheForAccount({ keys: [], nameKey: personNameKey("Paul", "Durand") }, [f2, f3]), undefined);
  assert.equal(findFicheForAccount({ keys: [], nameKey: personNameKey("Zoé", "Old") }, [f4]), undefined);
});
