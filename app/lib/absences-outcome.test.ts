import { test } from "node:test";
import assert from "node:assert/strict";
import { describeAbsenceOutcome } from "./absences-outcome";

const base = {
  data: { scope: "professeur", etablissement: "college" },
  createdBy: { roles: ["professeur"] },
};

test("validée puis clôturée automatiquement (rattrapage interne)", () => {
  const o = describeAbsenceOutcome({
    ...base,
    managerDecision: "VALIDEE",
    workflowStatus: "CLOTUREE",
    hoursTreatment: "RATTRAPAGE_INTERNE",
    adminTreatedBy: "Anne-Sophie DUMOUCHEL",
    adminTreatedAt: "2026-11-05T06:38:07.140Z",
    adminNote: "Clôture automatique — rattrapage interne, sans déclaration rectorat.",
    history: [
      { at: "2026-11-04T19:03:02.259Z", by: "Béatrice SERET", action: "CREATION" },
      { at: "2026-11-05T06:38:07.140Z", by: "Anne-Sophie DUMOUCHEL", action: "DECISION_VALIDEE" },
    ],
  });
  assert.equal(o.state, "validee");
  assert.equal(o.closed, true);
  assert.equal(o.shortLabel, "Validée · clôturée");
  assert.equal(o.decisionLine, "Validée par Anne-Sophie DUMOUCHEL le 05/11/2026 07:38");
  assert.equal(o.closureLine, "Dossier clôturé à la validation — rattrapage interne, sans déclaration rectorat.");
});

test("validée, clôturée plus tard par le traitement RH", () => {
  const o = describeAbsenceOutcome({
    ...base,
    managerDecision: "VALIDEE",
    workflowStatus: "CLOTUREE",
    adminTreatedBy: "Secrétariat",
    adminTreatedAt: "2026-11-07T09:00:00.000Z",
    adminNote: "Déclarée au rectorat",
    history: [{ at: "2026-11-05T06:38:07.140Z", by: "Directrice", action: "DECISION_VALIDEE" }],
  });
  assert.match(o.closureLine || "", /^Clôturée par Secrétariat le 07\/11\/2026 10:00 — Déclarée au rectorat$/);
});

test("validée, pas encore traitée", () => {
  const o = describeAbsenceOutcome({
    ...base,
    managerDecision: "VALIDEE",
    workflowStatus: "A_TRAITER",
    history: [{ at: "2026-11-05T06:38:07.140Z", by: "Directrice", action: "DECISION_VALIDEE" }],
  });
  assert.equal(o.closed, false);
  assert.equal(o.shortLabel, "Validée");
  assert.equal(o.closureLine, "En attente du traitement RH / rectorat");
});

test("refusée et en attente", () => {
  const r = describeAbsenceOutcome({
    ...base,
    managerDecision: "REFUSEE",
    workflowStatus: "CLOTUREE",
    history: [{ at: "2026-11-05T06:38:07.140Z", by: "Directrice", action: "DECISION_REFUSEE", note: "Conseil de classe" }],
  });
  assert.equal(r.state, "refusee");
  assert.equal(r.closureLine, "Motif : Conseil de classe");
  const p = describeAbsenceOutcome({ ...base, managerDecision: "EN_ATTENTE", workflowStatus: "OUVERTE", history: [] });
  assert.equal(p.state, "en_attente");
});
