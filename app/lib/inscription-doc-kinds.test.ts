import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  buildInscriptionDocumentTitle,
  guessInscriptionKindFromFileName,
} from "./inscription-doc-kinds";

describe("inscription-doc-kinds", () => {
  it("buildInscriptionDocumentTitle prefixes known student identity", () => {
    const title = buildInscriptionDocumentTitle({
      nom: "Dupont",
      prenom: "Alice",
      kind: "bulletin",
      detail: "Bulletin 2e semestre 2024-2025",
    });
    assert.equal(title, "DUPONT Alice — Bulletin 2e semestre 2024-2025");
  });

  it("falls back to kind label when detail empty", () => {
    const title = buildInscriptionDocumentTitle({
      nom: "Martin",
      prenom: "Léa",
      kind: "fiche_inscription",
    });
    assert.equal(title, "MARTIN Léa — Fiche d'inscription");
  });

  it("guesses kinds from file names", () => {
    assert.equal(guessInscriptionKindFromFileName("fiche-inscription-scan.pdf"), "fiche_inscription");
    assert.equal(guessInscriptionKindFromFileName("Bulletin_T1.pdf"), "bulletin");
    assert.equal(guessInscriptionKindFromFileName("CNI_recto.jpg"), "piece_identite");
    assert.equal(guessInscriptionKindFromFileName("random.pdf"), "autre");
  });
});
