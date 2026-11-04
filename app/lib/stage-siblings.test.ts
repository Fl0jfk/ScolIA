import { test } from "node:test";
import assert from "node:assert/strict";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { filterStageSiblings } from "@/app/lib/stage-siblings-filter";

function eleve(over: Partial<EleveConfig> & { id: string }): EleveConfig {
  return {
    ine: `INE${over.id}`,
    nom: "Dupont",
    prenom: `Enfant${over.id}`,
    folderName: "",
    classe: "6e A",
    status: "inscrit",
    dateNaissance: "2012-05-01",
    ...over,
  } as EleveConfig;
}

const ok = () => true;
const run = (eleves: EleveConfig[], extra: Partial<Parameters<typeof filterStageSiblings>[0]> = {}) =>
  filterStageSiblings({
    eleves,
    verifiedEmails: ["parent@x.fr"],
    isClassEligible: ok,
    ...extra,
  });

test("rattachement par parent1, parent et parent2, insensible à la casse et aux espaces", () => {
  const res = run([
    eleve({ id: "1", parent1Email: "  PARENT@X.fr " }),
    eleve({ id: "2", parentEmail: "Parent@x.FR" }),
    eleve({ id: "3", parent2Email: "parent@x.fr  " }),
    eleve({ id: "4", parent1Email: "autre@x.fr" }),
  ]);
  assert.deepEqual(res.map((s) => s.id).sort(), ["1", "2", "3"]);
});

test("le mail propre de l'élève n'est jamais un critère", () => {
  assert.equal(run([eleve({ id: "1", email: "parent@x.fr" })]).length, 0);
});

test("rattachement par foyer (ids fournis)", () => {
  const res = run([eleve({ id: "1" }), eleve({ id: "2" })], { foyerEleveIds: ["2"] });
  assert.deepEqual(res.map((s) => s.id), ["2"]);
});

test("élèves d'école exclus : secteur explicite, MEF, libellé de classe", () => {
  const mefMap = new Map([["12345678901", "ecole" as const]]);
  const res = run(
    [
      eleve({ id: "1", parentEmail: "parent@x.fr", secteur: "ecole", classe: "6e A" }),
      eleve({ id: "2", parentEmail: "parent@x.fr", mef: "12345678901", classe: "X" }),
      eleve({ id: "3", parentEmail: "parent@x.fr", classe: "CM2" }),
      eleve({ id: "4", parentEmail: "parent@x.fr", classe: "GS" }),
      eleve({ id: "5", parentEmail: "parent@x.fr", classe: "3e B" }),
    ],
    { mefMap },
  );
  assert.deepEqual(res.map((s) => s.id), ["5"]);
});

test("inactifs, sans date de naissance et classes non éligibles exclus", () => {
  const res = run(
    [
      eleve({ id: "1", parentEmail: "parent@x.fr", status: "ancien" }),
      eleve({ id: "2", parentEmail: "parent@x.fr", dateSortie: "2000-01-01" }),
      eleve({ id: "3", parentEmail: "parent@x.fr", dateNaissance: "" }),
      eleve({ id: "4", parentEmail: "parent@x.fr", classe: "4e FERMEE" }),
      eleve({ id: "5", parentEmail: "parent@x.fr" }),
    ],
    { isClassEligible: (c) => c !== "4e FERMEE" },
  );
  assert.deepEqual(res.map((s) => s.id), ["5"]);
});

test("l'élève courant est marqué ; sans adresse vérifiée, aucun résultat", () => {
  const eleves = [
    eleve({ id: "1", prenom: "Zoe", parentEmail: "parent@x.fr" }),
    eleve({ id: "2", prenom: "Adam", parentEmail: "parent@x.fr" }),
  ];
  const res = run(eleves, { currentEleveId: "1" });
  assert.deepEqual(res.map((s) => [s.id, s.current]), [["2", false], ["1", true]]);
  assert.deepEqual(run(eleves, { verifiedEmails: [] }), []);
});

test("école : nom « TAN », classe CM2, folderName « TAN Emma » → école (folderName ignoré)", () => {
  const res = run([
    eleve({ id: "1", parentEmail: "parent@x.fr", nom: "TAN", prenom: "Emma", folderName: "TAN Emma", classe: "CM2" }),
  ]);
  assert.equal(res.length, 0);
});

test("pas école : classe 3eA, nom « TERMINAL », folderName trompeur", () => {
  const res = run([
    eleve({ id: "1", parentEmail: "parent@x.fr", nom: "TERMINAL", folderName: "TERMINAL Lyc", classe: "3eA" }),
  ]);
  assert.equal(res.length, 1);
});
