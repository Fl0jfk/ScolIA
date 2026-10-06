import assert from "node:assert/strict";
import test from "node:test";
import { mergeElevesLists } from "./eleves-import";

test("import Excel inscrit ne réactive pas un ancien sans date de sortie", () => {
  const existing = [
    {
      ine: "111",
      nom: "BECIANI",
      prenom: "Thomas",
      folderName: "BECIANI Thomas",
      classe: "3A",
      status: "ancien" as const,
      dateSortie: "2026-09-01",
    },
  ];
  const incoming = [
    {
      ine: "111",
      nom: "BECIANI",
      prenom: "Thomas",
      folderName: "BECIANI Thomas",
      classe: "3A",
      status: "inscrit" as const,
    },
  ];
  const { eleves } = mergeElevesLists(existing, incoming);
  assert.equal(eleves[0]?.status, "ancien");
  assert.equal(eleves[0]?.dateSortie, "2026-09-01");
});
