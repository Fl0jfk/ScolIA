import { test } from "node:test";
import assert from "node:assert/strict";
import { resolveCashDraft, validateFamilyStructured } from "@/app/lib/travels-v2";
import {
  applyMealsToCuisineDetails,
  archiveInvitation,
  buildClosureLists,
  buildComptaRows,
  canCloseManually,
  closureDueDate,
  comptaCsv,
  computeMealsPerDay,
  decideClosureAction,
  engagementKeys,
  evaluateClosure,
  evaluateRectifiedQuote,
  isPurgeDue,
  planPurge,
  sanitizeAdultMeals,
} from "@/app/lib/travels-v2-lot4";
import { studentState, type StudentCase } from "@/app/lib/travels-v2-lot3";
import type { FamilyInvitation } from "@/app/lib/travels-v2-registration-types";

function inv(over: Partial<FamilyInvitation> & { id: string; ine: string }): FamilyInvitation {
  return {
    tripId: "trip12345",
    studentName: "Léa DUPONT",
    classe: "3A",
    email: `${over.id}@x.fr`,
    createdAt: "2026-11-01T00:00:00Z",
    tokenHashes: ["h1"],
    sendCount: 1,
    firstSentAt: "2026-11-01T00:00:00Z",
    answers: {},
    documents: {},
    exemptions: {},
    codeSends: [],
    ...over,
  };
}
const sigOnline = (key: string) => ({
  decision: "oui" as const,
  signedAt: "2026-11-02T10:00:00Z",
  signerName: "Parent A",
  signatureKey: `sig-${key}.png`,
  pdfKey: `eng-${key}.pdf`,
  ipHash: "abc",
});
const sigNo = { decision: "non" as const, signedAt: "2026-11-02T10:00:00Z", signerName: "Parent B" };
const engagement = (id: string, ine: string, name: string, amount = 90) => ({
  ine,
  studentName: name,
  signerName: "Parent A",
  invitationId: id,
  amount,
  firstMonth: "2027-01",
  lastMonth: "2027-06",
  months: [{ label: "janvier 2027", amount: 15 }],
  signedAt: "2026-11-02T10:00:00Z",
  pdfKey: `eng-${id}.pdf`,
});
const NO_CASES: StudentCase[] = [];

/* ─── Liste officielle ─── */

test("liste officielle : signé, papier, suspens, retiré, désisté, refus", () => {
  const a = inv({ id: "AAAAAAAA", ine: "I1", studentName: "Anna A", signature: sigOnline("AAAAAAAA"), engagement: engagement("AAAAAAAA", "I1", "Anna A") });
  const b = inv({
    id: "BBBBBBBB",
    ine: "I2",
    studentName: "Bob B",
    signature: {
      decision: "oui",
      signedAt: "2026-11-03T10:00:00Z",
      signerName: "Parent B",
      pdfKey: "scan-b.jpg",
      paper: { recordedBy: "Prof", recordedAt: "2026-11-03T10:00:00Z", scanFileName: "coupon.jpg", scanKey: "scan-b.jpg" },
    },
  });
  const c = inv({ id: "CCCCCCCC", ine: "I3", studentName: "Cléo C", signature: sigOnline("CCCCCCCC") });
  const cObj = inv({ id: "CCCCCCC2", ine: "I3", studentName: "Cléo C", objection: { at: "t", signerName: "Parent C2" } });
  const d = inv({ id: "DDDDDDDD", ine: "I4", studentName: "Dan D", signature: sigOnline("DDDDDDDD") });
  const e = inv({ id: "EEEEEEEE", ine: "I5", studentName: "Eve E", signature: sigOnline("EEEEEEEE"), withdrawal: { at: "t", by: "Prof" } });
  const f = inv({ id: "FFFFFFFF", ine: "I6", studentName: "Fay F", signature: sigNo });
  const g = inv({ id: "GGGGGGGG", ine: "I7", studentName: "Gus G" });
  const cases: StudentCase[] = [
    { id: "I4", tripId: "trip12345", ine: "I4", history: [], decision: { kind: "retirer", at: "t", by: "Dir" } },
  ];
  const lists = buildClosureLists([a, b, c, cObj, d, e, f, g], cases);
  assert.deepEqual(lists.official.map((o) => o.ine), ["I1", "I2"]);
  assert.deepEqual(lists.suspended, ["I3"]);
  assert.deepEqual(lists.removed.map((o) => o.ine), ["I4"]);
  assert.deepEqual(lists.withdrawn.map((o) => o.ine), ["I5"]);
  assert.equal(studentState([f], null), "aucun");
});

/* ─── Clôture : échéance et déclenchement ─── */

test("échéance de clôture : date limite, prolongation, plafond J-14", () => {
  assert.equal(closureDueDate({ deadline: "2026-12-01", departureIso: "2027-02-01" }), "2026-12-01");
  assert.equal(closureDueDate({ deadline: "2026-12-01", departureIso: "2027-02-01", extendedDeadlines: ["2026-12-10", undefined] }), "2026-12-10");
  // plafond : départ le 2027-01-10 → J-14 = 2026-12-27
  assert.equal(closureDueDate({ deadline: "2026-12-20", departureIso: "2027-01-10", extendedDeadlines: ["2027-01-05"] }), "2026-12-27");
  assert.equal(closureDueDate({ deadline: null, departureIso: "2027-01-10" }), null);
});

function checkWith(over: Parameters<typeof evaluateClosure>[0]) {
  return evaluateClosure(over);
}

test("clôture : automatique après la date limite, proposée si tout le monde a répondu, bloquée par un suspens", () => {
  const a = inv({ id: "AAAAAAAA", ine: "I1", signature: sigOnline("AAAAAAAA") });
  const b = inv({ id: "BBBBBBBB", ine: "I2", signature: sigNo });
  const c = inv({ id: "CCCCCCCC", ine: "I3" });
  const base = { deadline: "2026-12-01", departureIso: "2027-02-01", cases: NO_CASES, preListeInes: ["I1", "I2", "I3"] };
  // le jour de la date limite reste ouvert
  const same = checkWith({ ...base, todayIso: "2026-12-01", invs: [a, b, c] });
  assert.equal(same.dueNow, false);
  assert.equal(decideClosureAction(same), "wait");
  const after = checkWith({ ...base, todayIso: "2026-12-02", invs: [a, b, c] });
  assert.equal(after.dueNow, true);
  assert.equal(decideClosureAction(after), "close_auto");
  // tous ont répondu (oui ou non) avant la date → proposition à l'organisateur
  const all = checkWith({ ...base, todayIso: "2026-11-20", invs: [a, b, { ...c, signature: sigNo }] });
  assert.equal(all.allAnswered, true);
  assert.equal(decideClosureAction(all), "propose_early");
  assert.deepEqual(canCloseManually(all), { ok: true });
  const notYet = checkWith({ ...base, todayIso: "2026-11-20", invs: [a, b, c] });
  assert.equal(canCloseManually(notYet).ok, false);
  // un désaccord non tranché bloque (alerte), même après la date
  const cObj = inv({ id: "CCCCCCC2", ine: "I1", objection: { at: "t", signerName: "P2" } });
  const blocked = checkWith({ ...base, todayIso: "2026-12-05", invs: [a, b, c, cObj] });
  assert.deepEqual(blocked.suspendedInes, ["I1"]);
  assert.equal(decideClosureAction(blocked), "blocked_alert");
  assert.equal(canCloseManually(blocked).ok, false);
});

test("la prolongation accordée à une famille repousse la clôture automatique", () => {
  const a = inv({ id: "AAAAAAAA", ine: "I1", signature: sigOnline("AAAAAAAA") });
  const c = inv({ id: "CCCCCCCC", ine: "I3", extendedDeadline: "2026-12-10" });
  const r = checkWith({
    deadline: "2026-12-01",
    departureIso: "2027-02-01",
    todayIso: "2026-12-05",
    invs: [a, c],
    cases: NO_CASES,
    preListeInes: ["I1", "I3"],
  });
  assert.equal(r.dueDate, "2026-12-10");
  assert.equal(decideClosureAction(r), "wait");
});

/* ─── Bon cuisine, jour par jour ─── */

test("bon cuisine : total, classique, sans porc, végétarien par jour ; les « non » ne comptent pas", () => {
  const days = ["2027-03-01", "2027-03-02"];
  const mk = (id: string, ine: string, meals: FamilyInvitation["answers"]["meals"]) =>
    inv({ id, ine, signature: sigOnline(id), answers: { participates: "oui", meals } });
  const official = [
    mk("AAAAAAAA", "I1", { "2027-03-01": { wants: "oui", type: "classique" }, "2027-03-02": { wants: "non" } }),
    mk("BBBBBBBB", "I2", { "2027-03-01": { wants: "oui", type: "sans_porc" }, "2027-03-02": { wants: "oui", type: "vegetarien" } }),
    mk("CCCCCCCC", "I3", { "2027-03-01": { wants: "oui", type: "vegetarien" } }),
    mk("DDDDDDDD", "I4", undefined),
  ].map((i) => ({ inv: i }));
  const r = computeMealsPerDay(official, days);
  assert.deepEqual(r["2027-03-01"], { total: 3, classique: 1, sansPorc: 1, vegetarien: 1 });
  assert.deepEqual(r["2027-03-02"], { total: 1, classique: 0, sansPorc: 0, vegetarien: 1 });
  // uniquement les jours choisis par l'organisateur : un jour ajouté par la famille est ignoré
  assert.deepEqual(Object.keys(r), days);
});

test("bon cuisine : les repas adultes s'ajoutent au total, structure v1 (orders) remplie sur les jours choisis", () => {
  const details = {
    active: true,
    deliveryPlace: "Self",
    daysSelection: { "2027-03-01": true, "2027-03-02": true },
    orders: { "2027-03-01": { picnicTotal: "99", coffee: "2" } },
    datesMode: true,
  };
  const perDay = { "2027-03-01": { total: 3, classique: 1, sansPorc: 1, vegetarien: 1 }, "2027-03-02": { total: 1, classique: 0, sansPorc: 0, vegetarien: 1 } };
  const out = applyMealsToCuisineDetails(details, perDay, { "2027-03-01": 2, "2027-03-02": 0 }) as {
    orders: Record<string, Record<string, string>>;
  };
  assert.equal(out.orders["2027-03-01"].picnicTotal, "5");
  assert.equal(out.orders["2027-03-01"].picnicNoPork, "1");
  assert.equal(out.orders["2027-03-01"].picnicVeg, "1");
  assert.equal(out.orders["2027-03-01"].coffee, "2"); // le reste de la ligne n'est pas écrasé
  assert.equal(out.orders["2027-03-02"].picnicTotal, "1");
  // miroir lundi–vendredi pour la commande cuisine v1 (2027-03-01 = lundi, 2027-03-02 = mardi)
  assert.equal(out.orders.lundi.picnicTotal, "5");
  assert.equal(out.orders.mardi.picnicTotal, "1");
});

test("repas adultes : valeurs nettoyées", () => {
  assert.deepEqual(sanitizeAdultMeals({ "2027-03-01": "3", "2027-03-02": -4, x: 9 }, ["2027-03-01", "2027-03-02"]), {
    "2027-03-01": 3,
    "2027-03-02": 0,
  });
  assert.deepEqual(sanitizeAdultMeals(null, ["a"]), { a: 0 });
});

/* ─── Devis rectifié ─── */

test("devis rectifié : écart > 10 % ET au moins 14 jours avant le départ", () => {
  const base = { needsBus: true, departureIso: "2027-03-30", todayIso: "2027-03-01" }; // 29 jours
  assert.equal(evaluateRectifiedQuote({ ...base, quoteNbEleves: 50, officialCount: 45 }).proposed, false); // exactement 10 %
  assert.equal(evaluateRectifiedQuote({ ...base, quoteNbEleves: 50, officialCount: 44 }).proposed, true); // 12 %
  assert.equal(evaluateRectifiedQuote({ ...base, quoteNbEleves: 50, officialCount: 56 }).proposed, true); // plus nombreux aussi
  assert.equal(evaluateRectifiedQuote({ ...base, quoteNbEleves: 50, officialCount: 50 }).proposed, false);
  const v = evaluateRectifiedQuote({ ...base, quoteNbEleves: 50, officialCount: 40 });
  assert.equal(v.deviationPct, 20);
  // J-14 : 14 jours restants → oui ; 13 → non (on garde le devis)
  const dep = "2027-03-15";
  assert.equal(evaluateRectifiedQuote({ needsBus: true, quoteNbEleves: 50, officialCount: 40, departureIso: dep, todayIso: "2027-03-01" }).proposed, true);
  const late = evaluateRectifiedQuote({ needsBus: true, quoteNbEleves: 50, officialCount: 40, departureIso: dep, todayIso: "2027-03-02" });
  assert.equal(late.proposed, false);
  assert.match(late.reason, /garde le devis/);
  // pas de bus / pas de devis connu → rien à proposer
  assert.equal(evaluateRectifiedQuote({ ...base, needsBus: false, quoteNbEleves: 50, officialCount: 10 }).proposed, false);
  assert.equal(evaluateRectifiedQuote({ ...base, quoteNbEleves: null, officialCount: 10 }).proposed, false);
});

/* ─── Comptabilité ─── */

test("récapitulatif compta : tous les engagements « oui », signé / papier, montants, mois, statut", () => {
  const a = inv({ id: "AAAAAAAA", ine: "I1", studentName: "Anna A", classe: "3A", signature: sigOnline("AAAAAAAA"), engagement: engagement("AAAAAAAA", "I1", "Anna A", 120) });
  const b = inv({
    id: "BBBBBBBB",
    ine: "I2",
    studentName: "Bob B",
    classe: "3B",
    signature: {
      decision: "oui",
      signedAt: "2026-11-03T10:00:00Z",
      signerName: "Parent B",
      pdfKey: "scan-b.jpg",
      paper: { recordedBy: "Prof", recordedAt: "2026-11-03T10:00:00Z", scanFileName: "coupon.jpg", scanKey: "scan-b.jpg" },
    },
    engagement: { ...engagement("BBBBBBBB", "I2", "Bob B", 0), pdfKey: "scan-b.jpg" },
  });
  const d = inv({ id: "DDDDDDDD", ine: "I4", studentName: "Dan D", signature: sigOnline("DDDDDDDD"), engagement: engagement("DDDDDDDD", "I4", "Dan D", 90), withdrawal: { at: "t", by: "Prof" } });
  const f = inv({ id: "FFFFFFFF", ine: "I6", studentName: "Fay F", signature: sigNo });
  const rows = buildComptaRows([a, b, d, f], NO_CASES);
  assert.deepEqual(rows.map((r) => r.studentName), ["Anna A", "Bob B", "Dan D"]); // pas de « non »
  assert.equal(rows[0].mode, "signé en ligne");
  assert.equal(rows[0].amount, 120);
  assert.equal(rows[0].firstMonth, "2027-01");
  assert.equal(rows[0].lastMonth, "2027-06");
  assert.equal(rows[1].mode, "coupon papier");
  assert.equal(rows[1].zipPath.endsWith(".jpg"), true);
  assert.equal(rows[2].status, "desiste");
  const csv = comptaCsv(rows);
  assert.match(csv, /Élève";"Classe";"INE";"Parent signataire";"Mode";"Signé le";"Montant/);
  assert.match(csv, /Désisté après la clôture/);
  assert.match(csv, /coupon papier/);
  assert.match(csv, /"Parent A"/);
});

/* ─── Purge ─── */

test("purge : tout est supprimé sauf les engagements (PDF signés, scans « oui »)", () => {
  const yesOnline = inv({
    id: "AAAAAAAA",
    ine: "I1",
    signature: sigOnline("AAAAAAAA"),
    engagement: engagement("AAAAAAAA", "I1", "Anna A"),
    documents: { d1: { fileName: "a.pdf", key: "docs/A/d1.pdf", size: 1, contentType: "application/pdf", uploadedAt: "t" } },
    answers: { participates: "oui", phone: "0600000000", meals: { "2027-03-01": { wants: "oui", type: "classique" } } },
    code: { hash: "x", expiresAt: "t", attempts: 0, sentAt: "t" },
    codeSends: ["t"],
    objection: { at: "t", signerName: "P2", reason: "secret" },
    reminders: { lastDay: "2027-01-01" },
  });
  const yesPaper = inv({
    id: "BBBBBBBB",
    ine: "I2",
    signature: {
      decision: "oui",
      signedAt: "t",
      signerName: "Parent B",
      pdfKey: "papier-b.jpg",
      paper: { recordedBy: "Prof", recordedAt: "t", scanFileName: "c.jpg", scanKey: "papier-b.jpg" },
    },
    engagement: { ...engagement("BBBBBBBB", "I2", "Bob B"), pdfKey: "papier-b.jpg" },
  });
  const paperNo = inv({
    id: "CCCCCCCC",
    ine: "I3",
    signature: { decision: "non", signedAt: "t", signerName: "P", paper: { recordedBy: "Prof", recordedAt: "t", scanFileName: "n.jpg", scanKey: "papier-c.jpg" } },
    documents: { d1: { fileName: "c.pdf", key: "docs/C/d1.pdf", size: 1, contentType: "application/pdf", uploadedAt: "t" } },
  });
  const onlineNo = inv({ id: "DDDDDDDD", ine: "I4", signature: sigNo });
  const never = inv({
    id: "EEEEEEEE",
    ine: "I5",
    documents: { d1: { fileName: "e.pdf", key: "docs/E/d1.pdf", size: 1, contentType: "application/pdf", uploadedAt: "t" } },
  });
  const plan = planPurge([yesOnline, yesPaper, paperNo, onlineNo, never], NO_CASES, "2027-04-01T00:00:00Z");

  // supprimés : pièces déposées, tracé de signature, scan papier « non »
  assert.deepEqual(
    [...plan.deleteKeys].sort(),
    ["docs/A/d1.pdf", "docs/C/d1.pdf", "docs/E/d1.pdf", "papier-c.jpg", "sig-AAAAAAAA.png"].sort(),
  );
  // conservés : engagement signé en ligne et scan papier « oui »
  assert.deepEqual([...plan.keptKeys].sort(), ["eng-AAAAAAAA.pdf", "papier-b.jpg"].sort());
  for (const k of plan.keptKeys) assert.equal(plan.deleteKeys.includes(k), false);
  // invitations : celles sans « oui » disparaissent ; les engagements restent sous forme minimale
  assert.deepEqual([...plan.deleteInvitationIds].sort(), ["CCCCCCCC", "DDDDDDDD", "EEEEEEEE"]);
  assert.deepEqual(plan.archive.map((a) => a.id).sort(), ["AAAAAAAA", "BBBBBBBB"]);
  const a = plan.archive.find((x) => x.id === "AAAAAAAA")!;
  assert.equal(a.email, "");
  assert.deepEqual(a.tokenHashes, []);
  assert.deepEqual(a.answers, {});
  assert.deepEqual(a.documents, {});
  assert.equal(a.code, undefined);
  assert.equal(a.objection, undefined);
  assert.equal(a.reminders, undefined);
  assert.equal(a.signature?.signatureKey, undefined);
  assert.equal(a.signature?.ipHash, undefined);
  assert.equal(a.signature?.pdfKey, "eng-AAAAAAAA.pdf");
  assert.equal(a.engagement?.amount, 90);
  assert.equal(a.engagement?.pdfKey, "eng-AAAAAAAA.pdf");
  assert.equal(a.purgedAt, "2027-04-01T00:00:00Z");
  const b = plan.archive.find((x) => x.id === "BBBBBBBB")!;
  assert.equal(b.signature?.paper?.scanKey, "papier-b.jpg");
});

test("purge : aucun engagement n'est supprimé, même si une pièce déposée porte la clé d'un engagement", () => {
  const evil = inv({
    id: "AAAAAAAA",
    ine: "I1",
    signature: sigOnline("AAAAAAAA"),
    engagement: engagement("AAAAAAAA", "I1", "Anna A"),
    documents: { d1: { fileName: "x.pdf", key: "eng-AAAAAAAA.pdf", size: 1, contentType: "application/pdf", uploadedAt: "t" } },
  });
  const plan = planPurge([evil], NO_CASES);
  assert.equal(plan.deleteKeys.includes("eng-AAAAAAAA.pdf"), false);
  assert.equal(engagementKeys(evil).includes("eng-AAAAAAAA.pdf"), true);
  // le désisté et le retiré gardent aussi leur engagement
  const withdrawn = { ...evil, documents: {}, withdrawal: { at: "t", by: "P" } };
  const p2 = planPurge([withdrawn], NO_CASES);
  assert.equal(p2.archive.length, 1);
  assert.equal(p2.archive[0].withdrawal?.by, "P");
  const retired: StudentCase = { id: "I1", tripId: "trip12345", ine: "I1", history: [], decision: { kind: "retirer", at: "2027-01-01", by: "Dir", note: "privé" } };
  const p3 = planPurge([{ ...evil, documents: {} }], [retired]);
  assert.equal(p3.archive.length, 1);
  assert.equal(p3.archive[0].withdrawal?.kind, "retire_direction");
  assert.equal(JSON.stringify(p3.archive[0]).includes("privé"), false);
});

test("purge idempotente : rejouée sur des données déjà purgées, rien de plus à supprimer", () => {
  const first = planPurge([
    inv({
      id: "AAAAAAAA",
      ine: "I1",
      signature: sigOnline("AAAAAAAA"),
      engagement: engagement("AAAAAAAA", "I1", "Anna A"),
      documents: { d1: { fileName: "a.pdf", key: "docs/A/d1.pdf", size: 1, contentType: "application/pdf", uploadedAt: "t" } },
    }),
  ], NO_CASES, "2027-04-01T00:00:00Z");
  const second = planPurge(first.archive, NO_CASES, "2027-04-02T00:00:00Z");
  assert.deepEqual(second.deleteKeys, []);
  assert.deepEqual(second.deleteInvitationIds, []);
  assert.deepEqual(second.keptKeys, first.keptKeys);
  assert.deepEqual(second.archive, first.archive); // purgedAt d'origine conservé
  // l'archive d'une archive est identique
  assert.deepEqual(archiveInvitation(first.archive[0]), first.archive[0]);
});

test("purge : déclenchée à partir du lendemain du retour", () => {
  assert.equal(isPurgeDue("2027-03-10", "2027-03-10"), false);
  assert.equal(isPurgeDue("2027-03-10", "2027-03-11"), true);
  assert.equal(isPurgeDue("2027-03-10", "2027-04-30"), true);
  assert.equal(isPurgeDue(null, "2027-03-11"), false);
});

/* ─── Somme en liquide facultative ─── */

test("liquide facultatif : décochée = retirée ; cochée = montant et motif obligatoires", () => {
  assert.equal(resolveCashDraft({ enabled: false, amountText: "10", reason: "sandwich" }), undefined);
  const unchecked = { documents: [], cash: resolveCashDraft({ enabled: false, amountText: "10", reason: "x" }) };
  assert.equal("cash" in JSON.parse(JSON.stringify(unchecked)), false);
  assert.equal(validateFamilyStructured(unchecked).ok, true);
  const empty = resolveCashDraft({ enabled: true, amountText: "", reason: "" });
  assert.equal(validateFamilyStructured({ documents: [], cash: empty }).ok, false);
  const noReason = resolveCashDraft({ enabled: true, amountText: "5", reason: "  " });
  assert.equal(validateFamilyStructured({ documents: [], cash: noReason }).ok, false);
  const noAmount = resolveCashDraft({ enabled: true, amountText: "0", reason: "sandwich" });
  assert.equal(validateFamilyStructured({ documents: [], cash: noAmount }).ok, false);
  const ok = resolveCashDraft({ enabled: true, amountText: "5,50", reason: " sandwich " });
  assert.deepEqual(ok, { amount: 5.5, reason: "sandwich" });
  assert.equal(validateFamilyStructured({ documents: [], cash: ok }).ok, true);
});
