import assert from "node:assert/strict";
import test from "node:test";
import { canConsultAbsencesHubTab } from "@/app/lib/vs-absences-hub-access";

test("canConsultAbsencesHubTab — direction sans matrice modules", () => {
  assert.equal(
    canConsultAbsencesHubTab({
      isOrgAdmin: false,
      accessibleModuleIds: null,
      roles: ["direction"],
    }),
    true,
  );
  assert.equal(
    canConsultAbsencesHubTab({
      isOrgAdmin: false,
      accessibleModuleIds: null,
      roles: ["direction_college"],
    }),
    true,
  );
});

test("canConsultAbsencesHubTab — prof seul sans module consultation", () => {
  assert.equal(
    canConsultAbsencesHubTab({
      isOrgAdmin: false,
      accessibleModuleIds: new Set(["vs-appels"]),
      roles: ["professeur"],
    }),
    false,
  );
});

test("canConsultAbsencesHubTab — module vs-absences suffit", () => {
  assert.equal(
    canConsultAbsencesHubTab({
      isOrgAdmin: false,
      accessibleModuleIds: new Set(["vs-absences"]),
      roles: ["direction"],
    }),
    true,
  );
});
