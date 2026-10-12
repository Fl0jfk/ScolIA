import assert from "node:assert/strict";
import { getTripNextGuidance } from "@/app/lib/travels-next-guidance";
import type { TravelsTrip } from "@/app/lib/travels-types";

const baseTrip = {
  id: "t1",
  type: "SIMPLE",
  status: "FINALISE_DIR_ATTENTE_ELEVES",
  data: {
    title: "Sortie test",
    piqueNiqueDetails: { active: true },
  },
} as TravelsTrip;

{
  const g = getTripNextGuidance(baseTrip, {
    isOwner: false,
    canSign: false,
    isCompta: false,
    isGlobalAdmin: true,
  });
  assert.ok(g, "guidance attendue");
  assert.equal(g!.youMustAct, true, "admin général doit pouvoir agir sur liste + repas");
  assert.equal(g!.ctaTab, "eleves");
  assert.match(g!.headline, /liste des élèves/i);
}

{
  const g = getTripNextGuidance(baseTrip, {
    isOwner: false,
    canSign: false,
    isCompta: false,
    isAdministratif: true,
  });
  assert.ok(g);
  assert.equal(g!.youMustAct, true, "administratif doit pouvoir confirmer");
}

{
  const g = getTripNextGuidance(baseTrip, {
    isOwner: false,
    canSign: false,
    isCompta: false,
  });
  assert.ok(g);
  assert.equal(g!.youMustAct, false, "sans rôle staff : pas d’action obligatoire");
}

console.log("travels-next-guidance-eleves.test.ts: ok");
