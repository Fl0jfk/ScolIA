import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  canAccessPersonnelModule,
  canManagePersonnel,
  canViewPersonnelDashboard,
} from "@/app/lib/personnel-types";

describe("personnel access flags", () => {
  it("direction voit le dashboard sans récursion", () => {
    assert.equal(canViewPersonnelDashboard(["direction_college"]), true);
    assert.equal(canManagePersonnel(["direction_college"]), false);
  });

  it("direction + professeur ne stack overflow pas", () => {
    const roles = ["direction_college", "professeur"];
    assert.equal(canViewPersonnelDashboard(roles), true);
    assert.equal(canManagePersonnel(roles), false);
    assert.equal(canAccessPersonnelModule(roles), true);
  });

  it("professeur seul : pas de dashboard RH, pas de récursion", () => {
    assert.equal(canViewPersonnelDashboard(["professeur"]), false);
    assert.equal(canManagePersonnel(["professeur"]), false);
    assert.equal(canAccessPersonnelModule(["professeur"]), false);
  });

  it("compta gère et voit le dashboard", () => {
    assert.equal(canManagePersonnel(["comptabilite"]), true);
    assert.equal(canViewPersonnelDashboard(["comptabilite"]), true);
  });
});
