import assert from "node:assert/strict";
import test from "node:test";
import {
  eleveDossierAccompagnementCacheScopeSuffix,
  viewerMayLoadEleveAccompagnementListMetadata,
  viewerMayReceiveEleveAccompagnementMetadata,
} from "@/app/lib/eleve-dossier-accompagnement-access";
import { canOpenDocumentWithoutGrant } from "@/app/lib/eleve-dossier-access";

const papDoc = {
  tiroir: "sante" as const,
  confidentialite: "standard" as const,
  title: "Plan d'accompagnement personnalisé",
};

test("professeur seul — pas de métadonnées accompagnement (interrupteur false)", () => {
  const viewer = { roles: ["professeur"] };
  assert.equal(viewerMayLoadEleveAccompagnementListMetadata(viewer), false);
  assert.equal(
    viewerMayReceiveEleveAccompagnementMetadata(viewer, {
      eleveClasse: "4B",
      assignedClasses: ["4B"],
    }),
    false,
  );
  assert.equal(
    canOpenDocumentWithoutGrant(papDoc, ["professeur"], {
      accompagnementEleveContext: { eleveClasse: "4B", assignedClasses: ["4B"] },
    }),
    false,
  );
  assert.equal(eleveDossierAccompagnementCacheScopeSuffix(viewer), "accomp:none");
});

test("professeur + CPE — accès accompagnement conservé", () => {
  const roles = ["professeur", "cpe"];
  assert.equal(
    viewerMayReceiveEleveAccompagnementMetadata({ roles }, { eleveClasse: "3A" }),
    true,
  );
  assert.equal(
    canOpenDocumentWithoutGrant(papDoc, roles, {
      accompagnementEleveContext: { eleveClasse: "3A" },
    }),
    true,
  );
});

test("direction et infirmerie — ouverture PAP inchangée", () => {
  assert.equal(
    canOpenDocumentWithoutGrant(papDoc, ["direction"], {}),
    true,
  );
  assert.equal(
    canOpenDocumentWithoutGrant(papDoc, ["direction_college"], {}),
    true,
  );
  assert.equal(
    canOpenDocumentWithoutGrant(papDoc, ["infirmerie"], {}),
    true,
  );
});

test("interrupteur true — prof : uniquement classes affectées", () => {
  const viewer = { roles: ["professeur"] };
  assert.equal(
    viewerMayReceiveEleveAccompagnementMetadata(
      viewer,
      { eleveClasse: "4B", assignedClasses: ["4B", "4B1"] },
      true,
    ),
    true,
  );
  assert.equal(
    viewerMayReceiveEleveAccompagnementMetadata(
      viewer,
      { eleveClasse: "3A", assignedClasses: ["4B"] },
      true,
    ),
    false,
  );
  assert.equal(viewerMayLoadEleveAccompagnementListMetadata(viewer, true), true);
});
