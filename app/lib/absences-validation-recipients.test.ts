import { test } from "node:test";
import assert from "node:assert/strict";
import type { Establishment, NotificationsConfig } from "./app-config-schemas";
import type { AbsenceRecord } from "./absences-types";
import { collectAbsenceValidationEmails } from "./absences-validation-recipients";

const establishments: Establishment[] = [
  { id: "ecole", label: "École Notre-Dame", kind: "ecole", active: true },
  { id: "college", label: "Collège Notre-Dame", kind: "college", active: true },
  { id: "lycee", label: "Lycée Notre-Dame", kind: "lycee", active: true },
];

const notifications: NotificationsConfig = {
  travelsCompta: [],
  absencesNotifyOgecCompta: ["compta@etab.fr"],
  absencesNotifyProfEcole: { label: "Secrétariat école", email: "pauline.leblond@ac-normandie.fr" },
  absencesNotifyProfCollege: { label: "Sarah Villiers", email: "sarah.buno@ac-normandie.fr" },
  absencesNotifyProfLycee: { label: "Sarah Villiers", email: "sarah.buno@ac-normandie.fr" },
  absencesNotifyProfCollegeLycee: { label: "Sarah Villiers", email: "sarah.buno@ac-normandie.fr" },
};

function profRecord(etablissement: string): Pick<AbsenceRecord, "data" | "createdBy"> {
  return {
    createdBy: { userId: "u1", name: "Paul Martin", email: "paul@etab.fr", roles: ["enseignant"] },
    data: {
      scope: "professeur",
      etablissement,
      startDate: "2026-08-30",
      endDate: "2026-08-30",
      startAt: "2026-08-30T06:00:00.000Z",
      endAt: "2026-08-30T16:00:00.000Z",
      reason: "Maladie",
      details: "Déclaré à l’accueil (standard).",
    },
  };
}

test("prof collège / lycée → personne qui déclare au rectorat", () => {
  assert.deepEqual(collectAbsenceValidationEmails(profRecord("Collège Notre-Dame"), notifications, establishments), [
    "sarah.buno@ac-normandie.fr",
  ]);
  assert.deepEqual(collectAbsenceValidationEmails(profRecord("Lycée Notre-Dame"), notifications, establishments), [
    "sarah.buno@ac-normandie.fr",
  ]);
  assert.deepEqual(collectAbsenceValidationEmails(profRecord("college"), notifications, establishments), [
    "sarah.buno@ac-normandie.fr",
  ]);
});

test("prof école → secrétariat école (ONISE)", () => {
  assert.deepEqual(collectAbsenceValidationEmails(profRecord("École Notre-Dame"), notifications, establishments), [
    "pauline.leblond@ac-normandie.fr",
  ]);
});

test("cycle inconnu → toutes les personnes absences professeurs configurées", () => {
  const emails = collectAbsenceValidationEmails(profRecord("Site inconnu"), notifications, establishments);
  assert.equal(emails.includes("pauline.leblond@ac-normandie.fr"), true);
  assert.equal(emails.includes("sarah.buno@ac-normandie.fr"), true);
});

test("cycle collège sans destinataire dédié → repli college/lycée", () => {
  const n: NotificationsConfig = {
    ...notifications,
    absencesNotifyProfCollege: undefined,
    absencesNotifyProfLycee: undefined,
  };
  assert.deepEqual(collectAbsenceValidationEmails(profRecord("Collège Notre-Dame"), n, establishments), [
    "sarah.buno@ac-normandie.fr",
  ]);
});

test("personnel OGEC → compta RH, pas le secrétariat rectorat", () => {
  const record: Pick<AbsenceRecord, "data" | "createdBy"> = {
    createdBy: { userId: "u2", name: "Claire", email: "claire@etab.fr", roles: ["administratif"] },
    data: {
      scope: "ogec",
      etablissement: null,
      startDate: "2026-08-30",
      endDate: "2026-08-30",
      startAt: "2026-08-30T06:00:00.000Z",
      endAt: "2026-08-30T16:00:00.000Z",
      reason: "RDV",
      details: "",
    },
  };
  assert.deepEqual(collectAbsenceValidationEmails(record, notifications, establishments), ["compta@etab.fr"]);
});
