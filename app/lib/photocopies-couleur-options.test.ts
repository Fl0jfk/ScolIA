import { test } from "node:test";
import assert from "node:assert/strict";
import {
  normalizePhotoCopieFaces,
  normalizePhotoCopieFinitions,
  photoCopieOptionsLabel,
} from "./photocopies-couleur-types";

test("faces : recto par défaut, recto verso reconnu", () => {
  assert.equal(normalizePhotoCopieFaces(undefined), "RECTO");
  assert.equal(normalizePhotoCopieFaces("n'importe quoi"), "RECTO");
  assert.equal(normalizePhotoCopieFaces("RECTO_VERSO"), "RECTO_VERSO");
  assert.equal(normalizePhotoCopieFaces("recto-verso"), "RECTO_VERSO");
});

test("finitions : valeurs inconnues et doublons écartés, ordre du formulaire", () => {
  assert.deepEqual(normalizePhotoCopieFinitions("PLASTIFICATION"), []);
  assert.deepEqual(
    normalizePhotoCopieFinitions(["reliure", "PLASTIFICATION", "PLASTIFICATION", "<script>"]),
    ["PLASTIFICATION", "RELIURE"],
  );
});

test("libellé : vide pour une ancienne demande, cumul des options sinon", () => {
  assert.equal(photoCopieOptionsLabel({}), "");
  assert.equal(photoCopieOptionsLabel({ faces: "RECTO" }), "Recto");
  assert.equal(
    photoCopieOptionsLabel({ faces: "RECTO_VERSO", finitions: ["AGRAFAGE", "PLASTIFICATION"] }),
    "Recto verso · Plastification, Agrafage",
  );
});
