import assert from "node:assert/strict";
import { test } from "node:test";
import { resolveHomeMainTasks } from "@/app/lib/home-main-tasks";

test("direction — Absences RH en tête de Pour vous, lien file à valider", () => {
  const tasks = resolveHomeMainTasks({
    roles: ["direction_lycee"],
    accessibleModuleIds: new Set(["absences", "travels", "eleve-dossier", "rh"]),
  });
  assert.ok(tasks.length >= 1);
  assert.equal(tasks[0]!.id, "task-absences-rh-dir");
  assert.equal(tasks[0]!.label, "Absences RH");
  assert.match(tasks[0]!.href, /view=a-traiter/);
  assert.ok(tasks.some((t) => t.id === "task-travels-dir"));
});

test("direction_ecole / college — même raccourci Absences RH", () => {
  for (const role of ["direction_ecole", "direction_college", "direction"] as const) {
    const tasks = resolveHomeMainTasks({
      roles: [role],
      accessibleModuleIds: new Set(["absences", "travels", "eleve-dossier"]),
    });
    const abs = tasks.find((t) => t.moduleId === "absences");
    assert.ok(abs, `attendu Absences RH pour ${role}`);
    assert.match(abs!.href, /view=a-traiter/);
  }
});

test("compta / rh — Absences RH vers traitement (sans doublon direction)", () => {
  const tasks = resolveHomeMainTasks({
    roles: ["comptabilite"],
    accessibleModuleIds: new Set(["absences"]),
  });
  assert.equal(tasks.length, 1);
  assert.equal(tasks[0]!.id, "task-absences-rh");
  assert.match(tasks[0]!.href, /view=traitement/);
});

test("direction + compta — une seule tuile Absences (file direction)", () => {
  const tasks = resolveHomeMainTasks({
    roles: ["direction_college", "comptabilite"],
    accessibleModuleIds: new Set(["absences", "travels", "eleve-dossier"]),
  });
  const absTasks = tasks.filter((t) => t.moduleId === "absences");
  assert.equal(absTasks.length, 1);
  assert.equal(absTasks[0]!.id, "task-absences-rh-dir");
  assert.match(absTasks[0]!.href, /view=a-traiter/);
});

test("sans module absences accessible — pas de tuile", () => {
  const tasks = resolveHomeMainTasks({
    roles: ["direction_lycee"],
    accessibleModuleIds: new Set(["travels", "eleve-dossier"]),
  });
  assert.ok(!tasks.some((t) => t.moduleId === "absences"));
});
