import { test } from "node:test";
import assert from "node:assert/strict";
import { checkV2StatusTransition, guardV2Update, v2RoleFor, type V2Role } from "@/app/lib/travels-v2-update-guard";

const direction = { isDirectionOfEtab: true, isGlobalAdmin: false };
const nobody = { isDirectionOfEtab: false, isGlobalAdmin: false };

const priceBlock = { pricePerStudent: 120, firstMonth: "2026-12", lastMonth: "2027-07", paymentText: "" };
const docs = [{ id: "d1", label: "Autorisation de sortie", kind: "avant_signature" }];

function openedData(extra: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    workflowVersion: 2,
    startDate: "2027-03-20",
    endDate: "2027-03-24",
    familyPage: { documents: docs, deadline: "2027-02-20", openedAt: "2027-01-10T08:00:00Z", openedBy: "Direction" },
    priceBlock,
    ...extra,
  };
}

function guard(over: Partial<Parameters<typeof guardV2Update>[0]> & { incomingData: Record<string, unknown> }) {
  return guardV2Update({
    existingData: openedData(),
    previousStatus: "INSCRIPTIONS_OUVERTES",
    newStatus: "INSCRIPTIONS_OUVERTES",
    role: "writer",
    actor: nobody,
    ...over,
  });
}

/* ─── Rôles ─── */

test("B1 rôle : organisateur / direction / admin / administratif écrivent, la compta seule non, un inconnu rien", () => {
  const base = { isOwner: false, isDirection: false, isAdmin: false, isAdministratif: false, isCompta: false };
  assert.equal(v2RoleFor({ ...base, isOwner: true }), "writer");
  assert.equal(v2RoleFor({ ...base, isDirection: true }), "writer");
  assert.equal(v2RoleFor({ ...base, isAdmin: true }), "writer");
  assert.equal(v2RoleFor({ ...base, isAdministratif: true }), "writer");
  assert.equal(v2RoleFor({ ...base, isCompta: true }), "compta");
  assert.equal(v2RoleFor({ ...base, isCompta: true, isOwner: true }), "writer");
  assert.equal(v2RoleFor(base), "none");
});

test("B1 un utilisateur sans droit est refusé (403) sur un voyage v2 existant", () => {
  const r = guard({ role: "none", incomingData: openedData() });
  assert.equal(r.ok, false);
  if (!r.ok) assert.equal(r.status, 403);
});

test("B1 la compta seule ne garde que ses champs : le reste vient du serveur", () => {
  const existing = openedData({ title: "Rome", comptaSheet: { a: 1 } });
  const r = guardV2Update({
    existingData: existing,
    previousStatus: "EN_ATTENTE_COMPTA",
    newStatus: "EN_ATTENTE_DIR_FINAL",
    role: "compta",
    actor: nobody,
    incomingData: { ...openedData({ title: "PIRATÉ", destination: "Ailleurs" }), comptaSheet: { a: 2 }, finalTotalCost: 999 },
  });
  // page ouverte + compta : seul ce qui est dans la liste compta passe ; le prix est de toute façon figé (ouverte)
  assert.equal(r.ok, true);
  if (r.ok) {
    assert.equal(r.data.title, "Rome");
    assert.equal(r.data.destination, undefined);
    assert.deepEqual(r.data.comptaSheet, { a: 2 });
    assert.equal(r.data.finalTotalCost, 999);
  }
});

/* ─── Champs pilotés par le serveur ─── */

test("B1 v2Closure, v2Purge, liste élèves, ouverture et marqueur transport sont réinjectés depuis l'existant", () => {
  const existing = openedData({
    v2Closure: { closedAt: "2027-02-21T00:00:00Z", mode: "auto", officialCount: 20 },
    v2Purge: { at: "x", invitationsRemoved: 1, filesRemoved: 1, engagementsKept: 1 },
    listeElevesStatus: "confirmed",
    listeElevesConfirmedAt: "2027-02-21T00:00:00Z",
    listeElevesConfirmedBy: { userId: "u", name: "Système" },
    participantEleves: [{ ine: "I1" }],
    nbEleves: 1,
    classes: "3A",
    transportRequestSentAt: "2027-01-02T00:00:00Z",
  });
  const r = guard({
    existingData: existing,
    previousStatus: "VALIDE",
    newStatus: "VALIDE",
    incomingData: {
      ...openedData(),
      v2Closure: undefined,
      v2Purge: undefined,
      v2Autre: "pirate",
      listeElevesStatus: "draft",
      participantEleves: [],
      nbEleves: 99,
      classes: "TOUTES",
      transportRequestSentAt: undefined,
      familyPage: { documents: docs, deadline: "2027-02-20", openedAt: "2020-01-01T00:00:00Z", openedBy: "Moi" },
    },
  });
  assert.equal(r.ok, true);
  if (!r.ok) return;
  assert.deepEqual(r.data.v2Closure, existing.v2Closure);
  assert.deepEqual(r.data.v2Purge, existing.v2Purge);
  assert.equal(r.data.v2Autre, undefined);
  assert.equal(r.data.listeElevesStatus, "confirmed");
  assert.deepEqual(r.data.participantEleves, [{ ine: "I1" }]);
  assert.equal(r.data.nbEleves, 1);
  assert.equal(r.data.classes, "3A");
  assert.equal(r.data.transportRequestSentAt, "2027-01-02T00:00:00Z");
  const page = r.data.familyPage as Record<string, unknown>;
  assert.equal(page.openedAt, "2027-01-10T08:00:00Z");
  assert.equal(page.openedBy, "Direction");
});

test("B1 un client ne peut pas poser v2*, ouverture ou marqueur transport à la création", () => {
  const r = guardV2Update({
    existingData: null,
    previousStatus: null,
    newStatus: "EN_ATTENTE_DIR_INITIAL",
    role: "writer",
    actor: nobody,
    incomingData: {
      workflowVersion: 2,
      v2Closure: { closedAt: "x" },
      transportRequestSentAt: "2027-01-01T00:00:00Z",
      listeElevesStatus: "confirmed",
      familyPage: { documents: [], openedAt: "2027-01-01T00:00:00Z" },
    },
  });
  assert.equal(r.ok, true);
  if (!r.ok) return;
  assert.equal(r.data.v2Closure, undefined);
  assert.equal(r.data.transportRequestSentAt, undefined);
  assert.equal(r.data.listeElevesStatus, undefined);
  assert.equal((r.data.familyPage as Record<string, unknown>).openedAt, undefined);
});

test("B1 une fenêtre sans familyPage ne fait pas disparaître la page ouverte", () => {
  const { familyPage: _omit, ...withoutPage } = openedData();
  void _omit;
  const r = guard({ incomingData: withoutPage });
  assert.equal(r.ok, true);
  if (r.ok) assert.equal((r.data.familyPage as Record<string, unknown>).openedAt, "2027-01-10T08:00:00Z");
});

/* ─── Transitions de statut ─── */

test("B1 transitions : un enregistrement périmé ne rouvre ni ne rétrograde jamais un voyage", () => {
  const roles: V2Role[] = ["writer"];
  for (const role of roles) {
    // rétrogradation d'un voyage ouvert
    for (const next of ["EN_ATTENTE_DIR_FINAL", "EN_ATTENTE_COMPTA", "BESOIN_MODIFICATION", "EN_ATTENTE_DIR_INITIAL", "VALIDE"]) {
      const r = checkV2StatusTransition({ previous: "INSCRIPTIONS_OUVERTES", next, role, actor: direction });
      assert.equal(r.ok, false, `INSCRIPTIONS_OUVERTES -> ${next}`);
    }
    // réouverture d'un voyage clôturé / terminé
    for (const prev of ["VALIDE", "FINALISE_DIR_ATTENTE_ELEVES"]) {
      for (const next of ["INSCRIPTIONS_OUVERTES", "EN_ATTENTE_DIR_FINAL", "EN_ATTENTE_COMPTA", "BESOIN_MODIFICATION"]) {
        assert.equal(checkV2StatusTransition({ previous: prev, next, role, actor: direction }).ok, false, `${prev} -> ${next}`);
      }
    }
    // voyages annulés / refusés : clos
    for (const prev of ["ANNULE", "SEANCE_ANNULEE", "REJETE"]) {
      assert.equal(checkV2StatusTransition({ previous: prev, next: "EN_ATTENTE_DIR_INITIAL", role, actor: direction }).ok, false);
    }
  }
});

test("B1 transitions : VALIDE seulement par la clôture ; ouverture seulement par la direction/admin depuis la validation finale", () => {
  assert.equal(checkV2StatusTransition({ previous: "EN_ATTENTE_DIR_FINAL", next: "VALIDE", role: "writer", actor: direction }).ok, false);
  assert.equal(checkV2StatusTransition({ previous: "EN_ATTENTE_COMPTA", next: "INSCRIPTIONS_OUVERTES", role: "writer", actor: direction }).ok, false);
  const orga = checkV2StatusTransition({ previous: "EN_ATTENTE_DIR_FINAL", next: "INSCRIPTIONS_OUVERTES", role: "writer", actor: nobody });
  assert.equal(orga.ok, false);
  if (!orga.ok) assert.equal(orga.status, 403);
  assert.equal(checkV2StatusTransition({ previous: "EN_ATTENTE_DIR_FINAL", next: "INSCRIPTIONS_OUVERTES", role: "writer", actor: direction }).ok, true);
  assert.equal(
    checkV2StatusTransition({ previous: "EN_ATTENTE_DIR_FINAL", next: "INSCRIPTIONS_OUVERTES", role: "writer", actor: { isDirectionOfEtab: false, isGlobalAdmin: true } }).ok,
    true,
  );
});

test("B1 transitions : le circuit amont circule librement, l'annulation reste possible", () => {
  for (const [p, n] of [
    ["EN_ATTENTE_DIR_INITIAL", "PROF_LOGISTICS"],
    ["PROF_LOGISTICS", "EN_ATTENTE_BUS_SIGNATURE"],
    ["EN_ATTENTE_BUS_SIGNATURE", "EN_ATTENTE_COMPTA"],
    ["EN_ATTENTE_COMPTA", "EN_ATTENTE_DIR_FINAL"],
    ["EN_ATTENTE_DIR_FINAL", "BESOIN_MODIFICATION"],
    ["BESOIN_MODIFICATION", "EN_ATTENTE_DIR_INITIAL"],
    ["EN_ATTENTE_DIR_INITIAL", "REJETE"],
    ["INSCRIPTIONS_OUVERTES", "ANNULE"],
    ["VALIDE", "ANNULE"],
  ] as const) {
    assert.equal(checkV2StatusTransition({ previous: p, next: n, role: "writer", actor: nobody }).ok, true, `${p} -> ${n}`);
  }
  assert.equal(checkV2StatusTransition({ previous: null, next: "VALIDE", role: "writer", actor: nobody }).ok, false, "création directement finalisée");
});

test("B1 transitions : la compta seule ne valide que les finances", () => {
  assert.equal(checkV2StatusTransition({ previous: "EN_ATTENTE_COMPTA", next: "EN_ATTENTE_DIR_FINAL", role: "compta", actor: nobody }).ok, true);
  assert.equal(checkV2StatusTransition({ previous: "EN_ATTENTE_COMPTA", next: "BESOIN_MODIFICATION", role: "compta", actor: nobody }).ok, true);
  assert.equal(checkV2StatusTransition({ previous: "EN_ATTENTE_DIR_INITIAL", next: "PROF_LOGISTICS", role: "compta", actor: nobody }).ok, false);
  assert.equal(checkV2StatusTransition({ previous: "EN_ATTENTE_DIR_FINAL", next: "INSCRIPTIONS_OUVERTES", role: "compta", actor: direction }).ok, false);
});

/* ─── Gel après ouverture ─── */

test("B1 après ouverture : pièces et prix figés", () => {
  const changedDocs = guard({
    incomingData: openedData({ familyPage: { documents: [...docs, { id: "d2", label: "Nouvelle pièce", kind: "facultative" }], deadline: "2027-02-20", openedAt: "x" } }),
  });
  assert.equal(changedDocs.ok, false);
  if (!changedDocs.ok) assert.equal(changedDocs.status, 403);

  const changedPrice = guard({ incomingData: openedData({ priceBlock: { ...priceBlock, pricePerStudent: 50 } }) });
  assert.equal(changedPrice.ok, false);
  if (!changedPrice.ok) assert.equal(changedPrice.status, 403);

  const untouched = guard({ incomingData: openedData() });
  assert.equal(untouched.ok, true);
});

test("B1 après ouverture : la date limite ne peut que reculer, plafonnée à J-14", () => {
  const earlier = guard({ incomingData: openedData({ familyPage: { documents: docs, deadline: "2027-02-10" } }) });
  assert.equal(earlier.ok, false);
  if (!earlier.ok) assert.equal(earlier.status, 400);

  const later = guard({ incomingData: openedData({ familyPage: { documents: docs, deadline: "2027-03-01" } }) });
  assert.equal(later.ok, true);
  if (later.ok) assert.equal((later.data.familyPage as Record<string, unknown>).deadline, "2027-03-01");

  // J-14 du départ (2027-03-20) = 2027-03-06
  const tooLate = guard({ incomingData: openedData({ familyPage: { documents: docs, deadline: "2027-03-07" } }) });
  assert.equal(tooLate.ok, false);
  const exactCap = guard({ incomingData: openedData({ familyPage: { documents: docs, deadline: "2027-03-06" } }) });
  assert.equal(exactCap.ok, true);

  const invalid = guard({ incomingData: openedData({ familyPage: { documents: docs, deadline: "n'importe quoi" } }) });
  assert.equal(invalid.ok, false);

  const missing = guard({ incomingData: openedData({ familyPage: { documents: docs } }) });
  assert.equal(missing.ok, true);
  if (missing.ok) assert.equal((missing.data.familyPage as Record<string, unknown>).deadline, "2027-02-20");
});

test("B1 avant ouverture : l'organisateur modifie librement pièces, prix et date limite", () => {
  const existing = { workflowVersion: 2, startDate: "2027-03-20", familyPage: { documents: docs, deadline: "2027-02-20" }, priceBlock };
  const r = guardV2Update({
    existingData: existing,
    previousStatus: "EN_ATTENTE_COMPTA",
    newStatus: "EN_ATTENTE_COMPTA",
    role: "writer",
    actor: nobody,
    incomingData: { ...existing, familyPage: { documents: [], deadline: "2027-01-01", openedAt: "2027-01-01T00:00:00Z" } },
  });
  assert.equal(r.ok, true);
  if (r.ok) {
    const page = r.data.familyPage as Record<string, unknown>;
    assert.deepEqual(page.documents, []);
    assert.equal(page.deadline, "2027-01-01");
    assert.equal(page.openedAt, undefined, "l'ouverture ne se pose jamais depuis le client");
  }
});

/* ─── Séries récurrentes et v1 ─── */

test("B1 une série récurrente est refusée en v2 (à la création comme à la modification)", () => {
  const created = guardV2Update({
    existingData: null,
    previousStatus: null,
    newStatus: "EN_ATTENTE_DIR_INITIAL",
    role: "writer",
    actor: nobody,
    incomingData: { workflowVersion: 2, recurrenceSeriesId: "serie-1" },
  });
  assert.equal(created.ok, false);
  if (!created.ok) assert.equal(created.status, 400);
  const updated = guard({ incomingData: openedData({ recurrenceSeriesId: "serie-1" }) });
  assert.equal(updated.ok, false);
});

test("B1 le circuit v1 n'est pas concerné : la garde n'est appelée que pour workflowVersion === 2 (contrat de la route)", async () => {
  // Contrat vérifié sur le source de la route : isV2Guarded = existant ? existant.workflowVersion === 2 : entrant.workflowVersion === 2.
  const { readFileSync } = await import("node:fs");
  const src = readFileSync("app/api/travels/update/route.ts", "utf8");
  assert.match(src, /const isV2Guarded = existingOnS3 \? existingInner\.workflowVersion === 2 : saveInner\.workflowVersion === 2;/);
  assert.match(src, /if \(isV2Guarded\) \{/);
});
