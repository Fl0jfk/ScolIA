import { test } from "node:test";
import assert from "node:assert/strict";
import {
  AUTO_REMINDER_CAP_PER_PASS,
  autoReminderAudience,
  createSendBudget,
  planInvitationReminders,
  selectReminderTargets,
  studentState,
  isRemindable,
  type ReminderRowState,
  type ReminderTripState,
} from "@/app/lib/travels-v2-lot3";
import { closureSideEffectPlan, isDeadTripStatus, isTripPurgeDue, tripPurgeDueOn } from "@/app/lib/travels-v2-lot4";
import { buildShownSnapshot } from "@/app/lib/travels-v2-registration";
import { tokenAccessFor, type FamilyInvitation } from "@/app/lib/travels-v2-registration-types";
import { canRenderInPdf, pdfSafeDeep, toPdfSafe } from "@/app/lib/travels-v2-pdf-text";

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

const OPEN: ReminderTripState = { open: true, closed: false, purged: false, inPreListe: true };
const row = (over: Partial<ReminderRowState> = {}): ReminderRowState => ({
  invitationId: "I1",
  status: "ouvert",
  signed: false,
  official: false,
  ...over,
});

/* ─── B3 : à qui on relance ─── */

test("B3 une famille qui a encore à répondre est relancée", () => {
  assert.deepEqual(autoReminderAudience(row(), OPEN), { deadline: true, docs: false });
});

test("B3 pas de relance si l'autre responsable a déjà signé (oui ou non)", () => {
  assert.equal(autoReminderAudience(row({ otherSigned: true }), OPEN).deadline, false);
  // même chaîne que le suivi : le statut de la ligne reste « ouvert » mais otherSigned l'écarte aussi des relances manuelles
  assert.equal(isRemindable({ ...row({ otherSigned: true }) }), false);
  const rows = [row({ otherSigned: true, invitationId: "X1" }), row({ invitationId: "X2" })];
  assert.deepEqual(
    selectReminderTargets(rows.map((r) => ({ ...r, exempted: false, paper: false })), { status: "ouvert" }).map((r) => r.invitationId),
    ["X2"],
  );
});

test("B3 pas de relance si l'élève est retiré, désisté ou en suspens", () => {
  for (const status of ["retire", "desiste", "suspens", "desaccord"] as const) {
    assert.deepEqual(autoReminderAudience(row({ status }), OPEN), { deadline: false, docs: false }, status);
  }
});

test("B3 pas de relance de date limite si la famille a déjà signé ni si l'élève n'est plus dans la pré-liste", () => {
  assert.equal(autoReminderAudience(row({ signed: true, status: "signe_oui" }), OPEN).deadline, false);
  assert.equal(autoReminderAudience(row(), { ...OPEN, inPreListe: false }).deadline, false);
});

test("B3 voyage clôturé ou purgé : plus de rappel de date limite ; pièces seulement pour les inscrits actifs", () => {
  const closed: ReminderTripState = { open: false, closed: true, purged: false, inPreListe: true };
  assert.equal(autoReminderAudience(row(), closed).deadline, false);
  const official = row({ signed: true, status: "signe_oui", official: true });
  assert.deepEqual(autoReminderAudience(official, closed), { deadline: false, docs: true });
  const desiste = row({ signed: true, status: "desiste", official: false });
  assert.equal(autoReminderAudience(desiste, closed).docs, false);
  const purged: ReminderTripState = { open: false, closed: true, purged: true, inPreListe: true };
  assert.deepEqual(autoReminderAudience(official, purged), { deadline: false, docs: false });
});

test("B3 rappels de pièces : un élève retiré ou désisté ne reçoit rien, un inscrit actif oui (planInvitationReminders)", () => {
  const docs = [{ id: "d1", label: "Carte d'identité", kind: "avant_depart" as const, dueDate: "2027-03-10" }];
  const signed = inv({ id: "A1", ine: "I1", signature: yes });
  const plan = (audience: { deadline: boolean; docs: boolean }) =>
    planInvitationReminders({ inv: signed, deadline: "2027-02-20", docs, todayIso: "2027-03-08", windowOpen: false, audience });
  assert.ok(plan({ deadline: false, docs: true }), "inscrit actif : relancé");
  assert.equal(plan({ deadline: false, docs: false }), null, "retiré / désisté : rien");
});

test("B3 rappel de date limite : le plan n'existe pas si l'audience l'exclut, même fenêtre ouverte", () => {
  const unsigned = inv({ id: "A1", ine: "I1" });
  const args = { inv: unsigned, deadline: "2027-02-20", docs: [], todayIso: "2027-02-19", windowOpen: true };
  assert.ok(planInvitationReminders({ ...args, audience: { deadline: true, docs: false } }));
  assert.equal(planInvitationReminders({ ...args, audience: { deadline: false, docs: false } }), null);
  assert.ok(planInvitationReminders(args), "sans audience : comportement d'origine conservé");
});

test("B3 scénarios métier de bout en bout avec l'état partagé (studentState / signatures de l'autre parent)", () => {
  const a = inv({ id: "A1", ine: "I1", signature: yes });
  const b = inv({ id: "B1", ine: "I1" });
  // l'autre parent a signé « oui » : b n'a rien à faire
  assert.equal(studentState([a, b], null), "inscrit");
  const otherSigned = [a, b].some((o) => o.id !== b.id && Boolean(o.signature));
  assert.equal(autoReminderAudience(row({ otherSigned }), OPEN).deadline, false);
  // l'autre a signé « non » : idem
  const c = inv({ id: "C1", ine: "I2", signature: no });
  const d = inv({ id: "D1", ine: "I2" });
  const otherSignedNo = [c, d].some((o) => o.id !== d.id && Boolean(o.signature));
  assert.equal(autoReminderAudience(row({ otherSigned: otherSignedNo }), OPEN).deadline, false);
});

/* ─── B3 : plafond d'e-mails ─── */

test("B3 plafond : au-delà, on s'arrête (budget épuisé et signalé)", () => {
  const b = createSendBudget(3);
  assert.equal(b.take(), true);
  assert.equal(b.take(), true);
  assert.equal(b.take(), true);
  assert.equal(b.exhausted, false);
  assert.equal(b.take(), false);
  assert.equal(b.exhausted, true);
  assert.equal(b.used, 3);
  assert.equal(AUTO_REMINDER_CAP_PER_PASS, 300);
});

test("B3 plafond : un passage de 1000 familles n'envoie pas plus de 300 e-mails", () => {
  const b = createSendBudget(AUTO_REMINDER_CAP_PER_PASS);
  let sent = 0;
  for (let i = 0; i < 1000; i++) {
    if (!b.take()) break;
    sent += 1;
  }
  assert.equal(sent, 300);
  assert.equal(b.exhausted, true);
});

/* ─── M1 : la clôture automatique n'envoie rien à l'extérieur ─── */

test("M1 clôture automatique : ZIP et organisateur, mais ni liste transporteur ni e-mail compta", () => {
  const auto = closureSideEffectPlan({}, true);
  assert.deepEqual(auto, { zip: true, transport: false, compta: false, organizer: true });
});

test("M1 après « Confirmer et envoyer » (ou clôture manuelle) : la liste part au transporteur et la compta est prévenue", () => {
  const confirmed = closureSideEffectPlan({ sendConfirmedAt: "2027-02-22T10:00:00Z", comptaZip: {}, organizerNotifiedAt: "x" }, true);
  assert.deepEqual(confirmed, { zip: false, transport: true, compta: true, organizer: false });
  const noBus = closureSideEffectPlan({ sendConfirmedAt: "2027-02-22T10:00:00Z" }, false);
  assert.equal(noBus.transport, false, "pas de bus : pas de liste transporteur");
  assert.equal(noBus.compta, true);
});

test("M1 chaque envoi n'a lieu qu'une fois (idempotence)", () => {
  const done = closureSideEffectPlan(
    { sendConfirmedAt: "t", comptaZip: {}, transportListSentAt: "t", comptaMailSentAt: "t", organizerNotifiedAt: "t" },
    true,
  );
  assert.deepEqual(done, { zip: false, transport: false, compta: false, organizer: false });
});

/* ─── M2 : purge des voyages annulés, sans date de retour ; accès par jeton ─── */

test("M2 voyage terminé : purge le lendemain du retour", () => {
  assert.equal(tripPurgeDueOn({ status: "VALIDE", opened: true, returnIso: "2027-03-24", departureIso: "2027-03-20" }), "2027-03-25");
  assert.equal(isTripPurgeDue({ status: "VALIDE", opened: true, returnIso: "2027-03-24", departureIso: "2027-03-20", todayIso: "2027-03-24" }), false);
  assert.equal(isTripPurgeDue({ status: "VALIDE", opened: true, returnIso: "2027-03-24", departureIso: "2027-03-20", todayIso: "2027-03-25" }), true);
});

test("M2 voyage sans date de retour valide : purge le lendemain de la date de départ", () => {
  assert.equal(tripPurgeDueOn({ status: "VALIDE", opened: true, returnIso: null, departureIso: "2027-03-20" }), "2027-03-21");
  assert.equal(tripPurgeDueOn({ status: "VALIDE", opened: true, returnIso: "pas-une-date", departureIso: "2027-03-20" }), "2027-03-21");
});

test("M2 voyage annulé ou refusé après ouverture : le lendemain de l'annulation ou du retour prévu, le plus tôt", () => {
  for (const status of ["ANNULE", "REJETE", "SEANCE_ANNULEE"]) {
    assert.ok(isDeadTripStatus(status));
    // annulé bien avant le retour → lendemain de l'annulation
    assert.equal(
      tripPurgeDueOn({ status, opened: true, returnIso: "2027-03-24", departureIso: "2027-03-20", cancelledOnIso: "2027-02-10" }),
      "2027-02-11",
    );
    // annulé après le retour prévu → lendemain du retour prévu
    assert.equal(
      tripPurgeDueOn({ status, opened: true, returnIso: "2027-03-24", departureIso: "2027-03-20", cancelledOnIso: "2027-04-10" }),
      "2027-03-25",
    );
  }
  assert.equal(
    isTripPurgeDue({ status: "ANNULE", opened: true, returnIso: "2027-03-24", departureIso: "2027-03-20", cancelledOnIso: "2027-02-10", todayIso: "2027-02-11" }),
    true,
  );
  assert.equal(
    isTripPurgeDue({ status: "ANNULE", opened: true, returnIso: "2027-03-24", departureIso: "2027-03-20", cancelledOnIso: "2027-02-10", todayIso: "2027-02-10" }),
    false,
  );
});

test("M2 un voyage dont la page n'a jamais été ouverte n'est pas concerné ; un voyage vivant n'est pas purgé avant son retour", () => {
  assert.equal(tripPurgeDueOn({ status: "ANNULE", opened: false, returnIso: "2027-03-24", departureIso: "2027-03-20", cancelledOnIso: "2027-02-10" }), null);
  assert.equal(
    isTripPurgeDue({ status: "INSCRIPTIONS_OUVERTES", opened: true, returnIso: "2027-03-24", departureIso: "2027-03-20", cancelledOnIso: "2027-02-10", todayIso: "2027-03-01" }),
    false,
    "le jour de modification ne compte que pour un voyage annulé",
  );
});

test("M2 jetons : plus rien hors inscriptions ouvertes, sauf le PDF déjà signé", () => {
  assert.equal(tokenAccessFor({ tripStatus: "INSCRIPTIONS_OUVERTES", signedYes: false, withdrawn: false }), "full");
  assert.equal(tokenAccessFor({ tripStatus: "ANNULE", signedYes: false, withdrawn: false }), "none");
  assert.equal(tokenAccessFor({ tripStatus: "ANNULE", signedYes: true, withdrawn: false }), "pdf_only");
  assert.equal(tokenAccessFor({ tripStatus: "REJETE", signedYes: true, withdrawn: false }), "pdf_only");
  assert.equal(tokenAccessFor({ tripStatus: "EN_ATTENTE_DIR_FINAL", signedYes: false, withdrawn: false }), "none");
  assert.equal(tokenAccessFor({ tripStatus: "VALIDE", signedYes: false, withdrawn: false }), "none");
  // voyage clôturé : la famille inscrite garde sa page (pièces « avant départ »), une famille désistée n'a plus que le PDF
  assert.equal(tokenAccessFor({ tripStatus: "VALIDE", signedYes: true, withdrawn: false }), "signed_only");
  assert.equal(tokenAccessFor({ tripStatus: "VALIDE", signedYes: true, withdrawn: true }), "pdf_only");
});

/* ─── M3 : instantané de ce qui était affiché ─── */

test("M3 l'engagement garde un instantané du prix, des mois, des pièces et de la date limite", () => {
  const shown = buildShownSnapshot(
    {
      priceBlock: { pricePerStudent: 120, firstMonth: "2026-12", lastMonth: "2027-07", paymentText: "Prélèvement" },
      familyPage: { documents: [{ label: "Autorisation", kind: "avant_signature" }, { label: "Carte", kind: "avant_depart", dueDate: "2027-03-10" }], program: "Visite" },
    },
    "2027-02-20",
    "2027-02-01T10:00:00Z",
  );
  assert.equal(shown.pricePerStudent, 120);
  assert.equal(shown.firstMonth, "2026-12");
  assert.equal(shown.lastMonth, "2027-07");
  assert.equal(shown.deadline, "2027-02-20");
  assert.equal(shown.documents.length, 2);
  assert.equal(shown.documents[1].dueDate, "2027-03-10");
  assert.equal(shown.capturedAt, "2027-02-01T10:00:00Z");
});

/* ─── M4 : PDF — texte représentable ─── */

test("M4 accents français conservés, lettres latines étendues ramenées à leur base, alphabets non latins refusés", () => {
  assert.deepEqual(toPdfSafe("Hélène Dupré-Çelik œuvre à Noël €"), { text: "Hélène Dupré-Çelik œuvre à Noël €", lossy: false });
  assert.deepEqual(toPdfSafe("Łukasz Dąbrowski"), { text: "Lukasz Dabrowski", lossy: false });
  assert.equal(canRenderInPdf("Mohamed El Idrissi"), true);
  assert.equal(canRenderInPdf("Иван Петров"), false);
  assert.equal(canRenderInPdf("محمد"), false);
  assert.equal(canRenderInPdf("李小龍"), false);
  assert.equal(toPdfSafe("Иван").text, "????");
});

test("M4 pdfSafeDeep nettoie toutes les chaînes d'une structure sans toucher aux Buffer", () => {
  const png = Buffer.from([1, 2, 3]);
  const out = pdfSafeDeep({ a: "Łódź", b: ["ñ", { c: "Ж" }], png });
  assert.equal(out.a, "Lódz");
  assert.deepEqual(out.b, ["ñ", { c: "?" }]);
  assert.equal(out.png, png);
});
