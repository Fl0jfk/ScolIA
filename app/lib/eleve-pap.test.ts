import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  defaultAccompagnementDocumentTitle,
  detectAccompagnementKind,
  isAccompagnementDocumentTitle,
  isPapDocumentTitle,
} from "./eleve-pap";

describe("eleve-pap accompagnement", () => {
  it("détecte PAP / PAI / PPS", () => {
    assert.equal(detectAccompagnementKind("PAP 2025-2026"), "pap");
    assert.equal(detectAccompagnementKind("PAI allergie"), "pai");
    assert.equal(detectAccompagnementKind("PPS MDPH"), "pps");
    assert.equal(detectAccompagnementKind("Plan d'accompagnement personnalisé"), "pap");
    assert.equal(detectAccompagnementKind("Projet d'accueil individualisé"), "pai");
    assert.equal(detectAccompagnementKind("Bulletin T1"), null);
  });

  it("garde la compat isPapDocumentTitle pour tout accompagnement", () => {
    assert.equal(isAccompagnementDocumentTitle("PAI"), true);
    assert.equal(isPapDocumentTitle("PPS"), true);
  });

  it("génère un titre de dépôt", () => {
    assert.equal(defaultAccompagnementDocumentTitle("pai", "2025-2026"), "PAI 2025-2026");
    assert.equal(defaultAccompagnementDocumentTitle("pps", null), "PPS");
  });
});
