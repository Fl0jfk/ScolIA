import test from "node:test";
import assert from "node:assert/strict";
import { cleVerrouDocument } from "@/app/lib/ent-collection-db";

test("la clé du verrou par document ne contient pas d'octet nul (refusé par Postgres)", () => {
  const cle = cleVerrouDocument("etab", "internat__sorties-classe", "sc_abc12345");
  assert.equal(cle.includes("\0"), false);
  assert.notEqual(cle, cleVerrouDocument("etab", "internat__sorties-classe", "sc_abc12346"));
});
