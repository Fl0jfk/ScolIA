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

test("professeur — Pour vous : salles, photocopies, absence, HSE, sorties", () => {
  const tasks = resolveHomeMainTasks({
    roles: ["professeur"],
    accessibleModuleIds: new Set([
      "prof-room",
      "photocopies-couleur",
      "absences",
      "demandes-hse",
      "rh",
      "travels",
      "mon-planning",
    ]),
  });
  assert.equal(tasks[0]!.id, "task-prof-room");
  assert.ok(tasks.some((t) => t.id === "task-photocopies"));
  const absence = tasks.find((t) => t.id === "task-absences-prof");
  assert.ok(absence);
  assert.equal(absence!.label, "Demande d'absence");
  assert.match(absence!.href, /view=se-declarer/);
  assert.match(absence!.href, /#nouvelle-absence/);
  const hse = tasks.find((t) => t.id === "task-hse");
  assert.ok(hse);
  assert.match(hse!.href, /section=hse/);
  assert.ok(tasks.some((t) => t.id === "task-travels-prof"));
});

test("professeur — absence + HSE via hub RH si modules absents de la liste", () => {
  const tasks = resolveHomeMainTasks({
    roles: ["professeur"],
    accessibleModuleIds: new Set(["prof-room", "rh", "travels"]),
  });
  const absence = tasks.find((t) => t.id === "task-absences-prof");
  assert.ok(absence, "Demande d'absence doit remonter dès que rh est accessible");
  assert.match(absence!.href, /view=se-declarer/);
  const hse = tasks.find((t) => t.id === "task-hse");
  assert.ok(hse, "HSE doit remonter dès que rh est accessible");
  assert.match(hse!.href, /section=hse/);
});

test("professeur — pas de tuile si module non accessible", () => {
  const tasks = resolveHomeMainTasks({
    roles: ["professeur"],
    accessibleModuleIds: new Set(["prof-room"]),
  });
  assert.equal(tasks.length, 1);
  assert.equal(tasks[0]!.id, "task-prof-room");
  assert.ok(!tasks.some((t) => t.moduleId === "photocopies-couleur"));
  assert.ok(!tasks.some((t) => t.moduleId === "absences"));
  assert.ok(!tasks.some((t) => t.moduleId === "demandes-hse"));
  assert.ok(!tasks.some((t) => t.moduleId === "travels"));
});

test("direction + professeur — Absences RH direction prioritaire (pas de doublon demande)", () => {
  const tasks = resolveHomeMainTasks({
    roles: ["direction_college", "professeur"],
    accessibleModuleIds: new Set(["absences", "travels", "eleve-dossier", "prof-room", "rh"]),
  });
  const absTasks = tasks.filter((t) => t.moduleId === "absences");
  assert.equal(absTasks.length, 1);
  assert.equal(absTasks[0]!.id, "task-absences-rh-dir");
  assert.match(absTasks[0]!.href, /view=a-traiter/);
});
