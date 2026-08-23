import assert from "node:assert/strict";
import test from "node:test";
import {
  capabilitiesFromFluxes,
  elevesFluxIdForSecteur,
  ENSEIGNANTS_SHARED_BASE_PATH,
  findFluxBasePath,
  fluxesAssignedToUser,
  mergeOcrFluxGrid,
  migrateLegacyUserSecteursToOcrFlux,
  OCR_FLUX_META,
} from "./ocr-flux";

test("migre userSecteurs vers les flux élèves sans perdre le rattachement", () => {
  const grid = migrateLegacyUserSecteursToOcrFlux({
    userSecteurs: [
      {
        externalUserId: "user_lycee",
        match: "fh@ecole.fr",
        displayName: "François",
        secteur: "lycee",
      },
    ],
    basesBySecteur: { lycee: { basePath: "Dossier élèves/Lycée LaPro" } },
  });
  const lycee = grid.find((r) => r.id === "eleves_lycee");
  assert.equal(lycee?.externalUserId, "user_lycee");
  assert.equal(lycee?.basePath, "Dossier élèves/Lycée LaPro");
  assert.equal(grid.find((r) => r.id === "eleves_college")?.externalUserId, undefined);
});

test("ne recouvre pas un ocrFlux déjà renseigné", () => {
  const grid = migrateLegacyUserSecteursToOcrFlux({
    ocrFlux: [
      {
        id: "eleves_lycee",
        externalUserId: "nouveau",
        match: "n@ecole.fr",
      },
    ],
    userSecteurs: [{ externalUserId: "ancien", match: "a@ecole.fr", secteur: "lycee" }],
  });
  assert.equal(grid.find((r) => r.id === "eleves_lycee")?.externalUserId, "nouveau");
});

test("autorise la même personne sur plusieurs flux enseignants", () => {
  const grid = mergeOcrFluxGrid([
    { id: "eleves_college", externalUserId: "col", match: "c@ecole.fr" },
    { id: "enseignants_college", externalUserId: "col", match: "c@ecole.fr" },
    { id: "enseignants_lycee", externalUserId: "col", match: "c@ecole.fr" },
  ]);
  const assigned = fluxesAssignedToUser(grid, { id: "col", emails: ["c@ecole.fr"] });
  assert.deepEqual(
    assigned.map((f) => f.id).sort(),
    ["eleves_college", "enseignants_college", "enseignants_lycee"],
  );
  assert.equal(capabilitiesFromFluxes(assigned).primaryEleves?.secteur, "college");
});

test("les 3 flux enseignants partagent le même chemin OneDrive par défaut", () => {
  assert.equal(OCR_FLUX_META.enseignants_ecole.defaultBasePath, ENSEIGNANTS_SHARED_BASE_PATH);
  assert.equal(OCR_FLUX_META.enseignants_college.defaultBasePath, ENSEIGNANTS_SHARED_BASE_PATH);
  assert.equal(OCR_FLUX_META.enseignants_lycee.defaultBasePath, ENSEIGNANTS_SHARED_BASE_PATH);

  const grid = mergeOcrFluxGrid([
    { id: "enseignants_college", externalUserId: "col", match: "c@ecole.fr" },
    { id: "enseignants_lycee", externalUserId: "col", match: "c@ecole.fr" },
  ]);
  const assigned = fluxesAssignedToUser(grid, { id: "col", emails: ["c@ecole.fr"] });
  const caps = capabilitiesFromFluxes(assigned);
  assert.equal(findFluxBasePath(caps, "enseignants", "college"), ENSEIGNANTS_SHARED_BASE_PATH);
  assert.equal(findFluxBasePath(caps, "enseignants", "lycee"), ENSEIGNANTS_SHARED_BASE_PATH);
  assert.equal(findFluxBasePath(caps, "enseignants"), ENSEIGNANTS_SHARED_BASE_PATH);
});

test("migre l’ancien id unique enseignants vers les 3 lignes", () => {
  const grid = mergeOcrFluxGrid([
    {
      id: "enseignants",
      externalUserId: "col",
      match: "c@ecole.fr",
      basePath: "Dossier enseignants",
    },
  ] as Parameters<typeof mergeOcrFluxGrid>[0]);
  assert.equal(grid.find((r) => r.id === "enseignants_college")?.externalUserId, "col");
  assert.equal(grid.find((r) => r.id === "enseignants_lycee")?.externalUserId, "col");
  assert.equal(grid.find((r) => r.id === "enseignants_ecole")?.basePath, "Dossier enseignants");
});

test("collapse les anciens chemins …/Collège vers la racine commune", () => {
  const grid = mergeOcrFluxGrid([
    {
      id: "enseignants_college",
      externalUserId: "col",
      match: "c@ecole.fr",
      basePath: "Dossier enseignants/Collège",
    },
  ]);
  assert.equal(grid.find((r) => r.id === "enseignants_college")?.basePath, "Dossier enseignants");
});

test("mappe le secteur élèves vers l’id de flux", () => {
  assert.equal(elevesFluxIdForSecteur("ecole"), "eleves_ecole");
  assert.equal(elevesFluxIdForSecteur("lycee"), "eleves_lycee");
});
