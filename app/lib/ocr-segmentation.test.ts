import assert from "node:assert/strict";
import test from "node:test";
import { detectPageOwner, identityAnchoredSegments, type KnownStudent } from "./ocr-segmentation";

function stu(partial: Pick<KnownStudent, "ine" | "nom" | "prenom">): KnownStudent {
  return {
    ine: partial.ine,
    nom: partial.nom,
    prenom: partial.prenom,
    folderName: `${partial.nom} ${partial.prenom}`,
    normNom: partial.nom.toLowerCase(),
    normPrenom: partial.prenom.toLowerCase(),
  };
}

const roster: KnownStudent[] = [
  stu({ ine: "1111111111A", nom: "DUPONT", prenom: "Marie" }),
  stu({ ine: "2222222222B", nom: "MARTIN", prenom: "Paul" }),
];

test("INE inconnu de la liste = propriétaire anonyme, pas rattaché au suivant", () => {
  const owner = detectPageOwner("Bulletin INE 9999999999Z SORTI Jean", roster);
  assert.equal(owner?.ine, "9999999999Z");
  assert.equal(owner?.folderName, undefined);
});

test("deux élèves sortis puis un élève connu → 3 documents, pas un seul", () => {
  const pages: Record<string, string> = {
    "1": "Bulletin scolaire INE 9999999999Z SORTI Alice",
    "2": "Bulletin scolaire INE 8888888888Y SORTI Bruno",
    "3": "Bulletin scolaire INE 1111111111A DUPONT Marie",
  };
  const r = identityAnchoredSegments(pages, 3, roster);
  assert.equal(r.segments.length, 3);
  assert.equal(r.segments[0]?.folderName, undefined);
  assert.equal(r.segments[1]?.folderName, undefined);
  assert.equal(r.segments[2]?.folderName, "DUPONT Marie");
  assert.equal(r.segments[2]?.pageStart, 3);
  assert.equal(r.segments[2]?.pageEnd, 3);
});

test("bulletin 2 pages même INE inconnu → un seul document introuvable", () => {
  const pages: Record<string, string> = {
    "1": "Bulletin scolaire INE 9999999999Z SORTI Alice 1/2",
    "2": "Notes et appréciations 2/2",
    "3": "Bulletin scolaire INE 1111111111A DUPONT Marie",
  };
  const r = identityAnchoredSegments(pages, 3, roster);
  assert.equal(r.segments.length, 2);
  assert.equal(r.segments[0]?.pageStart, 1);
  assert.equal(r.segments[0]?.pageEnd, 2);
  assert.equal(r.segments[0]?.folderName, undefined);
  assert.equal(r.segments[1]?.folderName, "DUPONT Marie");
});

test("90 bulletins 1 page dont 30 INE inconnus → 90 segments pas 60", () => {
  const pages: Record<string, string> = {};
  const students: KnownStudent[] = [];
  for (let i = 1; i <= 90; i++) {
    const ine = `${String(i).padStart(10, "0")}A`;
    if (i > 30) {
      students.push(stu({ ine, nom: `NOM${i}`, prenom: `Prenom${i}` }));
    }
    pages[String(i)] = `Bulletin scolaire INE ${ine} ELEVE ${i}`;
  }
  const r = identityAnchoredSegments(pages, 90, students);
  assert.equal(r.segments.length, 90);
  assert.equal(r.segments.filter((s) => !s.folderName).length, 30);
  assert.equal(r.segments.filter((s) => s.folderName).length, 60);
});
