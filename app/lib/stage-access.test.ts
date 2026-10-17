import assert from "node:assert/strict";
import test from "node:test";
import {
  canFileConventionToOneDrive,
  canManageStageSettings,
  canReviewPreconvention,
  resolveStageViewerRole,
} from "@/app/lib/stage-access";

test("canReviewPreconvention — administratif et directions", () => {
  assert.equal(canReviewPreconvention(["administratif"]), true);
  assert.equal(canReviewPreconvention(["direction"]), true);
  assert.equal(canReviewPreconvention(["direction_college"]), true);
  assert.equal(canReviewPreconvention(["direction_lycee"]), true);
  assert.equal(canReviewPreconvention(["direction_ecole"]), true);
});

test("canReviewPreconvention — admin établissement (souvent le seul rôle du compte org)", () => {
  assert.equal(canReviewPreconvention(["admin"]), true);
  assert.equal(canReviewPreconvention(["master"]), true);
  assert.equal(canReviewPreconvention(["admin", "professeur"]), true);
});

test("canReviewPreconvention — rôles consultation seule refusés", () => {
  assert.equal(canReviewPreconvention(["professeur"]), false);
  assert.equal(canReviewPreconvention(["cpe"]), false);
  assert.equal(canReviewPreconvention(["surveillant"]), false);
  assert.equal(canReviewPreconvention(["accueil"]), false);
  assert.equal(canReviewPreconvention(["parent"]), false);
  assert.equal(canReviewPreconvention([]), false);
});

test("resolveStageViewerRole — admin ouvre le hub comme administratif", () => {
  assert.equal(resolveStageViewerRole(["admin"]), "administratif");
  assert.equal(resolveStageViewerRole(["master"]), "administratif");
  assert.equal(resolveStageViewerRole(["administratif"]), "administratif");
  assert.equal(resolveStageViewerRole(["direction_lycee"]), "direction");
  assert.equal(resolveStageViewerRole(["professeur"]), "professeur");
  assert.equal(resolveStageViewerRole([]), null);
});

test("canFileConventionToOneDrive et réglages — admin inclus", () => {
  assert.equal(canFileConventionToOneDrive(["admin"]), true);
  assert.equal(canManageStageSettings(["admin"]), true);
  assert.equal(canManageStageSettings(["professeur"]), false);
  assert.equal(canFileConventionToOneDrive(["cpe"]), false);
});
