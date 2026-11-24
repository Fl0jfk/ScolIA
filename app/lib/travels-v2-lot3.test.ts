import { test } from "node:test";
import assert from "node:assert/strict";
import {
  applyDecision,
  applyObjection,
  canObject,
  canUploadAfterSignature,
  caseId,
  csvCell,
  docsMissingAlert,
  evaluateClaim,
  isRemindable,
  markRemindersSent,
  officialInes,
  planInvitationReminders,
  rowMatchesFilter,
  selectReminderTargets,
  studentState,
  toCsv,
  type FollowupRow,
  type StudentCase,
} from "@/app/lib/travels-v2-lot3";
import type { FamilyInvitation } from "@/app/lib/travels-v2-registration-types";

function inv(over: Partial<FamilyInvitation> & { id: string; ine: string }): FamilyInvitation {
  return {
    tripId: "trip12345",
    studentName: "Léa DUPONT",
    email: `${over.id}@x.fr`,
    createdAt: "2026-11-01T00:00:00Z",
    tokenHashes: [],
    sendCount: 1,
    firstSentAt: "2026-11-01T00:00:00Z",
    answers: {},
    documents: {},
    exemptions: {},
    codeSends: [],
    ...over,
  };
}
const yes = { decision: "oui" as const, signedAt: "2026-11-02T10:00:00Z", signerName: "Parent A" };
const no = { decision: "non" as const, signedAt: "2026-11-02T10:00:00Z", signerName: "Parent B" };

/* ─── Désaccord / suspens / décision ─── */

test("état de l'élève : inscrit, suspens, retiré, aucun", () => {
  const a = inv({ id: "AAAAAAAA", ine: "I1", signature: yes });
  const b = inv({ id: "BBBBBBBB", ine: "I1" });
  assert.equal(studentState([a, b], null), "inscrit");
  assert.equal(studentState([b], null), "aucun");
  const bObj = { ...b, objection: { at: "2026-11-03T10:00:00Z", signerName: "Parent B" } };
  assert.equal(studentState([a, bObj], null), "suspens");
  const base: StudentCase = { id: "I1", tripId: "trip12345", ine: "I1", history: [] };
  assert.equal(studentState([a, bObj], { ...base, decision: { kind: "maintenir", at: "x", by: "d" } }), "inscrit");
  assert.equal(studentState([a, bObj], { ...base, decision: { kind: "retirer", at: "x", by: "d" } }), "retire");
});

test("l'élève en suspens n'est pas dans la liste officielle", () => {
  const a = inv({ id: "AAAAAAAA", ine: "I1", signature: yes });
  const bObj = inv({ id: "BBBBBBBB", ine: "I1", objection: { at: "t", signerName: "B" } });
  const c = inv({ id: "CCCCCCCC", ine: "I2", signature: yes });
  assert.deepEqual(officialInes([a, bObj, c], []).sort(), ["I2"]);
  const decided: StudentCase = { id: "I1", tripId: "trip12345", ine: "I1", history: [], decision: { kind: "maintenir", at: "t", by: "d" } };
  assert.deepEqual(officialInes([a, bObj, c], [decided]).sort(), ["I1", "I2"]);
  const removed: StudentCase = { ...decided, decision: { kind: "retirer", at: "t", by: "d" } };
  assert.deepEqual(officialInes([a, bObj, c], [removed]).sort(), ["I2"]);
});

test("canObject : il faut un « oui » de l'autre, pas de double désaccord, pas après décision", () => {
  const a = inv({ id: "AAAAAAAA", ine: "I1", signature: yes });
  const b = inv({ id: "BBBBBBBB", ine: "I1" });
  assert.equal(canObject({ inv: b, siblings: [a, b], caseRec: null }).ok, true);
  assert.equal(canObject({ inv: a, siblings: [a, b], caseRec: null }).ok, false, "le signataire ne s'oppose pas à lui-même");
  assert.equal(canObject({ inv: b, siblings: [b], caseRec: null }).ok, false, "personne n'a inscrit l'élève");
  const bNon = { ...b, signature: no };
  assert.equal(canObject({ inv: bNon, siblings: [a, bNon], caseRec: null }).ok, true, "un parent qui avait refusé peut s'opposer");
  const bObj = { ...b, objection: { at: "t", signerName: "B" } };
  assert.equal(canObject({ inv: bObj, siblings: [a, bObj], caseRec: null }).ok, false);
  const decided: StudentCase = { id: "I1", tripId: "trip12345", ine: "I1", history: [], decision: { kind: "maintenir", at: "t", by: "d" } };
  assert.equal(canObject({ inv: b, siblings: [a, b], caseRec: decided }).ok, false);
  const other = inv({ id: "DDDDDDDD", ine: "I2", signature: yes });
  assert.equal(canObject({ inv: b, siblings: [other, b], caseRec: null }).ok, false, "un autre élève ne compte pas");
});

test("décision de la direction : seulement en suspens, historisée", () => {
  const a = inv({ id: "AAAAAAAA", ine: "I1", signature: yes });
  const bObj = inv({ id: "BBBBBBBB", ine: "I1", objection: { at: "t", signerName: "B" } });
  const rec = applyObjection(null, { tripId: "trip12345", ine: "I1" }, "Parent B", "2026-11-03T10:00:00Z");
  assert.equal(rec.history.length, 1);
  assert.equal(rec.history[0].action, "objection");
  const res = applyDecision({ caseRec: rec, invs: [a, bObj], kind: "retirer", by: "Direction", atIso: "2026-11-04T08:00:00Z", note: "  RDV pris  " });
  assert.equal(res.ok, true);
  if (!res.ok) return;
  assert.equal(res.next.decision?.kind, "retirer");
  assert.equal(res.next.decision?.note, "RDV pris");
  assert.deepEqual(res.next.history.map((h) => h.action), ["objection", "retirer"]);
  // déjà tranché : refusé
  const again = applyDecision({ caseRec: res.next, invs: [a, bObj], kind: "maintenir", by: "D", atIso: "t" });
  assert.equal(again.ok, false);
  // pas de désaccord : refusé
  const none = applyDecision({ caseRec: null, invs: [a], kind: "maintenir", by: "D", atIso: "t" });
  assert.equal(none.ok, false);
});

/* ─── Unicité de l'engagement ─── */

test("jeton d'engagement : libre, repris, pris, périmé", () => {
  assert.equal(evaluateClaim(null, "A", false), "free");
  assert.equal(evaluateClaim({ invitationId: "A" }, "A", false), "mine");
  assert.equal(evaluateClaim({ invitationId: "B" }, "A", true), "taken");
  assert.equal(evaluateClaim({ invitationId: "B" }, "A", false), "stale");
});

test("deux « oui » simultanés : un seul obtient le jeton (simulation d'un INSERT ON CONFLICT DO NOTHING)", async () => {
  const store = new Map<string, string>();
  const insertIfAbsent = async (key: string, v: string) => {
    await Promise.resolve(); // entrelacement des deux appels
    if (store.has(key)) return false;
    store.set(key, v);
    return true;
  };
  const key = caseId("1234567890A");
  const results = await Promise.all([insertIfAbsent(key, "A"), insertIfAbsent(key, "B")]);
  assert.equal(results.filter(Boolean).length, 1);
  assert.equal(caseId(" ab-12/3 "), "AB123");
});

/* ─── Relances ─── */

const docs = [
  { id: "d1", kind: "avant_signature" as const, label: "Attestation" },
  { id: "d2", kind: "avant_depart" as const, label: "Assurance", dueDate: "2027-01-20" },
];

test("relances de date limite : J-3 puis J-1, rattrapage, jamais de doublon", () => {
  const i = inv({ id: "AAAAAAAA", ine: "I1" });
  const base = { inv: i, deadline: "2026-12-10", docs: [], windowOpen: true };
  assert.equal(planInvitationReminders({ ...base, todayIso: "2026-12-06" }), null, "J-4 : rien");
  const j3 = planInvitationReminders({ ...base, todayIso: "2026-12-07" });
  assert.deepEqual(j3?.keys, ["deadline_j3"]);
  assert.equal(j3?.deadline?.daysLeft, 3);
  // rattrapage : le serveur n'a tourné qu'à J-2
  assert.deepEqual(planInvitationReminders({ ...base, todayIso: "2026-12-08" })?.keys, ["deadline_j3"]);
  const j1 = planInvitationReminders({ ...base, todayIso: "2026-12-09" });
  assert.deepEqual(j1?.keys, ["deadline_j1"]);
  // idempotence : après envoi, rien le même jour ni plus tard pour la même clé
  const sent = { ...i, reminders: markRemindersSent(i, ["deadline_j3"], "2026-12-07") };
  assert.equal(planInvitationReminders({ ...base, inv: sent, todayIso: "2026-12-07" }), null, "même jour");
  assert.equal(planInvitationReminders({ ...base, inv: sent, todayIso: "2026-12-08" }), null, "J-3 déjà envoyé");
  assert.deepEqual(planInvitationReminders({ ...base, inv: sent, todayIso: "2026-12-09" })?.keys, ["deadline_j1"]);
  // après la date limite ou page fermée : rien
  assert.equal(planInvitationReminders({ ...base, todayIso: "2026-12-11" }), null);
  assert.equal(planInvitationReminders({ ...base, todayIso: "2026-12-09", windowOpen: false }), null);
});

test("relances : pas pour une famille ayant signé, ni sans lien envoyé; la prolongation décale", () => {
  const signed = inv({ id: "AAAAAAAA", ine: "I1", signature: yes });
  assert.equal(planInvitationReminders({ inv: signed, deadline: "2026-12-10", docs: [], todayIso: "2026-12-09", windowOpen: true }), null);
  const neverSent = inv({ id: "BBBBBBBB", ine: "I1", firstSentAt: undefined });
  assert.equal(planInvitationReminders({ inv: neverSent, deadline: "2026-12-10", docs: [], todayIso: "2026-12-09", windowOpen: true }), null);
  const ext = inv({ id: "CCCCCCCC", ine: "I1", extendedDeadline: "2026-12-20" });
  assert.equal(planInvitationReminders({ inv: ext, deadline: "2026-12-10", docs: [], todayIso: "2026-12-09", windowOpen: true }), null);
  assert.deepEqual(planInvitationReminders({ inv: ext, deadline: "2026-12-10", docs: [], todayIso: "2026-12-17", windowOpen: true })?.keys, ["deadline_j3"]);
});

test("pièces « avant départ » : relances J-7 / J-3 / J-1 de leur date, tant qu'elles manquent", () => {
  const i = inv({ id: "AAAAAAAA", ine: "I1", signature: yes });
  const at = (todayIso: string, v = i) => planInvitationReminders({ inv: v, deadline: "2026-12-10", docs, todayIso, windowOpen: false });
  assert.equal(at("2027-01-10"), null, "J-10");
  assert.deepEqual(at("2027-01-13")?.keys, ["doc_d2_j7"]);
  assert.deepEqual(at("2027-01-14")?.keys, ["doc_d2_j7"], "rattrapage à J-6");
  assert.deepEqual(at("2027-01-17")?.keys, ["doc_d2_j3"]);
  assert.deepEqual(at("2027-01-19")?.keys, ["doc_d2_j1"]);
  assert.equal(at("2027-01-21"), null, "date dépassée");
  const done = { ...i, documents: { d2: { fileName: "a.pdf", key: "k", size: 1, contentType: "application/pdf", uploadedAt: "t" } } };
  assert.equal(at("2027-01-19", done), null, "pièce déposée");
  const exempt = { ...i, exemptions: { d2: { at: "t", by: "o" } } };
  assert.equal(at("2027-01-19", exempt), null, "pièce dispensée");
  const sent = { ...i, reminders: markRemindersSent(i, ["doc_d2_j7"], "2027-01-13") };
  assert.equal(at("2027-01-13", sent), null, "idempotent le même jour");
  assert.equal(at("2027-01-14", sent), null, "J-7 déjà envoyé");
});

test("alerte organisateur à J-7 du départ : pièces avant départ manquantes des inscrits seulement", () => {
  const a = inv({ id: "AAAAAAAA", ine: "I1", studentName: "A", signature: yes });
  const b = inv({ id: "BBBBBBBB", ine: "I2", studentName: "B", signature: yes, documents: { d2: { fileName: "a", key: "k", size: 1, contentType: "x", uploadedAt: "t" } } });
  const c = inv({ id: "CCCCCCCC", ine: "I3", studentName: "C", signature: yes });
  const d = inv({ id: "DDDDDDDD", ine: "I4", studentName: "D" });
  const args = { docs, invs: [a, b, c, d], officialIneList: ["I1", "I2", "I3"] };
  assert.equal(docsMissingAlert({ ...args, departureIso: "2027-02-01", todayIso: "2027-01-20" }).length, 0, "J-12");
  const out = docsMissingAlert({ ...args, departureIso: "2027-01-27", todayIso: "2027-01-20" });
  assert.deepEqual(out.map((o) => o.ine), ["I1", "I3"]);
  assert.deepEqual(out[0].missing, ["Assurance"]);
  assert.equal(docsMissingAlert({ ...args, departureIso: "2027-01-27", todayIso: "2027-01-28" }).length, 0, "départ passé");
  assert.equal(docsMissingAlert({ ...args, officialIneList: ["I2"], departureIso: "2027-01-27", todayIso: "2027-01-20" }).length, 0, "suspens/retirés exclus");
});

test("dépôt après signature : seulement les pièces « avant départ », avant leur date", () => {
  const signed = { signature: yes };
  assert.equal(canUploadAfterSignature({ kind: "avant_depart", dueDate: "2027-01-20" }, signed, "2027-01-20"), true);
  assert.equal(canUploadAfterSignature({ kind: "avant_depart", dueDate: "2027-01-20" }, signed, "2027-01-21"), false);
  assert.equal(canUploadAfterSignature({ kind: "avant_signature" }, signed, "2027-01-10"), false);
  assert.equal(canUploadAfterSignature({ kind: "avant_depart" }, {}, "2027-01-10"), false, "pas signé « oui »");
  assert.equal(canUploadAfterSignature({ kind: "avant_depart" }, { signature: no }, "2027-01-10"), false);
});

/* ─── Suivi : filtres, ciblage, export ─── */

const mkRow = (over: Partial<FollowupRow>): FollowupRow => ({
  status: "pas_ouvert",
  exempted: false,
  paper: false,
  invitationId: "AAAAAAAA",
  signed: false,
  ...over,
});

test("filtres du suivi, y compris dispensé et papier", () => {
  assert.equal(rowMatchesFilter(mkRow({ status: "bloque_piece" }), "bloque_piece"), true);
  assert.equal(rowMatchesFilter(mkRow({ status: "pret", exempted: true }), "dispense"), true);
  assert.equal(rowMatchesFilter(mkRow({ status: "signe_oui", paper: true, signed: true }), "papier"), true);
  assert.equal(rowMatchesFilter(mkRow({ status: "pret" }), "papier"), false);
  assert.equal(rowMatchesFilter(mkRow({}), "all"), true);
});

test("relance ciblée : une famille ou tous les « en cours », jamais signées / suspens / sans e-mail", () => {
  const rows = [
    mkRow({ invitationId: "I1", status: "en_cours" }),
    mkRow({ invitationId: "I2", status: "en_cours" }),
    mkRow({ invitationId: "I3", status: "signe_oui", signed: true }),
    mkRow({ invitationId: "I4", status: "suspens" }),
    mkRow({ invitationId: null, status: "sans_email" }),
    mkRow({ invitationId: "I6", status: "pas_ouvert" }),
  ];
  assert.deepEqual(selectReminderTargets(rows, { status: "en_cours" }).map((r) => r.invitationId), ["I1", "I2"]);
  assert.deepEqual(selectReminderTargets(rows, { invitationId: "I6" }).map((r) => r.invitationId), ["I6"]);
  assert.deepEqual(selectReminderTargets(rows, { invitationId: "I3" }), [], "déjà signée");
  assert.deepEqual(selectReminderTargets(rows, { status: "suspens" }), [], "suspens non relançable");
  assert.equal(isRemindable(rows[4]), false);
});

test("export CSV : séparateur, guillemets, formules neutralisées", () => {
  assert.equal(csvCell('a"b'), '"a""b"');
  assert.equal(csvCell("=SOMME(A1)"), `"'=SOMME(A1)"`);
  assert.equal(csvCell("@cmd"), `"'@cmd"`);
  assert.equal(csvCell(null), '""');
  const csv = toCsv(["Élève", "Statut"], [["Léa", "Signé"]]);
  assert.ok(csv.startsWith("﻿"));
  assert.ok(csv.includes('"Élève";"Statut"\r\n"Léa";"Signé"'));
});
