import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  defaultAccompagnementDocumentTitle,
  detectAccompagnementKind,
  isAccompagnementDocumentTitle,
  isPapDocumentTitle,
} from "./eleve-pap";

describe("eleve-pap accompagnement", () => {
  it("détecte PAP / PAI / PPS / GEVASCO", () => {
    assert.equal(detectAccompagnementKind("PAP 2025-2026"), "pap");
    assert.equal(detectAccompagnementKind("PAI allergie"), "pai");
    assert.equal(detectAccompagnementKind("PPS MDPH"), "pps");
    assert.equal(detectAccompagnementKind("GEVASCO 2025-2026"), "gevasco");
    assert.equal(detectAccompagnementKind("Gevas-co élève"), "gevasco");
    assert.equal(detectAccompagnementKind("Plan d'accompagnement personnalisé"), "pap");
    assert.equal(detectAccompagnementKind("Projet d'accueil individualisé"), "pai");
    assert.equal(
      detectAccompagnementKind(
        "Guide d'évaluation des besoins de compensation en matière de scolarisation",
      ),
      "gevasco",
    );
    assert.equal(detectAccompagnementKind("Bulletin T1"), null);
  });

  it("garde la compat isPapDocumentTitle pour tout accompagnement", () => {
    assert.equal(isAccompagnementDocumentTitle("PAI"), true);
    assert.equal(isPapDocumentTitle("PPS"), true);
    assert.equal(isPapDocumentTitle("GEVASCO"), true);
  });

  it("génère un titre de dépôt", () => {
    assert.equal(defaultAccompagnementDocumentTitle("pai", "2025-2026"), "PAI 2025-2026");
    assert.equal(defaultAccompagnementDocumentTitle("pps", null), "PPS");
    assert.equal(defaultAccompagnementDocumentTitle("gevasco", "2025-2026"), "GEVASCO 2025-2026");
  });
});
