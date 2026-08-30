import { test } from "node:test";
import assert from "node:assert/strict";
import type { Establishment, NotificationsConfig } from "./app-config-schemas";
import type { AbsenceRecord } from "./absences-types";
import {
  isAbsencePendingForProcessor,
  processorMayAccessValidatedAbsence,
  viewerCanSeeProcessorQueue,
  viewerIsAbsenceProcessor,
} from "./absences-admin-access";

const establishments: Establishment[] = [
  { id: "ecole", label: "École", kind: "ecole", active: true },
  { id: "college", label: "Collège", kind: "college", active: true },
  { id: "lycee", label: "Lycée", kind: "lycee", active: true },
];

const notifications: NotificationsConfig = {
  travelsCompta: [],
  absencesNotifyOgecCompta: ["rh@etab.fr"],
  absencesNotifyProfEcole: {
    label: "Pauline",
    email: "pauline.leblond@ac-normandie.fr",
    userId: "u-pauline",
  },
  absencesNotifyProfCollege: { label: "Sarah", email: "sarah.buno@ac-normandie.fr", userId: "u-sarah" },
  absencesNotifyProfLycee: { label: "Sarah", email: "sarah.buno@ac-normandie.fr", userId: "u-sarah" },
};

function prof(etablissement: string): Pick<AbsenceRecord, "data" | "createdBy" | "managerDecision" | "workflowStatus" | "source"> {
  return {
    source: "accueil",
    managerDecision: "VALIDEE",
    workflowStatus: "A_TRAITER",
    createdBy: { userId: "p1", name: "Paul Martin", email: "paul@etab.fr", roles: ["enseignant"] },
    data: {
      scope: "professeur",
      etablissement,
      startDate: "2026-08-30",
      endDate: "2026-08-30",
      startAt: "",
      endAt: "",
      reason: "Maladie",
      details: "",
    },
  };
}

test("Sarah traite le lycée, pas l’école", () => {
  const sarah = { email: "sarah.buno@ac-normandie.fr", userId: "u-sarah", roles: ["administratif"] };
  assert.equal(
    viewerIsAbsenceProcessor(prof("Lycée"), sarah, notifications, establishments),
    true,
  );
  assert.equal(
    viewerIsAbsenceProcessor(prof("École"), sarah, notifications, establishments),
    false,
  );
});

test("Pauline est reconnue par userId", () => {
  assert.equal(
    viewerIsAbsenceProcessor(prof("École"), { userId: "u-pauline", roles: [] }, notifications, establishments),
    true,
  );
});

test("file processeur : validée et non clôturée", () => {
  const row = prof("Lycée") as AbsenceRecord;
  assert.equal(isAbsencePendingForProcessor(row), true);
  assert.equal(isAbsencePendingForProcessor({ ...row, workflowStatus: "CLOTUREE" }), false);
  assert.equal(isAbsencePendingForProcessor({ ...row, managerDecision: "EN_ATTENTE" }), false);
});

test("ingest calendrier n’entre pas dans la file rectorat", () => {
  const row = { ...prof("Lycée"), source: "admin_manual" as const } as AbsenceRecord;
  assert.equal(isAbsencePendingForProcessor(row), false);
});

test("RH OGEC par e-mail", () => {
  const record: Pick<AbsenceRecord, "data" | "createdBy"> = {
    createdBy: { userId: "c1", name: "Claire", email: "claire@etab.fr", roles: ["administratif"] },
    data: {
      scope: "ogec",
      etablissement: null,
      startDate: "2026-08-30",
      endDate: "2026-08-30",
      startAt: "",
      endAt: "",
      reason: "RDV",
      details: "",
    },
  };
  assert.equal(
    viewerIsAbsenceProcessor(record, { email: "rh@etab.fr", roles: [] }, notifications, establishments),
    true,
  );
  assert.equal(
    viewerIsAbsenceProcessor(record, { email: "sarah.buno@ac-normandie.fr", roles: [] }, notifications, establishments),
    false,
  );
});

test("Sarah voit l’onglet traitement", () => {
  assert.equal(
    viewerCanSeeProcessorQueue({ email: "sarah.buno@ac-normandie.fr" }, notifications),
    true,
  );
  assert.equal(viewerCanSeeProcessorQueue({ email: "autre@etab.fr" }, notifications), false);
  assert.equal(viewerCanSeeProcessorQueue({ email: "autre@etab.fr", roles: ["admin"] }, notifications), true);
});

test("Sarah ne voit pas un dossier encore en attente direction", () => {
  const pending = {
    ...prof("Lycée"),
    managerDecision: "EN_ATTENTE" as const,
    workflowStatus: "OUVERTE" as const,
  };
  const sarah = { email: "sarah.buno@ac-normandie.fr", userId: "u-sarah", roles: ["administratif"] };
  assert.equal(
    processorMayAccessValidatedAbsence(pending as AbsenceRecord, sarah, notifications, establishments),
    false,
  );
  assert.equal(
    processorMayAccessValidatedAbsence(prof("Lycée") as AbsenceRecord, sarah, notifications, establishments),
    true,
  );
});
