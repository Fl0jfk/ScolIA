import assert from "node:assert/strict";
import { test } from "node:test";
import type { Establishment, NotificationsConfig } from "@/app/lib/app-config-schemas";
import {
  describeAbsenceProcessorBasket,
  describeAbsenceValidationSuivi,
  findLastValidationHistory,
} from "@/app/lib/absences-suivi";

const establishments: Establishment[] = [
  {
    id: "lycee",
    label: "Lycée",
    kind: "lycee",
    active: true,
    directorEmail: "dona@etab.fr",
  },
];

const notifications: NotificationsConfig = {
  travelsCompta: [],
  absencesNotifyOgecCompta: ["rh@etab.fr", "compta@etab.fr"],
  absencesNotifyProfLycee: {
    label: "Sarah Buno",
    email: "sarah.buno@etab.fr",
    userId: "u-sarah",
  },
};

test("findLastValidationHistory prend la dernière DECISION_VALIDEE", () => {
  const hit = findLastValidationHistory([
    { at: "2026-10-01T08:00:00.000Z", by: "A", action: "CREATED" },
    { at: "2026-10-02T09:00:00.000Z", by: "Mme Dona", action: "DECISION_VALIDEE" },
    { at: "2026-10-02T10:00:00.000Z", by: "RH", action: "TRAITEMENT_ADMIN" },
  ]);
  assert.equal(hit?.by, "Mme Dona");
});

test("validation discrétionnaire : qui + quand", () => {
  const suivi = describeAbsenceValidationSuivi({
    managerDecision: "VALIDEE",
    hoursTreatment: "DECLARATION_RECTORAT",
    history: [
      {
        at: "2026-10-08T10:30:00.000Z",
        by: "Mme Dona",
        action: "DECISION_VALIDEE",
        note: "OK formation",
      },
    ],
    data: { scope: "professeur", etablissement: "Lycée" },
    createdBy: { roles: ["enseignant"] },
  });
  assert.ok(suivi);
  assert.equal(suivi!.priseActe, false);
  assert.match(suivi!.detail, /Mme Dona/);
  assert.match(suivi!.detail, /08\/10\/2026|10\/08\/2026/);
});

test("arrêt de travail : prise d’acte, pas validation discrétionnaire", () => {
  const suivi = describeAbsenceValidationSuivi({
    managerDecision: "VALIDEE",
    hoursTreatment: "MALADIE",
    history: [
      {
        at: "2026-10-08T11:00:00.000Z",
        by: "Mme Dona",
        action: "DECISION_VALIDEE",
        note: "Prise d'acte direction — arrêt de travail. Dossier transmis pour traitement administratif.",
      },
    ],
    data: { scope: "ogec", etablissement: null },
    createdBy: { roles: ["administratif"] },
  });
  assert.ok(suivi);
  assert.equal(suivi!.priseActe, true);
  assert.match(suivi!.headline, /prise d’acte|Passage direct/i);
  assert.match(suivi!.detail, /Mme Dona/);
});

test("panier OGEC : RH / compta configurées", () => {
  const basket = describeAbsenceProcessorBasket(
    {
      managerDecision: "VALIDEE",
      hoursTreatment: "MALADIE",
      data: { scope: "ogec", etablissement: null },
      createdBy: { roles: ["administratif"] },
    },
    notifications,
    establishments,
  );
  assert.match(basket.basketLabel, /OGEC|RH/i);
  assert.deepEqual(basket.peopleLabels, ["rh@etab.fr", "compta@etab.fr"]);
});

test("panier prof lycée : Sarah", () => {
  const basket = describeAbsenceProcessorBasket(
    {
      managerDecision: "VALIDEE",
      hoursTreatment: "DECLARATION_RECTORAT",
      data: { scope: "professeur", etablissement: "Lycée" },
      createdBy: { roles: ["enseignant"] },
    },
    notifications,
    establishments,
  );
  assert.match(basket.basketLabel, /lycée/i);
  assert.deepEqual(basket.peopleLabels, ["Sarah Buno"]);
});
