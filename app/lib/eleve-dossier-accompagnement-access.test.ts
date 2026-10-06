import assert from "node:assert/strict";
import test from "node:test";
import {
  brainAccompagnementExposure,
  eleveDossierAccompagnementCacheScopeSuffix,
  filterAccompagnementAlertsForViewer,
  viewerMayLoadEleveAccompagnementListMetadata,
  viewerMayReceiveAccompagnementDashboardAlerts,
  viewerMayReceiveAccompagnementKind,
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

const paiDoc = {
  tiroir: "sante" as const,
  confidentialite: "standard" as const,
  title: "PAI — Projet d'accueil individualisé",
};

const ppsDoc = {
  tiroir: "sante" as const,
  confidentialite: "standard" as const,
  title: "PPS",
};

const gevascoDoc = {
  tiroir: "sante" as const,
  confidentialite: "standard" as const,
  title: "GEVASCO",
};

const profInternat = { roles: ["professeur", "internat"] };
const ctx = { eleveClasse: "4B" };

test("professeur + internat — PAI uniquement (pas tiroir santé complet)", () => {
  assert.equal(viewerMayLoadEleveAccompagnementListMetadata(profInternat), true);
  assert.equal(viewerMayReceiveEleveAccompagnementMetadata(profInternat, ctx), true);
  assert.equal(viewerMayReceiveAccompagnementKind(profInternat, "pai", ctx), true);
  assert.equal(viewerMayReceiveAccompagnementKind(profInternat, "pap", ctx), false);
  assert.equal(viewerMayReceiveAccompagnementKind(profInternat, "pps", ctx), false);
  assert.equal(viewerMayReceiveAccompagnementKind(profInternat, "gevasco", ctx), false);
  assert.equal(eleveDossierAccompagnementCacheScopeSuffix(profInternat), "accomp:pai");
  assert.equal(eleveDocTiroirsForRoles(["professeur", "internat"]).has("sante"), false);
  assert.equal(
    canOpenDocumentWithoutGrant(paiDoc, ["professeur", "internat"], {
      accompagnementEleveContext: ctx,
    }),
    true,
  );
  assert.equal(
    canOpenDocumentWithoutGrant(papDoc, ["professeur", "internat"], {
      accompagnementEleveContext: ctx,
    }),
    false,
  );
});

test("professeur + internat — brain et alertes : PAI seulement", () => {
  const items = [
    { kind: "pai" as const, documentId: "d-pai" },
    { kind: "pap" as const, documentId: "d-pap" },
    { kind: "pps" as const, documentId: "d-pps" },
  ];
  const exposure = brainAccompagnementExposure(profInternat, "4B", items);
  assert.deepEqual(exposure.kinds, ["pai"]);
  assert.equal(exposure.items.length, 1);
  assert.equal(exposure.items[0]?.documentId, "d-pai");

  assert.equal(viewerMayReceiveAccompagnementDashboardAlerts(profInternat), true);
  const alerts = filterAccompagnementAlertsForViewer(profInternat, [
    { classe: "4B", documentId: "a1", kind: "pai" },
    { classe: "4B", documentId: "a2", kind: "pap" },
  ]);
  assert.equal(alerts.length, 1);
  assert.equal(alerts[0]?.kind, "pai");
});

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
  assert.equal(
    canOpenDocumentWithoutGrant(ppsDoc, ["professeur"], {
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
  assert.equal(viewerMayReceiveAccompagnementKind({ roles }, "pap", { eleveClasse: "3A" }), true);
  assert.equal(
    canOpenDocumentWithoutGrant(papDoc, roles, {
      accompagnementEleveContext: { eleveClasse: "3A" },
    }),
    true,
  );
  assert.equal(
    canOpenDocumentWithoutGrant(gevascoDoc, roles, {
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
    { classe: "4B", documentId: "d1", kind: "pai" },
  ]);
  assert.equal(alerts.length, 0);
});

test("dashboard — prof + direction conserve les alertes", () => {
  const viewer = { roles: ["professeur", "direction"] };
  assert.equal(viewerMayReceiveAccompagnementDashboardAlerts(viewer), true);
  const alerts = filterAccompagnementAlertsForViewer(viewer, [
    { classe: "4B", documentId: "d1", kind: "pap" },
  ]);
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
