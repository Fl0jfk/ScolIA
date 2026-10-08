import assert from "node:assert/strict";
import { test } from "node:test";
import {
  dedupeInternatStudents,
  internatStudentMatchesRoster,
  normalizeInternatPersonPart,
} from "@/app/lib/internat-dedupe";
import type { InternatStudent } from "@/app/lib/internat-types";

function stub(partial: {
  id: string;
  nom: string;
  prenom: string;
  ine?: string;
  folderName?: string;
  actif?: boolean;
  roomId?: string | null;
}): InternatStudent {
  const now = "2026-01-01T00:00:00.000Z";
  return {
    id: partial.id,
    eleveRef: {
      nom: partial.nom,
      prenom: partial.prenom,
      ine: partial.ine,
      folderName: partial.folderName || `${partial.nom} — ${partial.prenom}`,
    },
    sexe: "M",
    etablissement: "Lycée",
    classe: "2nde",
    roomId: partial.roomId,
    actif: partial.actif ?? true,
    createdAt: now,
    updatedAt: now,
  };
}

test("normalizeInternatPersonPart ignore accents et casse", () => {
  assert.equal(normalizeInternatPersonPart("Damné"), normalizeInternatPersonPart("DAMNE"));
  assert.equal(
    normalizeInternatPersonPart("Ensoni Junior"),
    normalizeInternatPersonPart("ensoní junior"),
  );
});

test("internatStudentMatchesRoster — accents + INE", () => {
  const a = stub({ id: "a", nom: "Damné", prenom: "Ensoni Junior", ine: "123" });
  assert.equal(
    internatStudentMatchesRoster(a, {
      nom: "DAMNE",
      prenom: "Ensoni Junior",
    }),
    true,
  );
  assert.equal(
    internatStudentMatchesRoster(a, {
      nom: "Autre",
      prenom: "Personne",
      ine: "123",
    }),
    true,
  );
  assert.equal(
    internatStudentMatchesRoster(a, { nom: "Autre", prenom: "Personne", ine: "999" }),
    false,
  );
});

test("dedupeInternatStudents fusionne Damné Ensoni en double", () => {
  const students = [
    stub({
      id: "stu-1",
      nom: "Damné",
      prenom: "Ensoni Junior",
      folderName: "Damné — Ensoni Junior",
    }),
    stub({
      id: "stu-2",
      nom: "DAMNE",
      prenom: "Ensoni Junior",
      folderName: "DAMNE — Ensoni Junior",
      ine: "INE001",
      roomId: "room-1",
    }),
  ];
  const result = dedupeInternatStudents(students, { by: "test", at: "2026-10-08T00:00:00.000Z" });
  assert.equal(result.mergedGroups, 1);
  assert.equal(result.removedActifs, 1);
  assert.equal(result.students.length, 1);
  assert.equal(result.students[0]!.actif, true);
  assert.equal(result.students[0]!.roomId, "room-1");
  assert.equal(result.students[0]!.eleveRef.ine, "INE001");
});

test("dedupeInternatStudents relie INE et nom séparés (union-find)", () => {
  const students = [
    stub({ id: "a", nom: "Damné", prenom: "Ensoni Junior", ine: "X1" }),
    stub({ id: "b", nom: "Damné", prenom: "Ensoni Junior" }),
    stub({ id: "c", nom: "AutreNom", prenom: "Autre", ine: "X1", roomId: "r" }),
  ];
  const result = dedupeInternatStudents(students);
  assert.equal(result.mergedGroups, 1);
  assert.equal(result.students.length, 1);
  assert.equal(result.students[0]!.roomId, "r");
});

test("dedupeInternatStudents laisse deux élèves distincts", () => {
  const students = [
    stub({ id: "a", nom: "Dupont", prenom: "Alice" }),
    stub({ id: "b", nom: "Martin", prenom: "Bob" }),
  ];
  const result = dedupeInternatStudents(students);
  assert.equal(result.mergedGroups, 0);
  assert.equal(result.students.length, 2);
});

test("normalize N'SONI → nsoni (apostrophe)", () => {
  assert.equal(normalizeInternatPersonPart("N'SONI"), "nsoni");
  assert.equal(normalizeInternatPersonPart("NSONI"), "nsoni");
  assert.equal(normalizeInternatPersonPart("N’SONI"), "nsoni");
});

test("dedupe Dane Junior N'SONI vs NSONI / colonnes inversées", () => {
  const students = [
    stub({
      id: "stu-a",
      nom: "N'SONI",
      prenom: "Dane Junior",
      folderName: "N'SONI — Dane Junior",
    }),
    stub({
      id: "stu-b",
      nom: "NSONI",
      prenom: "Dane Junior",
      folderName: "NSONI — Dane Junior",
      roomId: "ch-12",
    }),
    stub({
      id: "stu-c",
      nom: "Dane Junior",
      prenom: "N'SONI",
      folderName: "Dane Junior — N'SONI",
    }),
  ];
  const result = dedupeInternatStudents(students);
  assert.equal(result.mergedGroups, 1);
  assert.equal(result.students.length, 1);
  assert.equal(result.students[0]!.roomId, "ch-12");
  assert.equal(result.removedActifs, 2);
});
