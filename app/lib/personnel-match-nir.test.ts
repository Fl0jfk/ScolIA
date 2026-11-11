import assert from "node:assert/strict";
import test from "node:test";
import { maskNirForAi } from "@/app/lib/personnel-match";

test("NJ-45 : le NIR est masqué avant l'IA", () => {
  assert.equal(maskNirForAi("NIR 1 85 05 78 006 084 36 fin").includes("1 85 05"), false);
  assert.equal(maskNirForAi("185057800608436").includes("185057800608436"), false);
  assert.equal(maskNirForAi("Dupont Jean 2024"), "Dupont Jean 2024");
});
