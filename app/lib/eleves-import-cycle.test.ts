import assert from "node:assert/strict";
import test from "node:test";
import { mergeElevesLists } from "@/app/lib/eleves-import";
import type { EleveConfig } from "@/app/lib/eleves-config";

test("sortie collège ne radie pas un lycéen 2A (même INE)", () => {
  const existing: EleveConfig[] = [
    {
      ine: "140329773DA",
      nom: "GOMEZ PETIT",
      prenom: "Arthur",
      folderName: "GOMEZ PETIT Arthur",
      classe: "2A",
      status: "inscrit",
    },
  ];
  const sortiCollege: EleveConfig = {
    ine: "140329773DA",
    nom: "GOMEZ PETIT",
    prenom: "Arthur",
    folderName: "GOMEZ PETIT Arthur",
    status: "ancien",
    regime: "Externe",
  };
  const { eleves } = mergeElevesLists(existing, [sortiCollege], {
    replaceRegime: true,
    importCycle: "college",
  });
  assert.equal(eleves.length, 1);
  assert.equal(eleves[0]?.status, "inscrit");
  assert.equal(eleves[0]?.classe, "2A");
});

test("import collège ne réécrit pas la classe d’un lycéen 2B", () => {
  const existing: EleveConfig[] = [
    {
      ine: "INE2B",
      nom: "HAVIS",
      prenom: "Gabriel",
      folderName: "HAVIS Gabriel",
      classe: "2B",
      status: "inscrit",
    },
  ];
  const collegeRow: EleveConfig = {
    ine: "INE2B",
    nom: "HAVIS",
    prenom: "Gabriel",
    folderName: "HAVIS Gabriel",
    classe: "3F",
    status: "inscrit",
  };
  const { eleves, stats } = mergeElevesLists(existing, [collegeRow], {
    importCycle: "college",
  });
  assert.equal(eleves[0]?.classe, "2B");
  assert.equal(eleves[0]?.status, "inscrit");
  assert.equal(stats.updated, 0);
  assert.equal(stats.kept, 1);
});

test("sortie collège radie bien un élève encore en classe collège (pas de fiche lycée)", () => {
  const existing: EleveConfig[] = [
    {
      ine: "INE1",
      nom: "DUPONT",
      prenom: "Lea",
      folderName: "DUPONT Lea",
      classe: "3B",
      status: "inscrit",
    },
  ];
  const sorti: EleveConfig = {
    ine: "INE1",
    nom: "DUPONT",
    prenom: "Lea",
    folderName: "DUPONT Lea",
    status: "ancien",
    regime: "Externe",
  };
  const { eleves } = mergeElevesLists(existing, [sorti], {
    replaceRegime: true,
    importCycle: "college",
  });
  assert.equal(eleves[0]?.status, "ancien");
});

test("sortie lycée ne radie pas un collégien", () => {
  const existing: EleveConfig[] = [
    {
      ine: "INE2",
      nom: "MARTIN",
      prenom: "Tom",
      folderName: "MARTIN Tom",
      classe: "5A",
      status: "inscrit",
    },
  ];
  const sorti: EleveConfig = {
    ine: "INE2",
    nom: "MARTIN",
    prenom: "Tom",
    folderName: "MARTIN Tom",
    status: "ancien",
    regime: "Externe",
  };
  const { eleves } = mergeElevesLists(existing, [sorti], {
    replaceRegime: true,
    importCycle: "lycee",
  });
  assert.equal(eleves[0]?.status, "inscrit");
  assert.equal(eleves[0]?.classe, "5A");
});

test("sortie collège radie un élève sans classe (pas de fiche lycée détectable)", () => {
  const existing: EleveConfig[] = [
    {
      ine: "INE3",
      nom: "BERNARD",
      prenom: "Nina",
      folderName: "BERNARD Nina",
      status: "inscrit",
    },
  ];
  const sorti: EleveConfig = {
    ine: "INE3",
    nom: "BERNARD",
    prenom: "Nina",
    folderName: "BERNARD Nina",
    status: "ancien",
    regime: "Externe",
  };
  const { eleves } = mergeElevesLists(existing, [sorti], {
    replaceRegime: true,
    importCycle: "college",
  });
  assert.equal(eleves[0]?.status, "ancien");
});
