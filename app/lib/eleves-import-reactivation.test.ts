import assert from "node:assert/strict";
import test from "node:test";
import { mergeElevesLists } from "./eleves-import";

test("réimport Excel inscrit en classe réactive un ancien (vide date_sortie)", () => {
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
  assert.equal(eleves[0]?.status, "inscrit");
  assert.equal(eleves[0]?.dateSortie, undefined);
});

test("réimport avec date de sortie future garde inscrit", () => {
  const existing = [
    {
      ine: "222",
      nom: "DUPONT",
      prenom: "Marie",
      folderName: "DUPONT Marie",
      classe: "4B",
      status: "ancien" as const,
      dateSortie: "2026-09-01",
    },
  ];
  const incoming = [
    {
      ine: "222",
      nom: "DUPONT",
      prenom: "Marie",
      folderName: "DUPONT Marie",
      classe: "4B",
      status: "inscrit" as const,
      dateSortie: "2027-06-30",
    },
  ];
  const { eleves } = mergeElevesLists(existing, incoming);
  assert.equal(eleves[0]?.status, "inscrit");
  assert.equal(eleves[0]?.dateSortie, "2027-06-30");
});
