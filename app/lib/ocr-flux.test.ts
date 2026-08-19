import assert from "node:assert/strict";
import test from "node:test";
import {
  capabilitiesFromFluxes,
  elevesFluxIdForSecteur,
  fluxesAssignedToUser,
  mergeOcrFluxGrid,
  migrateLegacyUserSecteursToOcrFlux,
} from "./ocr-flux";

test("migre userSecteurs vers les flux élèves sans perdre le rattachement", () => {
  const grid = migrateLegacyUserSecteursToOcrFlux({
    userSecteurs: [
      {
        clerkUserId: "user_lycee",
        match: "fh@ecole.fr",
        displayName: "François",
        secteur: "lycee",
      },
    ],
    basesBySecteur: { lycee: { basePath: "Dossier élèves/Lycée LaPro" } },
  });
  const lycee = grid.find((r) => r.id === "eleves_lycee");
  assert.equal(lycee?.clerkUserId, "user_lycee");
  assert.equal(lycee?.basePath, "Dossier élèves/Lycée LaPro");
  assert.equal(grid.find((r) => r.id === "eleves_college")?.clerkUserId, undefined);
});

test("ne recouvre pas un ocrFlux déjà renseigné", () => {
  const grid = migrateLegacyUserSecteursToOcrFlux({
    ocrFlux: [
      {
        id: "eleves_lycee",
        clerkUserId: "nouveau",
        match: "n@ecole.fr",
      },
    ],
    userSecteurs: [{ clerkUserId: "ancien", match: "a@ecole.fr", secteur: "lycee" }],
  });
  assert.equal(grid.find((r) => r.id === "eleves_lycee")?.clerkUserId, "nouveau");
});

test("autorise la même personne sur plusieurs flux", () => {
  const grid = mergeOcrFluxGrid([
    { id: "eleves_college", clerkUserId: "col", match: "c@ecole.fr" },
    { id: "enseignants_college", clerkUserId: "col", match: "c@ecole.fr" },
    { id: "enseignants_lycee", clerkUserId: "col", match: "c@ecole.fr" },
  ]);
  const assigned = fluxesAssignedToUser(grid, { id: "col", emails: ["c@ecole.fr"] });
  assert.deepEqual(
    assigned.map((f) => f.id).sort(),
    ["eleves_college", "enseignants_college", "enseignants_lycee"],
  );
  assert.equal(capabilitiesFromFluxes(assigned).primaryEleves?.secteur, "college");
});

test("mappe le secteur élèves vers l’id de flux", () => {
  assert.equal(elevesFluxIdForSecteur("ecole"), "eleves_ecole");
  assert.equal(elevesFluxIdForSecteur("lycee"), "eleves_lycee");
});
