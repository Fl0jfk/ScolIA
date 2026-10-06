import assert from "node:assert/strict";
import test from "node:test";
import { isEleveActifPourListes } from "./eleve-actif-shared";
import type { EleveConfig } from "./eleves-config";

test("suivi stages — élève avec date de sortie passée exclu des effectifs actifs", () => {
  const now = new Date(2026, 9, 6);
  const eleves: EleveConfig[] = [
    {
      ine: "A",
      nom: "ACTIF",
      prenom: "Lea",
      folderName: "ACTIF Lea",
      classe: "3A",
      status: "inscrit",
    },
    {
      ine: "B",
      nom: "SORTI",
      prenom: "Tom",
      folderName: "SORTI Tom",
      classe: "3A",
      status: "inscrit",
      dateSortie: "2026-10-01",
    },
  ];
  const actifs = eleves.filter((e) => isEleveActifPourListes(e, now));
  assert.equal(actifs.length, 1);
  assert.equal(actifs[0]?.nom, "ACTIF");
});
