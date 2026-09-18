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

test("DATE_SORTIE → ancien mais conserve la classe 2B (même sans CODE_STRUCTURE)", () => {
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
  const sortiSansClasse: EleveConfig = {
    ine: "INE2B",
    nom: "HAVIS",
    prenom: "Gabriel",
    folderName: "HAVIS Gabriel",
    status: "ancien",
    regime: "Externe",
  };
  const { eleves } = mergeElevesLists(existing, [sortiSansClasse], {
    replaceRegime: true,
    importCycle: "lycee",
  });
  assert.equal(eleves[0]?.status, "ancien");
  assert.equal(eleves[0]?.classe, "2B");
});

test("DATE_SORTIE ne remplace pas une classe 2B par TA du fichier sorti", () => {
  const existing: EleveConfig[] = [
    {
      ine: "INE2B2",
      nom: "DUPONT",
      prenom: "Alice",
      folderName: "DUPONT Alice",
      classe: "2B",
      status: "inscrit",
    },
  ];
  const sortiTerm: EleveConfig = {
    ine: "INE2B2",
    nom: "DUPONT",
    prenom: "Alice",
    folderName: "DUPONT Alice",
    classe: "TA",
    status: "ancien",
    regime: "Externe",
  };
  const { eleves } = mergeElevesLists(existing, [sortiTerm], {
    replaceRegime: true,
    importCycle: "lycee",
  });
  assert.equal(eleves[0]?.status, "ancien");
  assert.equal(eleves[0]?.classe, "2B");
});

test("XML sans classe ne vide pas une classe déjà en fiche", () => {
  const existing: EleveConfig[] = [
    {
      ine: "INE2C",
      nom: "DURAND",
      prenom: "Leo",
      folderName: "DURAND Leo",
      classe: "2C",
      status: "inscrit",
      regime: "Demi-pensionnaire",
    },
  ];
  const sansClasse: EleveConfig = {
    ine: "INE2C",
    nom: "DURAND",
    prenom: "Leo",
    folderName: "DURAND Leo",
    status: "inscrit",
  };
  const { eleves } = mergeElevesLists(existing, [sansClasse], {
    importCycle: "lycee",
  });
  assert.equal(eleves[0]?.classe, "2C");
  assert.equal(eleves[0]?.status, "inscrit");
  assert.equal(eleves[0]?.regime, "Demi-pensionnaire");
});

test("sortie lycée sans INE ne radie pas par homonymie", () => {
  const existing: EleveConfig[] = [
    {
      ine: "INEOK",
      nom: "MARTIN",
      prenom: "Paul",
      folderName: "MARTIN Paul",
      classe: "2A",
      status: "inscrit",
    },
  ];
  const sortiSansIne: EleveConfig = {
    nom: "MARTIN",
    prenom: "Paul",
    folderName: "MARTIN Paul",
    status: "ancien",
    regime: "Externe",
  };
  const { eleves, stats } = mergeElevesLists(existing, [sortiSansIne], {
    replaceRegime: true,
    importCycle: "lycee",
  });
  assert.equal(eleves[0]?.status, "inscrit");
  assert.equal(stats.updated, 0);
});

test("sortie lycée radie bien une terminale TA en gardant la classe", () => {
  const existing: EleveConfig[] = [
    {
      ine: "INET",
      nom: "LEROY",
      prenom: "Ines",
      folderName: "LEROY Ines",
      classe: "TA",
      status: "inscrit",
    },
  ];
  const sorti: EleveConfig = {
    ine: "INET",
    nom: "LEROY",
    prenom: "Ines",
    folderName: "LEROY Ines",
    status: "ancien",
    regime: "Externe",
  };
  const { eleves } = mergeElevesLists(existing, [sorti], {
    replaceRegime: true,
    importCycle: "lycee",
  });
  assert.equal(eleves[0]?.status, "ancien");
  assert.equal(eleves[0]?.classe, "TA");
});
