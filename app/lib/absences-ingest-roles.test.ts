import assert from "node:assert/strict";
import test from "node:test";
import { canAdminIngest } from "@/app/lib/absences-types";

test("NJ-57 : surveillants et CPE n'importent pas d'absences validées", () => {
  assert.equal(canAdminIngest(["surveillant"]), false);
  assert.equal(canAdminIngest(["cpe"]), false);
  assert.equal(canAdminIngest(["administratif"]), true);
  assert.equal(canAdminIngest(["direction_college"]), true);
});
