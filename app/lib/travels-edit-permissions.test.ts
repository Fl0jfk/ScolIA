import assert from "node:assert/strict";
import {
  canEditTravelsDates,
  canEditTravelsEffectif,
} from "@/app/lib/travels-edit-permissions";

const base = {
  isOwner: false,
  isDirection: false,
  isAdministratif: false,
  isGlobalAdmin: false,
  isCompta: false,
  status: "VALIDE",
};

assert.equal(
  canEditTravelsEffectif({ ...base, isCompta: true, status: "VALIDE" }),
  true,
  "compta peut éditer effectif / élèves sur dossier finalisé",
);

assert.equal(
  canEditTravelsEffectif({ ...base, isCompta: true, status: "EN_ATTENTE_COMPTA" }),
  true,
  "compta peut éditer pendant le circuit",
);

assert.equal(
  canEditTravelsEffectif({ ...base, isCompta: true, status: "ANNULE" }),
  false,
  "compta ne peut pas éditer un dossier annulé",
);

assert.equal(
  canEditTravelsEffectif({ ...base, status: "VALIDE" }),
  false,
  "sans rôle métier : pas d’édition",
);

assert.equal(
  canEditTravelsDates({
    isOwner: false,
    isDirection: false,
    isAdministratif: false,
    isGlobalAdmin: false,
    status: "VALIDE",
  }),
  false,
  "dates : pas d’édition sans rôle opérationnel",
);

assert.equal(
  canEditTravelsDates({
    isOwner: true,
    isDirection: false,
    isAdministratif: false,
    isGlobalAdmin: false,
    status: "VALIDE",
  }),
  true,
  "créateur peut éditer les dates",
);

console.log("travels-edit-permissions.test.ts: ok");
