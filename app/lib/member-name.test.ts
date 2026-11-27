import assert from "node:assert/strict";
import { test } from "node:test";
import { MEMBER_NAME_MAX, parseMemberName } from "@/app/lib/member-name";

test("sans prénom ni nom dans la demande, le nom n'est pas touché", () => {
  assert.deepEqual(parseMemberName(undefined, undefined), { ok: true, value: null });
});

test("prénom puis nom, espaces et caractères invisibles nettoyés, casse conservée", () => {
  assert.deepEqual(parseMemberName("  Marie​  Claire ", "de\tLA\nTour "), {
    ok: true,
    value: { firstName: "Marie Claire", lastName: "de LA Tour", displayName: "Marie Claire de LA Tour" },
  });
});

test("les deux champs sont obligatoires et limités en longueur", () => {
  for (const [first, last] of [["", "Dupont"], ["Marie", "  "], ["Marie", undefined], [12, "Dupont"]]) {
    assert.equal(parseMemberName(first, last).ok, false);
  }
  assert.equal(parseMemberName("a".repeat(MEMBER_NAME_MAX + 1), "b").ok, false);
});
