import { test } from "node:test";
import assert from "node:assert/strict";
import type { Establishment, NotificationsConfig } from "./app-config-schemas";
import {
  canManageAbsence,
  type AbsenceRecord,
} from "./absences-types";
import {
  defaultOgecValidatorsFromConfig,
  parsePersonnelAbsenceManager,
  resolveOgecValidatorsForAbsence,
  serializePersonnelAbsenceManager,
} from "./absences-ogec-validators-shared";

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
    directorEmail: "dona@etab.fr",
    directorExternalUserId: "u-dona",
  },
];

const notifications: NotificationsConfig = {
  travelsCompta: [],
  absencesNotifyOgecCompta: ["rh@etab.fr"],
};

function ogecAbs(validator?: { email: string; userId?: string; label?: string } | null): AbsenceRecord {
  return {
    id: "a1",
    createdAt: "",
    updatedAt: "",
    source: "accueil",
    displayName: "Séverine Colas",
    calendarVisible: false,
    createdBy: { userId: "u-colas", name: "Séverine Colas", email: "colas@etab.fr", roles: ["administratif"] },
    data: {
      scope: "ogec",
      etablissement: null,
      ogecValidator: validator || null,
      startDate: "2026-09-24",
      endDate: "2026-09-24",
      startAt: "",
      endAt: "",
      reason: "Maladie",
      details: "",
    },
    workflowStatus: "OUVERTE",
    managerDecision: "EN_ATTENTE",
    history: [],
  };
}

test("défaut OGEC = direction du lycée (pas toute direction)", () => {
  const defaults = defaultOgecValidatorsFromConfig(notifications, establishments);
  assert.equal(defaults.length, 1);
  assert.equal(defaults[0]?.email, "dona@etab.fr");
  assert.equal(defaults[0]?.userId, "u-dona");
});

test("snapshot nominatif : seule Plantec peut valider", () => {
  const abs = ogecAbs({
    email: "plantec@etab.fr",
    userId: "u-plantec",
    label: "Mme Plantec",
  });
  assert.equal(
    canManageAbsence(abs, ["direction_ecole"], {
      establishments,
      notifications,
      userId: "u-plantec",
      email: "plantec@etab.fr",
    }),
    true,
  );
  assert.equal(
    canManageAbsence(abs, ["direction_lycee"], {
      establishments,
      notifications,
      userId: "u-dona",
      email: "dona@etab.fr",
    }),
    false,
  );
});

test("sans snapshot : directrice lycée valide, pas l’école", () => {
  const abs = ogecAbs(null);
  assert.equal(
    canManageAbsence(abs, ["direction_lycee"], {
      establishments,
      notifications,
      userId: "u-dona",
      email: "dona@etab.fr",
    }),
    true,
  );
  assert.equal(
    canManageAbsence(abs, ["direction_ecole"], {
      establishments,
      notifications,
      userId: "u-plantec",
      email: "plantec@etab.fr",
    }),
    false,
  );
});

test("serialize / parse managerId fiche RH", () => {
  const raw = serializePersonnelAbsenceManager({
    email: "plantec@etab.fr",
    userId: "u-plantec",
    label: "Mme Plantec",
  });
  assert.ok(raw);
  const parsed = parsePersonnelAbsenceManager(raw);
  assert.equal(parsed?.email, "plantec@etab.fr");
  assert.equal(parsed?.userId, "u-plantec");
  assert.equal(parsePersonnelAbsenceManager("plantec@etab.fr")?.email, "plantec@etab.fr");
});

test("resolveOgecValidatorsForAbsence priorise le snapshot", () => {
  const abs = ogecAbs({ email: "plantec@etab.fr", userId: "u-plantec" });
  const people = resolveOgecValidatorsForAbsence(abs, notifications, establishments);
  assert.equal(people[0]?.email, "plantec@etab.fr");
});
