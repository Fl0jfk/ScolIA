/**
 * Preuve : avec paramétrage Validation clair, la directrice lycée voit
 * OGEC défaut + profs lycée ; pas les nominatifs ailleurs ni les autres cycles.
 */
import assert from "node:assert/strict";
import { test } from "node:test";
import type { Establishment, NotificationsConfig } from "@/app/lib/app-config-schemas";
import {
  canManageAbsence,
  isAbsencePendingForManager,
  type AbsenceRecord,
} from "@/app/lib/absences-types";

const establishments: Establishment[] = [
  {
    id: "ecole",
    label: "École",
    kind: "ecole",
    active: true,
    directorEmail: "plantec@etab.fr",
    directorExternalUserId: "u-plantec",
  },
  {
    id: "lycee",
    label: "Lycée",
    kind: "lycee",
    active: true,
    directorEmail: "0761713z@ac-normandie.fr",
    directorExternalUserId: "u-dona-biz",
  },
];

/** Paramétrage réaliste : validates = comptes perso (pas UAI). */
const notifications: NotificationsConfig = {
  travelsCompta: [],
  absencesNotifyOgecCompta: ["rh@etab.fr"],
  absencesValidatorsOgec: [
    { label: "Mme Dona", email: "anne-marie.dona@etab.fr", userId: "u-dona-biz" },
  ],
  absencesValidatorsProfLycee: [
    { label: "Mme Dona", email: "anne-marie.dona@etab.fr", userId: "u-dona-biz" },
  ],
  absencesValidatorsProfEcole: [
    { label: "Mme Plantec", email: "plantec@etab.fr", userId: "u-plantec" },
  ],
};

function abs(partial: {
  id: string;
  displayName: string;
  createdBy?: AbsenceRecord["createdBy"];
  data: AbsenceRecord["data"];
}): AbsenceRecord {
  return {
    id: partial.id,
    createdAt: "2026-10-01T10:00:00.000Z",
    updatedAt: "2026-10-01T10:00:00.000Z",
    source: "self",
    displayName: partial.displayName,
    calendarVisible: false,
    createdBy: partial.createdBy || {
      userId: "u-agent",
      name: "Agent",
      email: "agent@etab.fr",
      roles: ["administratif"],
    },
    data: partial.data,
    workflowStatus: "OUVERTE",
    managerDecision: "EN_ATTENTE",
    history: [],
  };
}

function fivePending(): AbsenceRecord[] {
  return [
    abs({
      id: "ogec-1",
      displayName: "OGEC Dupont",
      data: {
        scope: "ogec",
        etablissement: null,
        ogecValidator: {
          email: "anne-marie.dona@etab.fr",
          userId: "u-dona-biz",
        },
        startDate: "2026-10-08",
        endDate: "2026-10-08",
        startAt: "",
        endAt: "",
        reason: "RDV",
        details: "",
      },
    }),
    abs({
      id: "ogec-2",
      displayName: "OGEC Martin",
      data: {
        scope: "ogec",
        etablissement: null,
        startDate: "2026-10-08",
        endDate: "2026-10-08",
        startAt: "",
        endAt: "",
        reason: "Maladie",
        details: "",
      },
    }),
    abs({
      id: "ogec-nominatif",
      displayName: "Colas",
      data: {
        scope: "ogec",
        etablissement: null,
        ogecValidator: { email: "plantec@etab.fr", userId: "u-plantec" },
        startDate: "2026-10-08",
        endDate: "2026-10-08",
        startAt: "",
        endAt: "",
        reason: "Enfant malade",
        details: "",
      },
    }),
    abs({
      id: "prof-lycee",
      displayName: "Prof Lycée",
      createdBy: {
        userId: "u-prof-l",
        name: "Prof L",
        email: "prof.l@etab.fr",
        roles: ["professeur"],
      },
      data: {
        scope: "professeur",
        etablissement: "Lycée",
        startDate: "2026-10-08",
        endDate: "2026-10-08",
        startAt: "",
        endAt: "",
        reason: "Formation",
        details: "",
      },
    }),
    abs({
      id: "prof-ecole",
      displayName: "Prof École",
      createdBy: {
        userId: "u-prof-e",
        name: "Prof E",
        email: "prof.e@etab.fr",
        roles: ["professeur"],
      },
      data: {
        scope: "professeur",
        etablissement: "École",
        startDate: "2026-10-08",
        endDate: "2026-10-08",
        startAt: "",
        endAt: "",
        reason: "Formation",
        details: "",
      },
    }),
  ];
}

const dirCtx = {
  establishments,
  notifications,
  userId: "auth-dona",
  userIds: ["auth-dona", "u-dona-biz"],
  email: "anne-marie.dona@etab.fr",
};

test("admin voit les 5 ; direction_lycee (config) en voit 3", () => {
  const all = fivePending();
  assert.equal(all.length, 5);
  const admin = all.filter((a) =>
    isAbsencePendingForManager(a, "admin", ["admin"], {
      establishments,
      notifications,
      userId: "admin",
    }),
  );
  assert.equal(admin.length, 5);

  const pending = all.filter((a) =>
    isAbsencePendingForManager(a, "auth-dona", ["direction_lycee"], dirCtx),
  );
  const ids = pending.map((a) => a.id).sort();
  assert.deepEqual(ids, ["ogec-1", "ogec-2", "prof-lycee"].sort());
  assert.ok(!ids.includes("ogec-nominatif"));
  assert.ok(!ids.includes("prof-ecole"));
});

test("match e-mail config suffit même sans rôle direction_lycee", () => {
  assert.equal(
    canManageAbsence(fivePending()[0]!, [], dirCtx),
    true,
  );
});
