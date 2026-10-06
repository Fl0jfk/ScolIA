import assert from "node:assert/strict";
import test from "node:test";
import {
  brainAccompagnementExposure,
  eleveDossierAccompagnementCacheScopeSuffix,
  filterAccompagnementAlertsForViewer,
  viewerMayLoadEleveAccompagnementListMetadata,
  viewerMayReceiveAccompagnementDashboardAlerts,
  viewerMayReceiveEleveAccompagnementMetadata,
} from "@/app/lib/eleve-dossier-accompagnement-access";
import {
  canOpenDocumentWithoutGrant,
  eleveDocTiroirsForRoles,
} from "@/app/lib/eleve-dossier-access";

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
  assert.equal(eleveDocTiroirsForRoles(roles).has("sante"), true);
});

test("professeur + comptabilité — tiroir santé pour liste documents", () => {
  const roles = ["professeur", "comptabilite"];
  assert.equal(eleveDocTiroirsForRoles(roles).has("sante"), true);
});

test("dashboard — prof seul sans alertes accompagnement", () => {
  const viewer = { roles: ["professeur"] };
  assert.equal(viewerMayReceiveAccompagnementDashboardAlerts(viewer), false);
  const alerts = filterAccompagnementAlertsForViewer(viewer, [
    { classe: "4B", documentId: "d1" },
  ] as Array<{ classe: string | null; documentId: string }>);
  assert.equal(alerts.length, 0);
});

test("dashboard — prof + direction conserve les alertes", () => {
  const viewer = { roles: ["professeur", "direction"] };
  assert.equal(viewerMayReceiveAccompagnementDashboardAlerts(viewer), true);
  const alerts = filterAccompagnementAlertsForViewer(viewer, [
    { classe: "4B", documentId: "d1" },
  ] as Array<{ classe: string | null; documentId: string }>);
  assert.equal(alerts.length, 1);
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

test("brain — prof seul sans PAP ni lien document", () => {
  const viewer = { roles: ["professeur"] };
  const exposure = brainAccompagnementExposure(viewer, "4B", [
    { kind: "pap", documentId: "doc-1" },
    { kind: "pps", documentId: "doc-2" },
  ]);
  assert.deepEqual(exposure.kinds, []);
  assert.equal(exposure.items.length, 0);
});

test("brain — prof + direction conserve les dispositifs", () => {
  const viewer = { roles: ["professeur", "direction"] };
  const exposure = brainAccompagnementExposure(viewer, "4B", [
    { kind: "pap", documentId: "doc-1" },
  ]);
  assert.deepEqual(exposure.kinds, ["pap"]);
  assert.equal(exposure.items.length, 1);
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
