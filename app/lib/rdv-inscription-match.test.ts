import assert from "node:assert/strict";
import { describe, it } from "node:test";
import type { EleveConfig } from "@/app/lib/eleves-config";
import {
  elevesMatchingParentContact,
  matchRdvInscriptionCandidates,
  normalizeParentPhone,
  scoreEleveNameMatch,
} from "@/app/lib/rdv-inscription-match";

function eleve(partial: Partial<EleveConfig> & Pick<EleveConfig, "id" | "nom" | "prenom">): EleveConfig {
  return {
    id: partial.id,
    ine: partial.ine || "",
    nom: partial.nom,
    prenom: partial.prenom,
    folderName: `${partial.nom}_${partial.prenom}`,
    classe: partial.classe,
    parentEmail: partial.parentEmail,
    parent1Email: partial.parent1Email,
    parent2Email: partial.parent2Email,
    parentPhone: partial.parentPhone,
    parent1Phone: partial.parent1Phone,
    parent2Phone: partial.parent2Phone,
    status: partial.status || "inscrit",
  } as EleveConfig;
}

describe("rdv-inscription-match", () => {
  it("normalise les téléphones FR", () => {
    assert.equal(normalizeParentPhone("06 12 34 56 78"), "0612345678");
    assert.equal(normalizeParentPhone("+33 6 12 34 56 78"), "0612345678");
  });

  it("ne matche jamais hors contact parent", () => {
    const eleves = [
      eleve({
        id: "a",
        nom: "DUPONT",
        prenom: "Marie",
        parentEmail: "autre@example.com",
      }),
      eleve({
        id: "b",
        nom: "DUPONT",
        prenom: "Marie",
        parentEmail: "parent@example.com",
      }),
    ];
    const pool = elevesMatchingParentContact(eleves, "parent@example.com");
    assert.equal(pool.length, 1);
    assert.equal(pool[0]!.id, "b");

    const cands = matchRdvInscriptionCandidates({
      eleves,
      parentEmail: "inconnu@example.com",
      studentLastName: "DUPONT",
      studentFirstName: "Marie",
    });
    assert.equal(cands.length, 0);
  });

  it("gère les noms composés", () => {
    const e = eleve({
      id: "c",
      nom: "LE ROUX",
      prenom: "Sophie",
      parentEmail: "p@ex.com",
    });
    assert.ok(scoreEleveNameMatch(e, "Le Roux", "Sophie") >= 80);
    const cands = matchRdvInscriptionCandidates({
      eleves: [e],
      parentEmail: "p@ex.com",
      studentLastName: "LE ROUX",
      studentFirstName: "Sophie",
    });
    assert.equal(cands[0]?.id, "c");
  });

  it("propose les frères/sœurs du même mail si le nom matche", () => {
    const eleves = [
      eleve({
        id: "1",
        nom: "MARTIN",
        prenom: "Alice",
        parentEmail: "famille@ex.com",
        classe: "3e A",
      }),
      eleve({
        id: "2",
        nom: "MARTIN",
        prenom: "Bob",
        parentEmail: "famille@ex.com",
        classe: "5e B",
      }),
    ];
    const cands = matchRdvInscriptionCandidates({
      eleves,
      parentEmail: "famille@ex.com",
      studentLastName: "Martin",
      studentFirstName: "Bob",
    });
    assert.equal(cands.length, 1);
    assert.equal(cands[0]!.id, "2");
  });
});
