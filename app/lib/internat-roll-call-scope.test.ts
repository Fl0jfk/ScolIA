import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  filterInternatStudentsByViewerScope,
  resolveInternatRollCallViewerScope,
} from "@/app/lib/internat-rbac";
import { resolveInternatStudentKind } from "@/app/lib/internat-etablissement-kind";
import type { InternatStudent } from "@/app/lib/internat-types";
import type { Establishment } from "@/app/lib/app-config-schemas";

function student(partial: Partial<InternatStudent> & { id: string; etablissement: string }): InternatStudent {
  return {
    eleveRef: { nom: "Test", prenom: "A", folderName: "TEST—A" },
    sexe: "M",
    classe: "3A",
    actif: true,
    createdAt: "",
    updatedAt: "",
    ...partial,
  };
}

const establishments: Establishment[] = [
  {
    id: "college",
    label: "Collège La Providence",
    kind: "college",
    active: true,
  },
  {
    id: "lycee",
    label: "Lycée La Providence",
    kind: "lycee",
    active: true,
  },
];

describe("resolveInternatStudentKind", () => {
  it("utilise la classe en priorité (évite les lycéens mal étiquetés collège)", () => {
    assert.equal(
      resolveInternatStudentKind(
        { etablissement: "Collège La Providence", classe: "Tle A" },
        establishments,
      ),
      "lycee",
    );
    assert.equal(
      resolveInternatStudentKind(
        { etablissement: "Lycée La Providence", classe: "5e B" },
        establishments,
      ),
      "college",
    );
  });

  it("retombe sur le catalogue / libellé si pas de classe fiable", () => {
    assert.equal(
      resolveInternatStudentKind({ etablissement: "Lycée La Providence", classe: "" }, establishments),
      "lycee",
    );
    assert.equal(
      resolveInternatStudentKind({ etablissement: "Ensemble Collège Lycée", classe: "" }, []),
      null,
    );
  });
});

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
});

describe("filterInternatStudentsByViewerScope", () => {
  const students = [
    student({ id: "1", etablissement: "Collège La Providence", classe: "3A" }),
    student({ id: "2", etablissement: "Lycée La Providence", classe: "1re A" }),
  ];

  it("filtre collège / lycée", () => {
    assert.deepEqual(
      filterInternatStudentsByViewerScope(students, "college", establishments).map((s) => s.id),
      ["1"],
    );
    assert.deepEqual(
      filterInternatStudentsByViewerScope(students, "lycee", establishments).map((s) => s.id),
      ["2"],
    );
    assert.equal(filterInternatStudentsByViewerScope(students, "all").length, 2);
  });
});
