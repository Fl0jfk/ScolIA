import assert from "node:assert/strict";
import { test } from "node:test";
import type { Establishment, NotificationsConfig } from "@/app/lib/app-config-schemas";
import {
  resolveAbsenceValidationQueue,
  viewerCanValidateAbsence,
} from "@/app/lib/absences-routing";
import type { AbsenceRecord } from "@/app/lib/absences-types";

const establishments: Establishment[] = [
  {
    id: "ecole",
    label: "École",
    kind: "ecole",
    active: true,
    directorName: "Mme Plantec",
    directorEmail: "plantec@etab.fr",
    directorExternalUserId: "u-plantec",
  },
  {
    id: "lycee",
    label: "Lycée",
    kind: "lycee",
    active: true,
    directorName: "Mme Dona",
    directorEmail: "0761713z@ac-normandie.fr",
    directorExternalUserId: "u-dona-biz",
  },
];

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

function ogec(over?: Partial<AbsenceRecord["data"]>): AbsenceRecord {
  return {
    id: "a1",
    createdAt: "",
    updatedAt: "",
    source: "self",
    displayName: "Agent",
    calendarVisible: false,
    createdBy: {
      userId: "u-agent",
      name: "Agent",
      email: "agent@etab.fr",
      roles: ["administratif"],
    },
    data: {
      scope: "ogec",
      etablissement: null,
      startDate: "2026-10-08",
      endDate: "2026-10-08",
      startAt: "",
      endAt: "",
      reason: "RDV",
      details: "",
      ...over,
    },
    workflowStatus: "OUVERTE",
    managerDecision: "EN_ATTENTE",
    history: [],
  };
}

function prof(etab: string): AbsenceRecord {
  return {
    ...ogec(),
    id: "p1",
    createdBy: {
      userId: "u-prof",
      name: "Prof",
      email: "prof@etab.fr",
      roles: ["professeur"],
    },
    data: {
      scope: "professeur",
      etablissement: etab,
      startDate: "2026-10-08",
      endDate: "2026-10-08",
      startAt: "",
      endAt: "",
      reason: "Formation",
      details: "",
    },
  };
}

test("OGEC défaut → absencesValidatorsOgec (config), pas UAI hardcodé", () => {
  const queue = resolveAbsenceValidationQueue(ogec(), notifications, establishments);
  assert.equal(queue.length, 1);
  assert.equal(queue[0]!.email, "anne-marie.dona@etab.fr");
});

test("OGEC nominatif fiche → uniquement cette personne", () => {
  const queue = resolveAbsenceValidationQueue(
    ogec({
      ogecValidator: { email: "plantec@etab.fr", userId: "u-plantec", label: "Plantec" },
    }),
    notifications,
    establishments,
  );
  assert.equal(queue[0]!.email, "plantec@etab.fr");
  assert.equal(
    viewerCanValidateAbsence(
      ogec({
        ogecValidator: { email: "plantec@etab.fr", userId: "u-plantec" },
      }),
      ["direction_lycee"],
      {
        establishments,
        notifications,
        email: "anne-marie.dona@etab.fr",
        userId: "u-dona-biz",
      },
    ),
    false,
  );
  assert.equal(
    viewerCanValidateAbsence(
      ogec({
        ogecValidator: { email: "plantec@etab.fr", userId: "u-plantec" },
      }),
      ["direction_ecole"],
      {
        establishments,
        notifications,
        email: "plantec@etab.fr",
        userId: "u-plantec",
      },
    ),
    true,
  );
});

test("directrice lycée (config) valide OGEC défaut + prof lycée", () => {
  const ctx = {
    establishments,
    notifications,
    email: "anne-marie.dona@etab.fr",
    userId: "auth-dona",
    userIds: ["auth-dona", "u-dona-biz"],
  };
  assert.equal(viewerCanValidateAbsence(ogec(), ["direction_lycee"], ctx), true);
  assert.equal(viewerCanValidateAbsence(prof("Lycée"), ["direction_lycee"], ctx), true);
  assert.equal(viewerCanValidateAbsence(prof("École"), ["direction_lycee"], ctx), false);
});

test("liste vide OGEC → repli directeur établissement lycée (Établissements)", () => {
  const empty: NotificationsConfig = {
    travelsCompta: [],
    absencesNotifyOgecCompta: [],
    absencesValidatorsOgec: [],
  };
  const queue = resolveAbsenceValidationQueue(ogec(), empty, establishments);
  assert.equal(queue[0]!.email, "0761713z@ac-normandie.fr");
  assert.equal(
    viewerCanValidateAbsence(ogec(), ["direction_lycee"], {
      establishments,
      notifications: empty,
      email: "autre@etab.fr",
      userId: "x",
    }),
    true,
  );
});

test("prof école → absencesValidatorsProfEcole", () => {
  const queue = resolveAbsenceValidationQueue(prof("École"), notifications, establishments);
  assert.equal(queue[0]!.email, "plantec@etab.fr");
});
