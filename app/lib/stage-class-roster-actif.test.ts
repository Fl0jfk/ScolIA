import assert from "node:assert/strict";
import test from "node:test";
import { isEleveActifPourListes } from "./eleve-actif-shared";
import type { EleveConfig } from "./eleves-config";
import { orphanConventionDisposition, stagePersonNamesMatch } from "./stage-person-name";

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
    {
      ine: "C",
      nom: "PITTE",
      prenom: "Sohan",
      folderName: "PITTE Sohan",
      classe: "3B",
      status: "ancien",
      dateSortie: "2026-09-15",
    },
  ];
  const actifs = eleves.filter((e) => isEleveActifPourListes(e, now));
  assert.equal(actifs.length, 1);
  assert.equal(actifs[0]?.nom, "ACTIF");
  assert.equal(
    orphanConventionDisposition({
      actifInOtherClass: false,
      knownButNotActif: !actifs.some((e) =>
        stagePersonNamesMatch(e, { lastName: "PITTE", firstName: "Sohan" }),
      ),
    }),
    "skip_sorti",
  );
});

test("suivi stages — Apolline Seglas 2nde ne doit pas créer d’orphelin en 3e", () => {
  const eleves: EleveConfig[] = [
    {
      ine: "S1",
      nom: "SEGLAS",
      prenom: "Apolline",
      folderName: "SEGLAS Apolline",
      classe: "2A",
      status: "inscrit",
    },
  ];
  const className = "3A";
  const actifs = eleves.filter((e) => isEleveActifPourListes(e));
  const inClass = actifs.filter((e) => e.classe === className);
  const matchedInClass = inClass.some((e) =>
    stagePersonNamesMatch(e, { lastName: "SEGLAS", firstName: "Apolline" }),
  );
  assert.equal(matchedInClass, false);
  const actifElsewhere = actifs.some(
    (e) =>
      stagePersonNamesMatch(e, { lastName: "SEGLAS", firstName: "Apolline" }) &&
      e.classe !== className,
  );
  assert.equal(
    orphanConventionDisposition({ actifInOtherClass: actifElsewhere, knownButNotActif: false }),
    "skip_other_class",
  );
});
