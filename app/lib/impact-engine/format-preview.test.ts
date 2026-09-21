import { test } from "node:test";
import assert from "node:assert/strict";
import { formatImpactPreviewAlert, formatImpactPreviewLines } from "./format-preview";

test("formatImpactPreview — créneaux vidés + zéro planning", () => {
  const lines = formatImpactPreviewLines({
    status: "VALIDE",
    title: "Musée",
    participantCount: 24,
    participantLinkedCount: 24,
    wroteTeacherPlanningReplacement: false,
    creneauxVides: [{ id: "1" }, { id: "2" }],
    impacts: [
      {
        domaine: "resto",
        constat: "Paniers",
        tiroir: "C",
        question: "Les repas panier sont-ils bien commandés ?",
      },
    ],
  });
  assert.ok(lines.some((l) => /24 élève/.test(l)));
  assert.ok(lines.some((l) => /2 créneau/.test(l) && /pas de remplacement/.test(l)));
  assert.ok(lines.some((l) => /aucune écriture/.test(l)));
  assert.ok(lines.some((l) => /Question/.test(l) && /repas/.test(l)));
});

test("formatImpactPreviewAlert — vide si null", () => {
  assert.equal(formatImpactPreviewAlert(null), "");
  assert.equal(formatImpactPreviewAlert(undefined), "");
});
