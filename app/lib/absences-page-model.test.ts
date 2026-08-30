import { test } from "node:test";
import assert from "node:assert/strict";
import {
  canDepositJustificatif,
  isWaitingAdminTreatment,
  type AbsenceItem,
} from "./absences-page-model";

function item(over: Partial<AbsenceItem> & Pick<AbsenceItem, "workflowStatus" | "managerDecision">): AbsenceItem {
  return {
    id: "a1",
    createdAt: "2026-08-30T08:00:00.000Z",
    updatedAt: "2026-08-30T08:00:00.000Z",
    createdBy: { userId: "u1", name: "Paul", email: "paul@etab.fr", roles: ["enseignant"] },
    data: {
      scope: "professeur",
      etablissement: "Lycée",
      startDate: "2026-08-30",
      endDate: "2026-08-30",
      reason: "Maladie",
      details: "",
    },
    ...over,
  };
}

test("file RH : validée et non clôturée", () => {
  const row = item({ workflowStatus: "A_TRAITER", managerDecision: "VALIDEE" });
  assert.equal(isWaitingAdminTreatment(row), true);
  assert.equal(canDepositJustificatif(row), true);
});

test("clôturée : plus de dépôt ni file RH", () => {
  const row = item({ workflowStatus: "CLOTUREE", managerDecision: "VALIDEE" });
  assert.equal(isWaitingAdminTreatment(row), false);
  assert.equal(canDepositJustificatif(row), false);
});

test("refusée : pas de dépôt", () => {
  const row = item({ workflowStatus: "CLOTUREE", managerDecision: "REFUSEE" });
  assert.equal(canDepositJustificatif(row), false);
});

test("en attente direction : dépôt possible", () => {
  const row = item({ workflowStatus: "OUVERTE", managerDecision: "EN_ATTENTE" });
  assert.equal(canDepositJustificatif(row), true);
});
