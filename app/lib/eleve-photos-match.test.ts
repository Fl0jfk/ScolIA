import assert from "node:assert/strict";
import test from "node:test";
import type { EleveConfig } from "./eleves-config";
import {
  matchEleveForPhoto,
  matchEleveFromPhotoFilename,
  parsePhotoFilename,
} from "./eleve-photos-match";

function eleve(nom: string, prenom: string, id = "1"): EleveConfig {
  return { id, ine: "", nom, prenom, folderName: `${nom} ${prenom}` };
}

test("parse NOM Prenom photo filename", () => {
  assert.deepEqual(parsePhotoFilename("DUPONT Marie.jpg"), { nom: "DUPONT", prenom: "Marie" });
  assert.deepEqual(parsePhotoFilename("MARTIN_Jean-Pierre.png"), {
    nom: "MARTIN",
    prenom: "Jean Pierre",
  });
});

test("match noms composés avec espaces (LE ROUX, DE LA …)", () => {
  const eleves = [
    eleve("DUPONT", "Marie", "a"),
    eleve("LE ROUX", "Sophie", "b"),
    eleve("DE LA FONTAINE", "Jean", "c"),
  ];

  assert.equal(matchEleveFromPhotoFilename(eleves, "LE ROUX Sophie.jpg")?.id, "b");
  assert.equal(matchEleveFromPhotoFilename(eleves, "LE_ROUX_Sophie.jpg")?.id, "b");
  assert.equal(matchEleveFromPhotoFilename(eleves, "DE LA FONTAINE Jean.jpg")?.id, "c");
  assert.equal(matchEleveFromPhotoFilename(eleves, "DUPONT Marie.jpg")?.id, "a");
});

test("match prénom composé + nom simple", () => {
  const eleves = [eleve("MARTIN", "Jean-Pierre", "jp")];
  assert.equal(matchEleveFromPhotoFilename(eleves, "MARTIN_Jean-Pierre.png")?.id, "jp");
  assert.equal(matchEleveForPhoto(eleves, "MARTIN", "Jean Pierre")?.id, "jp");
});
