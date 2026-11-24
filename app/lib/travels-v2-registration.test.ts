import { test } from "node:test";
import assert from "node:assert/strict";
import { deflateSync } from "node:zlib";
import {
  addTokenHash,
  buildEngagement,
  buildFamilyIcs,
  buildInviteToken,
  buildPublicFamilyView,
  checkSigningCode,
  generateInviteSecret,
  generateSigningCode,
  hashInviteSecret,
  issueSigningCode,
  parseInviteToken,
  siblingDecision,
  verifyInviteSecret,
} from "@/app/lib/travels-v2-registration";
import {
  CODE_MAX_ATTEMPTS,
  CODE_MAX_SENDS_PER_HOUR,
  CODE_MIN_INTERVAL_MS,
  CODE_TTL_MS,
  MAX_TOKEN_HASHES,
  canSendCode,
  computeInvitationStatus,
  decodeSignaturePng,
  effectiveDeadline,
  isLocked,
  mergeAnswers,
  missingBlockingDocs,
  registrationWindow,
  signingProblems,
  sniffAllowedDocType,
  type FamilyInvitation,
} from "@/app/lib/travels-v2-registration-types";
import { PROXY_PUBLIC_ROUTE_MATCHERS } from "@/app/lib/public-routes";
import { isWorkflowV2 } from "@/app/lib/travels-v2";
import type { TravelsFamilyDocRequest } from "@/app/lib/travels-types";

const TRIP = "11111111-2222-3333-4444-555555555555";
const INV = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

function makeInv(over: Partial<FamilyInvitation> = {}): FamilyInvitation {
  return {
    id: INV,
    tripId: TRIP,
    ine: "123456789AB",
    studentName: "Camille DURAND",
    email: "parent@example.org",
    createdAt: "2026-11-25T08:00:00.000Z",
    tokenHashes: [],
    sendCount: 0,
    answers: {},
    documents: {},
    exemptions: {},
    codeSends: [],
    ...over,
  };
}

const docs: TravelsFamilyDocRequest[] = [
  { id: "d1", label: "Attestation d'aisance aquatique", kind: "avant_signature" },
  { id: "d2", label: "Autorisation photo", kind: "avant_depart", dueDate: "2027-02-01" },
  { id: "d3", label: "Modèle facultatif", kind: "facultative" },
];

/* ─── Jetons ─── */

test("jeton : secret de 32 octets (43 caractères base64url), aléatoire", () => {
  const a = generateInviteSecret();
  const b = generateInviteSecret();
  assert.match(a, /^[A-Za-z0-9_-]{43}$/);
  assert.notEqual(a, b);
});

test("jeton : l'empreinte stockée ne contient pas le secret et vérifie le bon secret seulement", () => {
  const secret = generateInviteSecret();
  const h = hashInviteSecret(secret);
  assert.match(h, /^[0-9a-f]{64}$/);
  assert.ok(!h.includes(secret));
  assert.ok(verifyInviteSecret([h], secret));
  assert.ok(!verifyInviteSecret([h], generateInviteSecret()));
  assert.ok(!verifyInviteSecret([], secret));
  assert.ok(!verifyInviteSecret([h], "trop-court"));
  // L'empreinte brute (ce qui est stocké) ne permet pas d'ouvrir la page.
  assert.ok(!verifyInviteSecret([h], h.slice(0, 43)));
});

test("jeton : construction / lecture, et rejet des formats invalides", () => {
  const secret = generateInviteSecret();
  const tok = buildInviteToken({ invitationId: INV, tripId: TRIP, secret });
  assert.deepEqual(parseInviteToken(tok), { invitationId: INV, tripId: TRIP, secret });
  for (const bad of ["", "abc", `${INV}.${TRIP}`, `${INV}.${TRIP}.court`, `../x.${TRIP}.${secret}`, `${INV}.${TRIP}.${secret}.extra`, null, undefined]) {
    assert.equal(parseInviteToken(bad as string), null);
  }
});

test("jeton : les anciens liens restent valides au renvoi, dans la limite de 5", () => {
  let hashes: string[] = [];
  const secrets: string[] = [];
  for (let i = 0; i < MAX_TOKEN_HASHES + 2; i++) {
    const s = generateInviteSecret();
    secrets.push(s);
    hashes = addTokenHash(hashes, hashInviteSecret(s));
  }
  assert.equal(hashes.length, MAX_TOKEN_HASHES);
  assert.ok(verifyInviteSecret(hashes, secrets[secrets.length - 1]));
  assert.ok(verifyInviteSecret(hashes, secrets[2]));
  assert.ok(!verifyInviteSecret(hashes, secrets[0]));
});

/* ─── Code de signature ─── */

test("code : 6 chiffres, haché, expire en 10 minutes", () => {
  assert.match(generateSigningCode(), /^\d{6}$/);
  const now = new Date("2026-11-25T10:00:00Z");
  const { code, state } = issueSigningCode(INV, now);
  assert.ok(!JSON.stringify(state).includes(code) || state.hash.includes(code) === false);
  assert.equal(new Date(state.expiresAt).getTime() - now.getTime(), CODE_TTL_MS);
  assert.equal(CODE_TTL_MS, 10 * 60 * 1000);
  assert.equal(state.attempts, 0);
});

test("code : bon code accepté une seule fois (état supprimé)", () => {
  const now = new Date("2026-11-25T10:00:00Z");
  const { code, state } = issueSigningCode(INV, now);
  const r = checkSigningCode(INV, state, code, new Date(now.getTime() + 60_000));
  assert.equal(r.result, "ok");
  assert.equal(r.next, undefined);
  assert.equal(checkSigningCode(INV, undefined, code, now).result, "none");
});

test("code : un code d'une autre invitation est refusé", () => {
  const now = new Date();
  const { code, state } = issueSigningCode(INV, now);
  assert.equal(checkSigningCode("autre-invitation-id", state, code, now).result, "wrong");
});

test("code : expiré après 10 minutes", () => {
  const now = new Date("2026-11-25T10:00:00Z");
  const { code, state } = issueSigningCode(INV, now);
  const r = checkSigningCode(INV, state, code, new Date(now.getTime() + CODE_TTL_MS + 1000));
  assert.equal(r.result, "expired");
});

test("code : 5 essais maximum, puis le code est annulé même si le bon est ensuite saisi", () => {
  const now = new Date("2026-11-25T10:00:00Z");
  const { code, state } = issueSigningCode(INV, now);
  const wrong = code === "000000" ? "111111" : "000000";
  let cur: typeof state | undefined = state;
  for (let i = 1; i < CODE_MAX_ATTEMPTS; i++) {
    const r = checkSigningCode(INV, cur, wrong, now);
    assert.equal(r.result, "wrong");
    if (r.result === "wrong") {
      assert.equal(r.attemptsLeft, CODE_MAX_ATTEMPTS - i);
      cur = r.next;
    }
  }
  const last = checkSigningCode(INV, cur, wrong, now);
  assert.equal(last.result, "locked");
  assert.equal(last.next, undefined);
  assert.equal(checkSigningCode(INV, last.next, code, now).result, "none");
});

test("code : limite d'envoi (30 s entre deux envois, 5 par heure)", () => {
  const now = new Date("2026-11-25T10:00:00Z");
  assert.ok(canSendCode({ codeSends: [], code: undefined }, now).ok);
  const recent = [new Date(now.getTime() - 10_000).toISOString()];
  const r1 = canSendCode({ codeSends: recent, code: undefined }, now);
  assert.equal(r1.ok, false);
  const spaced = Array.from({ length: CODE_MAX_SENDS_PER_HOUR }, (_, i) =>
    new Date(now.getTime() - (CODE_MIN_INTERVAL_MS + 1000) * (i + 1)).toISOString(),
  );
  assert.equal(canSendCode({ codeSends: spaced, code: undefined }, now).ok, false);
  const old = spaced.map((s) => new Date(new Date(s).getTime() - 3600_000).toISOString());
  assert.ok(canSendCode({ codeSends: old, code: undefined }, now).ok);
});

/* ─── État d'une invitation ─── */

const picnic = ["2027-03-10", "2027-03-11"];

test("état : pas ouvert → ouvert → en cours", () => {
  const inv = makeInv();
  assert.equal(computeInvitationStatus({ inv, docs, picnicDayKeys: picnic }), "pas_ouvert");
  inv.firstOpenedAt = "2026-11-25T09:00:00Z";
  assert.equal(computeInvitationStatus({ inv, docs, picnicDayKeys: picnic }), "ouvert");
  inv.answers = { participates: "oui" };
  assert.equal(computeInvitationStatus({ inv, docs, picnicDayKeys: picnic }), "bloque_piece");
});

test("état : pièce « avant signature » manquante bloque ; dépôt ou dispense débloque ; « avant départ » ne bloque pas", () => {
  const inv = makeInv({
    firstOpenedAt: "x",
    answers: { participates: "oui", phone: "06 12 34 56 78", meals: { "2027-03-10": { wants: "non" }, "2027-03-11": { wants: "oui", type: "vegetarien" } } },
  });
  assert.deepEqual(missingBlockingDocs(docs, inv), ["d1"]);
  assert.equal(computeInvitationStatus({ inv, docs, picnicDayKeys: picnic }), "bloque_piece");
  inv.documents = { d1: { fileName: "a.pdf", key: "k", size: 10, contentType: "application/pdf", uploadedAt: "x" } };
  assert.equal(computeInvitationStatus({ inv, docs, picnicDayKeys: picnic }), "pret");
  const exempt = makeInv({ ...inv, documents: {}, exemptions: { d1: { at: "x", by: "Organisateur" } } });
  assert.equal(computeInvitationStatus({ inv: exempt, docs, picnicDayKeys: picnic }), "pret");
});

test("état : prêt seulement si téléphone et tous les jours de panier renseignés", () => {
  const inv = makeInv({
    firstOpenedAt: "x",
    documents: { d1: { fileName: "a.pdf", key: "k", size: 10, contentType: "application/pdf", uploadedAt: "x" } },
    answers: { participates: "oui" },
  });
  assert.equal(computeInvitationStatus({ inv, docs, picnicDayKeys: picnic }), "en_cours");
  assert.equal(signingProblems({ answers: inv.answers, picnicDayKeys: picnic, docs, inv }).length, 2);
});

test("état : « non » se signe sans pièce, sans téléphone ni panier", () => {
  const inv = makeInv({ firstOpenedAt: "x", answers: { participates: "non" } });
  assert.deepEqual(signingProblems({ answers: inv.answers, picnicDayKeys: picnic, docs, inv }), []);
  assert.equal(computeInvitationStatus({ inv, docs, picnicDayKeys: picnic }), "pret");
});

test("état : signé oui / refus, et réponse verrouillée après signature", () => {
  const oui = makeInv({ signature: { decision: "oui", signedAt: "x", signerName: "A B" } });
  const non = makeInv({ signature: { decision: "non", signedAt: "x", signerName: "A B" } });
  assert.equal(computeInvitationStatus({ inv: oui, docs, picnicDayKeys: [] }), "signe_oui");
  assert.equal(computeInvitationStatus({ inv: non, docs, picnicDayKeys: [] }), "refus");
  assert.ok(isLocked(oui));
  assert.ok(!isLocked(makeInv()));
  // Fenêtre : une réponse signée n'est plus modifiable, même avant la date limite.
  const w = registrationWindow({ tripStatus: "INSCRIPTIONS_OUVERTES", openedAt: "x", deadline: "2027-01-31", signed: true, todayIso: "2026-12-01" });
  assert.deepEqual(w, { open: false, reason: "locked", deadline: "2027-01-31" });
});

test("second parent : un « oui » déjà signé est détecté", () => {
  const a = makeInv({ id: "a1111111", signature: { decision: "oui", signedAt: "x", signerName: "P" } });
  const b = makeInv({ id: "b2222222" });
  const c = makeInv({ id: "c3333333", ine: "AUTRE" });
  assert.equal(siblingDecision([a, b, c], b), "oui");
  assert.equal(siblingDecision([a, b, c], c), null);
  assert.equal(siblingDecision([a, b], a), null);
});

/* ─── Fermeture par date limite ─── */

test("fenêtre : ouverte jusqu'au jour de la date limite inclus, fermée le lendemain", () => {
  const base = { tripStatus: "INSCRIPTIONS_OUVERTES", openedAt: "x", deadline: "2027-01-31", signed: false };
  assert.equal(registrationWindow({ ...base, todayIso: "2027-01-31" }).open, true);
  const closed = registrationWindow({ ...base, todayIso: "2027-02-01" });
  assert.deepEqual(closed, { open: false, reason: "deadline", deadline: "2027-01-31" });
});

test("fenêtre : prolongation par famille, jamais plus courte que la date limite générale", () => {
  assert.equal(effectiveDeadline("2027-01-31", "2027-02-05"), "2027-02-05");
  assert.equal(effectiveDeadline("2027-01-31", "2027-01-10"), "2027-01-31");
  assert.equal(effectiveDeadline(undefined, undefined), null);
  const w = registrationWindow({ tripStatus: "INSCRIPTIONS_OUVERTES", openedAt: "x", deadline: "2027-01-31", extendedDeadline: "2027-02-05", signed: false, todayIso: "2027-02-03" });
  assert.equal(w.open, true);
});

test("fenêtre : fermée si le voyage n'est plus « inscriptions ouvertes » ou jamais ouvert, ou sans date limite", () => {
  const base = { openedAt: "x", deadline: "2027-01-31", signed: false, todayIso: "2027-01-10" };
  assert.deepEqual(registrationWindow({ ...base, tripStatus: "VALIDE" }), { open: false, reason: "status", deadline: "2027-01-31" });
  assert.deepEqual(registrationWindow({ ...base, tripStatus: "EN_ATTENTE_DIR_FINAL", openedAt: undefined }), { open: false, reason: "not_opened", deadline: "2027-01-31" });
  assert.equal(registrationWindow({ ...base, tripStatus: "INSCRIPTIONS_OUVERTES", deadline: undefined }).open, false);
});

/* ─── Réponses ─── */

test("réponses : la famille ne peut pas ajouter un jour de panier", () => {
  const r = mergeAnswers({ current: {}, input: { meals: { "2027-03-15": { wants: "oui" } } }, picnicDayKeys: picnic, nowIso: "x" });
  assert.equal(r.ok, false);
});

test("réponses : « oui » prend le type classique par défaut, « non » n'a pas de type", () => {
  const r = mergeAnswers({
    current: {},
    input: { participates: "oui", meals: { "2027-03-10": { wants: "oui" }, "2027-03-11": { wants: "non", type: "vegetarien" } }, phone: "06 12 34 56 78" },
    picnicDayKeys: picnic,
    nowIso: "2026-11-25T00:00:00Z",
  });
  assert.ok(r.ok);
  if (r.ok) {
    assert.deepEqual(r.answers.meals, { "2027-03-10": { wants: "oui", type: "classique" }, "2027-03-11": { wants: "non" } });
    assert.equal(r.answers.participates, "oui");
  }
});

test("réponses : champs inconnus, types invalides et téléphone invalide refusés", () => {
  const bad = (input: unknown) => mergeAnswers({ current: {}, input, picnicDayKeys: picnic, nowIso: "x" }).ok;
  assert.equal(bad({ participates: "peut-etre" }), false);
  assert.equal(bad({ isAdmin: true }), false);
  assert.equal(bad({ meals: { "2027-03-10": { wants: "oui", type: "halal" } } }), false);
  assert.equal(bad({ phone: "<script>alert(1)</script>" }), false);
  assert.equal(bad(null), false);
  assert.equal(bad({ phone: "06 12 34 56 78" }), true);
});

/* ─── Fichiers ─── */

function pngBytes(w = 300, h = 100): Buffer {
  const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  const chunk = (type: string, data: Buffer) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    return Buffer.concat([len, Buffer.from(type), data, Buffer.alloc(4)]);
  };
  return Buffer.concat([sig, chunk("IHDR", ihdr), chunk("IDAT", deflateSync(Buffer.from(Array.from({ length: 600 }, (_, i) => (i * 37) % 251)))), chunk("IEND", Buffer.alloc(0))]);
}

test("signature : PNG valide accepté, autres formats et tailles aberrantes refusés", () => {
  const ok = decodeSignaturePng(`data:image/png;base64,${pngBytes().toString("base64")}`);
  assert.ok(ok.ok);
  assert.equal(decodeSignaturePng(undefined).ok, false);
  assert.equal(decodeSignaturePng("data:image/jpeg;base64,AAAA").ok, false);
  assert.equal(decodeSignaturePng(`data:image/png;base64,${Buffer.alloc(500, 1).toString("base64")}`).ok, false);
  assert.equal(decodeSignaturePng(`data:image/png;base64,${pngBytes(5000, 100).toString("base64")}`).ok, false);
  const huge = Buffer.concat([pngBytes(), Buffer.alloc(250 * 1024)]);
  assert.equal(decodeSignaturePng(`data:image/png;base64,${huge.toString("base64")}`).ok, false);
});

test("pièces : PDF et images reconnus par leurs octets, le reste refusé", () => {
  const pad = (b: number[]) => Uint8Array.from([...b, ...new Array(20).fill(0)]);
  assert.equal(sniffAllowedDocType(pad([0x25, 0x50, 0x44, 0x46, 0x2d]))?.ext, "pdf");
  assert.equal(sniffAllowedDocType(pad([0xff, 0xd8, 0xff, 0xe0]))?.ext, "jpg");
  assert.equal(sniffAllowedDocType(pngBytes())?.ext, "png");
  assert.equal(sniffAllowedDocType(Buffer.from("<html><script>alert(1)</script></html>")), null);
  assert.equal(sniffAllowedDocType(Buffer.from("<svg xmlns='http://www.w3.org/2000/svg'></svg>")), null);
  assert.equal(sniffAllowedDocType(Buffer.from("MZ\x90\x00\x03\x00\x00\x00\x04\x00\x00\x00\xff\xff")), null);
});

/* ─── ICS ─── */

test("ICS : séjour + rendez-vous de départ et de retour avec heure et lieu", () => {
  const ics = buildFamilyIcs({
    tripId: TRIP,
    invitationId: INV,
    title: "Voyage à Biarritz",
    destination: "Biarritz",
    startDate: "2027-03-08",
    endDate: "2027-03-12",
    studentName: "Camille DURAND",
    parentCalendar: {
      points: [
        { id: "p1", kind: "depot", date: "2027-03-08", time: "07:30", place: "Parking du lycée", durationMinutes: 45 },
        { id: "p2", kind: "recuperation", date: "2027-03-12", time: "19:15", place: "Cour d'honneur" },
      ],
    },
  });
  assert.equal((ics.match(/BEGIN:VEVENT/g) || []).length, 3);
  assert.match(ics, /DTSTART;TZID=Europe\/Paris:20270308T073000/);
  assert.match(ics, /DTSTART;TZID=Europe\/Paris:20270312T191500/);
  assert.match(ics, /LOCATION:Parking du lycée/);
  assert.match(ics, /LOCATION:Cour d'honneur/);
  assert.match(ics, /SUMMARY:Rendez-vous de départ/);
  assert.match(ics, /SUMMARY:Rendez-vous de retour/);
  assert.match(ics, /UID:voyage-[^@]+-sejour@scola/);
  assert.match(ics, /^BEGIN:VCALENDAR/);
  assert.match(ics, /END:VCALENDAR$/);
});

test("ICS : sans rendez-vous, seulement l'événement séjour ; sans dates, aucun événement", () => {
  const one = buildFamilyIcs({ tripId: TRIP, invitationId: INV, title: "Sortie", startDate: "2027-03-08", endDate: "2027-03-08", parentCalendar: { points: [] } });
  assert.equal((one.match(/BEGIN:VEVENT/g) || []).length, 1);
  const none = buildFamilyIcs({ tripId: TRIP, invitationId: INV, title: "Sortie", startDate: null, endDate: null });
  assert.equal((none.match(/BEGIN:VEVENT/g) || []).length, 0);
});

/* ─── Engagement ─── */

test("engagement : montant, mois de facturation, date et référence du PDF", () => {
  const e = buildEngagement({
    inv: { id: INV, ine: "123456789AB", studentName: "Camille DURAND" },
    priceBlock: { pricePerStudent: 100, firstMonth: "2027-01", lastMonth: "2027-03" },
    signerName: "Marie Durand",
    signedAt: "2027-01-05T10:00:00Z",
    pdfKey: "travels-v2/x/engagements/y.pdf",
  });
  assert.equal(e.amount, 100);
  assert.deepEqual(e.months.map((m) => m.amount), [33.33, 33.33, 33.34]);
  assert.equal(e.pdfKey, "travels-v2/x/engagements/y.pdf");
  assert.equal(e.signedAt, "2027-01-05T10:00:00Z");
  assert.equal(e.signerName, "Marie Durand");
  const free = buildEngagement({ inv: { id: INV, ine: "I", studentName: "X" }, priceBlock: { pricePerStudent: 0 }, signerName: "A B", signedAt: "x", pdfKey: "k" });
  assert.equal(free.amount, 0);
  assert.deepEqual(free.months, []);
});

/* ─── Vue publique : aucune fuite ─── */

test("vue publique : ni e-mail, ni empreinte, ni clé objet ; seulement les jours choisis par l'organisateur", () => {
  const inv = makeInv({
    firstOpenedAt: "x",
    tokenHashes: ["deadbeef".repeat(8)],
    documents: { d1: { fileName: "attestation.pdf", key: "travels-v2/secret/key.pdf", size: 1200, contentType: "application/pdf", uploadedAt: "x" } },
    code: { hash: "c0de".repeat(16), expiresAt: new Date(Date.now() + 60_000).toISOString(), attempts: 0, sentAt: "x" },
  });
  const trip = {
    status: "INSCRIPTIONS_OUVERTES",
    ownerName: "Mme Martin",
    imageUrl: undefined,
    data: {
      workflowVersion: 2 as const,
      title: "Voyage à Biarritz",
      startDate: "2027-03-08",
      endDate: "2027-03-12",
      etablissement: "Lycée",
      piqueNique: true,
      piqueNiqueDetails: { active: true, daysSelection: { "2027-03-10": true, "2027-03-11": false, "2027-03-09": true } },
      priceBlock: { pricePerStudent: 100, firstMonth: "2027-01", lastMonth: "2027-03" },
      familyPage: { documents: docs, deadline: "2027-01-31", openedAt: "2026-12-01T00:00:00Z" },
    },
  };
  const view = buildPublicFamilyView({ trip, inv, todayIso: "2027-01-10", maskedEmail: "pa•••@ex•••.org", siblingDecision: null });
  const json = JSON.stringify(view);
  assert.ok(!json.includes("parent@example.org"));
  assert.ok(!json.includes("deadbeef"));
  assert.ok(!json.includes("c0de"));
  assert.ok(!json.includes("travels-v2/secret"));
  assert.ok(!json.includes("tokenHashes"));
  assert.deepEqual(view.model.picnicDays.map((d) => d.key), ["2027-03-09", "2027-03-10"]);
  assert.equal(view.window.open, true);
  assert.equal(view.documents.find((d) => d.id === "d1")?.uploaded?.fileName, "attestation.pdf");
  assert.deepEqual(view.blockingDocIds, []);
  assert.equal(view.model.previewLabel, null);
  // Prolongation par famille : la date affichée est la date effective.
  const ext = buildPublicFamilyView({ trip, inv: { ...inv, extendedDeadline: "2027-02-10" }, todayIso: "2027-02-05", maskedEmail: "", siblingDecision: null });
  assert.equal(ext.model.deadline, "2027-02-10");
  assert.equal(ext.window.open, true);
  const late = buildPublicFamilyView({ trip, inv, todayIso: "2027-02-05", maskedEmail: "", siblingDecision: null });
  assert.equal(late.window.open, false);
});

/* ─── Voyages v1 inchangés ─── */

test("v1 : un voyage sans workflowVersion n'est pas v2 et n'a aucune fenêtre d'inscription", () => {
  assert.equal(isWorkflowV2({ data: {} }), false);
  assert.equal(isWorkflowV2({ data: { workflowVersion: undefined } }), false);
  const w = registrationWindow({ tripStatus: "VALIDE", openedAt: undefined, deadline: undefined, signed: false, todayIso: "2027-01-01" });
  assert.equal(w.open, false);
});

test("routes publiques : seules la page des familles et ses API sont déclarées, rien de plus large", () => {
  const list = PROXY_PUBLIC_ROUTE_MATCHERS as readonly string[];
  assert.ok(list.includes("/voyages/famille"));
  assert.ok(list.includes("/voyages/famille/(.*)"));
  assert.ok(list.includes("/api/voyages/famille/(.*)"));
  for (const wide of ["/voyages(.*)", "/api/voyages(.*)", "/api/travels(.*)", "/api/travels/(.*)", "/api/travels/v2(.*)", "/api/travels/v2/(.*)"]) {
    assert.ok(!list.includes(wide), `${wide} ne doit pas être public`);
  }
  assert.ok(!list.some((p) => p.startsWith("/api/travels/v2")), "l'API interne des inscriptions reste protégée");
});
