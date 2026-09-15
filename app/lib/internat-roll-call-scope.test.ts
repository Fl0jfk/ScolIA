import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  filterInternatStudentsByViewerScope,
  resolveInternatRollCallViewerScope,
} from "@/app/lib/internat-rbac";
import type { InternatStudent } from "@/app/lib/internat-types";

function student(partial: Partial<InternatStudent> & { id: string; etablissement: string }): InternatStudent {
  return {
    eleveRef: { nom: "Test", prenom: "A" },
    sexe: "M",
    classe: "3A",
    actif: true,
    createdAt: "",
    updatedAt: "",
    ...partial,
  };
}

describe("resolveInternatRollCallViewerScope", () => {
  it("laisse l'équipe internat voir tout", () => {
    assert.equal(resolveInternatRollCallViewerScope({ roles: ["internat"] }), "all");
    assert.equal(resolveInternatRollCallViewerScope({ roles: ["surveillant"] }), "all");
  });

  it("restreint direction collège / lycée", () => {
    assert.equal(
      resolveInternatRollCallViewerScope({ roles: ["direction_college"] }),
      "college",
    );
    assert.equal(
      resolveInternatRollCallViewerScope({ roles: ["direction_lycee"] }),
      "lycee",
    );
  });

  it("restreint CPE via e-mail configuré", () => {
    assert.equal(
      resolveInternatRollCallViewerScope({
        roles: ["cpe"],
        email: "cpe.college@example.fr",
        recipients: { cpeCollege: "cpe.college@example.fr", cpeLycee: "cpe.lycee@example.fr" },
      }),
      "college",
    );
    assert.equal(
      resolveInternatRollCallViewerScope({
        roles: ["cpe"],
        email: "cpe.lycee@example.fr",
        recipients: { cpeCollege: "cpe.college@example.fr", cpeLycee: "cpe.lycee@example.fr" },
      }),
      "lycee",
    );
  });
});

describe("filterInternatStudentsByViewerScope", () => {
  const students = [
    student({ id: "1", etablissement: "Collège La Providence" }),
    student({ id: "2", etablissement: "Lycée La Providence" }),
  ];

  it("filtre collège / lycée", () => {
    assert.deepEqual(
      filterInternatStudentsByViewerScope(students, "college").map((s) => s.id),
      ["1"],
    );
    assert.deepEqual(
      filterInternatStudentsByViewerScope(students, "lycee").map((s) => s.id),
      ["2"],
    );
    assert.equal(filterInternatStudentsByViewerScope(students, "all").length, 2);
  });
});
