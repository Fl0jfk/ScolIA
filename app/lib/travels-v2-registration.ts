/**
 * Inscriptions des familles (circuit v2, lot 2) — jetons, code de signature, ICS, engagement, vue publique.
 * Fonctions SANS accès aux données (testables) ; le stockage est dans travels-v2-registration-store.ts.
 * Serveur uniquement (utilise `node:crypto`).
 */

import { createHash, randomBytes, randomInt, timingSafeEqual } from "node:crypto";
import { buildCalendarEventsIcs, type CalendarIcsEvent } from "@/app/lib/calendar-ics";
import { computeMonthlySchedule } from "@/app/lib/travels-billing-schedule";
import {
  buildFamilyPagePreviewModel,
  listPicnicDays,
  type FamilyPagePreviewModel,
} from "@/app/lib/travels-family-page";
import type { TravelsParentCalendar, TravelsPriceBlock, TravelsTrip } from "@/app/lib/travels-types";
import { tripDepartureIso, tripReturnIso } from "@/app/lib/travels-v2";
import { canUploadAfterSignature, type StudentState } from "@/app/lib/travels-v2-lot3";
import {
  CODE_MAX_ATTEMPTS,
  CODE_TTL_MS,
  MAX_AUX_TOKEN_HASHES,
  MAX_TOKEN_HASHES,
  computeInvitationStatus,
  isLocked,
  missingBlockingDocs,
  registrationWindow,
  signingProblems,
  type EngagementShown,
  type FamilyEngagement,
  type FamilyInvitation,
  type InvitationStatus,
  type RegistrationWindow,
  type SigningCodeState,
} from "@/app/lib/travels-v2-registration-types";

/* ─── Jetons personnels ─── */

const ID_RE = /^[A-Za-z0-9_-]{8,64}$/;
const SECRET_RE = /^[A-Za-z0-9_-]{43}$/;

export function sha256Hex(s: string): string {
  return createHash("sha256").update(s, "utf8").digest("hex");
}

/** Secret aléatoire de 32 octets (256 bits), encodé en base64url (43 caractères). */
export function generateInviteSecret(): string {
  return randomBytes(32).toString("base64url");
}

export function hashInviteSecret(secret: string): string {
  return sha256Hex(`invite:${secret}`);
}

/** Jeton du lien : `<invitation>.<voyage>.<secret>` — seul le secret est sensible, et il n'est jamais stocké. */
export function buildInviteToken(parts: { invitationId: string; tripId: string; secret: string }): string {
  return `${parts.invitationId}.${parts.tripId}.${parts.secret}`;
}

export type ParsedInviteToken = { invitationId: string; tripId: string; secret: string };

export function parseInviteToken(token: string | null | undefined): ParsedInviteToken | null {
  const raw = String(token || "").trim();
  if (!raw || raw.length > 200) return null;
  const parts = raw.split(".");
  if (parts.length !== 3) return null;
  const [invitationId, tripId, secret] = parts;
  if (!ID_RE.test(invitationId) || !ID_RE.test(tripId) || !SECRET_RE.test(secret)) return null;
  return { invitationId, tripId, secret };
}

function safeEqualHex(a: string, b: string): boolean {
  const ba = Buffer.from(a, "hex");
  const bb = Buffer.from(b, "hex");
  return ba.length === bb.length && ba.length > 0 && timingSafeEqual(ba, bb);
}

/** Vérifie un secret contre les empreintes stockées (comparaison en temps constant). */
export function verifyInviteSecret(tokenHashes: string[] | undefined, secret: string): boolean {
  if (!SECRET_RE.test(secret)) return false;
  const h = hashInviteSecret(secret);
  let ok = false;
  for (const stored of tokenHashes || []) {
    if (safeEqualHex(stored, h)) ok = true;
  }
  return ok;
}

/** Ajoute une empreinte (renvoi du lien) en gardant les plus récentes : les anciens liens restent valables. */
export function addTokenHash(tokenHashes: string[] | undefined, newHash: string): string[] {
  return [...(tokenHashes || []).filter((h) => h !== newHash), newHash].slice(-MAX_TOKEN_HASHES);
}

/** Idem pour les liens des relances et copies automatiques (emplacements séparés des liens principaux). */
export function addAuxTokenHash(auxHashes: string[] | undefined, newHash: string): string[] {
  return [...(auxHashes || []).filter((h) => h !== newHash), newHash].slice(-MAX_AUX_TOKEN_HASHES);
}

/* ─── Code de signature à 6 chiffres ─── */

export function generateSigningCode(): string {
  return String(randomInt(0, 1_000_000)).padStart(6, "0");
}

export function hashSigningCode(invitationId: string, code: string): string {
  return sha256Hex(`code:${invitationId}:${code}`);
}

export function issueSigningCode(invitationId: string, now: Date): { code: string; state: SigningCodeState } {
  const code = generateSigningCode();
  return {
    code,
    state: {
      hash: hashSigningCode(invitationId, code),
      expiresAt: new Date(now.getTime() + CODE_TTL_MS).toISOString(),
      attempts: 0,
      sentAt: now.toISOString(),
    },
  };
}

export type CodeCheck =
  | { result: "ok"; next: undefined }
  | { result: "none" | "expired" | "locked"; next: undefined }
  | { result: "wrong"; next: SigningCodeState; attemptsLeft: number };

/**
 * Vérifie le code saisi. `next` = nouvel état à stocker :
 * - ok → le code est consommé (usage unique) ;
 * - expiré ou trop d'essais → le code est supprimé (il faut en redemander un) ;
 * - faux → compteur d'essais incrémenté.
 */
export function checkSigningCode(
  invitationId: string,
  state: SigningCodeState | undefined,
  input: string,
  now: Date,
): CodeCheck {
  if (!state) return { result: "none", next: undefined };
  if (now.getTime() > new Date(state.expiresAt).getTime()) return { result: "expired", next: undefined };
  if (state.attempts >= CODE_MAX_ATTEMPTS) return { result: "locked", next: undefined };
  const given = hashSigningCode(invitationId, String(input || ""));
  if (safeEqualHex(given, state.hash)) return { result: "ok", next: undefined };
  const attempts = state.attempts + 1;
  if (attempts >= CODE_MAX_ATTEMPTS) return { result: "locked", next: undefined };
  return { result: "wrong", next: { ...state, attempts }, attemptsLeft: CODE_MAX_ATTEMPTS - attempts };
}

/* ─── ICS : séjour + rendez-vous départ et retour ─── */

function timeOrNull(t: unknown): string | null {
  const s = String(t || "").trim();
  return /^\d{2}:\d{2}$/.test(s) ? s : null;
}

export function buildFamilyIcs(opts: {
  tripId: string;
  invitationId: string;
  title: string;
  destination?: string;
  startDate: string | null;
  endDate: string | null;
  startTime?: string | null;
  endTime?: string | null;
  parentCalendar?: TravelsParentCalendar | null;
  studentName?: string;
}): string {
  const events: CalendarIcsEvent[] = [];
  const uidBase = `voyage-${opts.tripId}-${opts.invitationId}`;
  const points = Array.isArray(opts.parentCalendar?.points) ? opts.parentCalendar!.points : [];
  const depot = points.find((p) => p?.kind === "depot");
  const retour = points.find((p) => p?.kind === "recuperation");
  const who = opts.studentName ? ` — ${opts.studentName}` : "";

  if (opts.startDate) {
    const startTime = timeOrNull(opts.startTime) || timeOrNull(depot?.time) || "08:00";
    const endDate = opts.endDate && opts.endDate >= opts.startDate ? opts.endDate : opts.startDate;
    const endTime = timeOrNull(opts.endTime) || timeOrNull(retour?.time) || "18:00";
    const hasEnd = endDate > opts.startDate || endTime > startTime;
    events.push({
      title: `${opts.title}${who}`,
      description: opts.destination ? `Voyage scolaire — ${opts.destination}` : "Voyage scolaire",
      location: opts.destination || undefined,
      startDate: opts.startDate,
      startTime,
      ...(hasEnd ? { endDate, endTime } : { durationMinutes: 480 }),
      uid: `${uidBase}-sejour@scola`,
    });
  }
  if (depot?.date && timeOrNull(depot.time)) {
    events.push({
      title: `Rendez-vous de départ — ${opts.title}${who}`,
      description: depot.note || undefined,
      location: depot.place || undefined,
      startDate: depot.date,
      startTime: depot.time,
      durationMinutes: depot.durationMinutes && depot.durationMinutes > 0 ? depot.durationMinutes : 30,
      uid: `${uidBase}-depart@scola`,
      alarms: [{ trigger: "-P1D", description: `Départ demain — ${opts.title}` }],
    });
  }
  if (retour?.date && timeOrNull(retour.time)) {
    events.push({
      title: `Rendez-vous de retour — ${opts.title}${who}`,
      description: retour.note || undefined,
      location: retour.place || undefined,
      startDate: retour.date,
      startTime: retour.time,
      durationMinutes: retour.durationMinutes && retour.durationMinutes > 0 ? retour.durationMinutes : 30,
      uid: `${uidBase}-retour@scola`,
    });
  }
  return buildCalendarEventsIcs({ events, prodId: "-//Scola//Voyages//FR" });
}

/* ─── Engagement établissement ─── */

export function buildEngagement(opts: {
  inv: Pick<FamilyInvitation, "id" | "ine" | "studentName">;
  priceBlock: TravelsPriceBlock | null | undefined;
  signerName: string;
  signedAt: string;
  pdfKey: string;
  shown?: EngagementShown;
}): FamilyEngagement {
  const amount = Number(opts.priceBlock?.pricePerStudent);
  const safeAmount = Number.isFinite(amount) && amount >= 0 ? amount : 0;
  const schedule = computeMonthlySchedule(safeAmount, opts.priceBlock?.firstMonth, opts.priceBlock?.lastMonth);
  return {
    ine: opts.inv.ine,
    studentName: opts.inv.studentName,
    signerName: opts.signerName,
    invitationId: opts.inv.id,
    amount: safeAmount,
    firstMonth: opts.priceBlock?.firstMonth,
    lastMonth: opts.priceBlock?.lastMonth,
    months: schedule.ok ? schedule.months.map((m) => ({ label: m.label, amount: m.amount })) : [],
    signedAt: opts.signedAt,
    pdfKey: opts.pdfKey,
    ...(opts.shown ? { shown: opts.shown } : {}),
  };
}

/** Instantané de la page telle qu'affichée à la famille (prix, mois, pièces demandées, date limite). */
export function buildShownSnapshot(
  data: {
    priceBlock?: TravelsPriceBlock | null;
    familyPage?: { documents?: { label: string; kind: string; dueDate?: string }[]; program?: string; equipment?: string[]; cash?: { amount: number; reason: string } } | null;
  } | null | undefined,
  deadline: string | null | undefined,
  nowIso: string,
): EngagementShown {
  const pb = data?.priceBlock;
  const page = data?.familyPage;
  const price = Number(pb?.pricePerStudent);
  return {
    capturedAt: nowIso,
    pricePerStudent: pb && Number.isFinite(price) ? price : null,
    ...(pb?.firstMonth ? { firstMonth: pb.firstMonth } : {}),
    ...(pb?.lastMonth ? { lastMonth: pb.lastMonth } : {}),
    ...(pb?.paymentText ? { paymentText: String(pb.paymentText).slice(0, 500) } : {}),
    deadline: deadline || null,
    documents: (page?.documents || []).map((d) => ({ label: d.label, kind: d.kind, ...(d.dueDate ? { dueDate: d.dueDate } : {}) })),
    ...(page?.program ? { program: String(page.program).slice(0, 700) } : {}),
    ...(page?.equipment?.length ? { equipment: page.equipment.slice(0, 40) } : {}),
    ...(page?.cash ? { cash: { amount: Number(page.cash.amount), reason: String(page.cash.reason || "") } } : {}),
  };
}

/** L'élève est-il déjà engagé (« oui » signé) par une autre invitation (second parent) ? */
export function siblingDecision(
  invitations: Pick<FamilyInvitation, "id" | "ine" | "signature">[],
  inv: Pick<FamilyInvitation, "id" | "ine">,
): "oui" | "non" | null {
  const others = invitations.filter((o) => o.ine === inv.ine && o.id !== inv.id && o.signature);
  if (others.some((o) => o.signature?.decision === "oui")) return "oui";
  if (others.some((o) => o.signature?.decision === "non")) return "non";
  return null;
}

/* ─── Vue publique : uniquement ce qui concerne CETTE invitation ─── */

export type PublicFamilyView = {
  model: FamilyPagePreviewModel;
  studentName: string;
  maskedEmail: string;
  status: InvitationStatus;
  window: RegistrationWindow;
  locked: boolean;
  signed: null | { decision: "oui" | "non"; signedAt: string; signerName: string; hasPdf: boolean };
  answers: FamilyInvitation["answers"];
  /** Pièces : état de dépôt (jamais de clé objet). */
  documents: { id: string; uploaded: null | { fileName: string; size: number; uploadedAt: string }; exempt: boolean }[];
  blockingDocIds: string[];
  problems: string[];
  code: { pending: boolean; expiresAt: string | null };
  /** L'autre responsable a déjà répondu pour cet élève. */
  siblingDecision: "oui" | "non" | null;
  /** Situation de l'élève : un désaccord non tranché le met « en suspens » (message neutre, sans détail). */
  studentState: StudentState;
  /** Le responsable peut déclarer son désaccord (l'autre a inscrit l'élève). */
  canObject: boolean;
  /** Ce responsable a déjà déclaré son désaccord ; décision de la direction si elle est prise. */
  objection: null | { at: string; decision: "maintenir" | "retirer" | null };
  /** Pièces « avant le départ » encore déposables après la signature. */
  uploadableAfterSign: string[];
};

export function buildPublicFamilyView(opts: {
  trip: Pick<TravelsTrip, "data" | "imageUrl" | "ownerName" | "status">;
  inv: FamilyInvitation;
  todayIso: string;
  maskedEmail: string;
  siblingDecision: "oui" | "non" | null;
  studentState?: StudentState;
  canObject?: boolean;
  objectionDecision?: "maintenir" | "retirer" | null;
}): PublicFamilyView {
  const { trip, inv } = opts;
  const page = trip.data?.familyPage;
  const win = registrationWindow({
    tripStatus: trip.status,
    openedAt: page?.openedAt,
    deadline: page?.deadline,
    extendedDeadline: inv.extendedDeadline,
    signed: isLocked(inv),
    todayIso: opts.todayIso,
  });
  const model = buildFamilyPagePreviewModel(trip, {
    ...(win.deadline ? { familyPage: { deadline: win.deadline } } : {}),
  });
  const docs = page?.documents || [];
  const picnicKeys = listPicnicDays(trip.data?.piqueNiqueDetails).map((d) => d.key);
  const status = computeInvitationStatus({ inv, docs, picnicDayKeys: picnicKeys });
  const codeLive = inv.code && new Date(inv.code.expiresAt).getTime() > Date.now();
  return {
    model,
    studentName: inv.studentName,
    maskedEmail: opts.maskedEmail,
    status,
    window: win,
    locked: isLocked(inv),
    signed: inv.signature
      ? {
          decision: inv.signature.decision,
          signedAt: inv.signature.signedAt,
          signerName: inv.signature.signerName,
          hasPdf: Boolean(inv.signature.pdfKey),
        }
      : null,
    answers: inv.answers || {},
    documents: docs.map((d) => {
      const u = inv.documents?.[d.id];
      return {
        id: d.id,
        uploaded: u ? { fileName: u.fileName, size: u.size, uploadedAt: u.uploadedAt } : null,
        exempt: Boolean(inv.exemptions?.[d.id]),
      };
    }),
    blockingDocIds: missingBlockingDocs(docs, inv),
    problems: signingProblems({ answers: inv.answers || {}, picnicDayKeys: picnicKeys, docs, inv }),
    code: { pending: Boolean(codeLive), expiresAt: codeLive ? inv.code!.expiresAt : null },
    siblingDecision: opts.siblingDecision,
    studentState: opts.studentState ?? "aucun",
    canObject: Boolean(opts.canObject),
    objection: inv.objection ? { at: inv.objection.at, decision: opts.objectionDecision ?? null } : null,
    uploadableAfterSign: docs
      .filter((d) => canUploadAfterSignature(d, inv, opts.todayIso))
      .map((d) => d.id),
  };
}

/** Dates du séjour utiles à l'ICS et au PDF. */
export function tripDatesForIcs(trip: Pick<TravelsTrip, "data">) {
  return {
    startDate: tripDepartureIso(trip.data),
    endDate: tripReturnIso(trip.data),
    startTime: trip.data?.startTime ?? null,
    endTime: trip.data?.endTime ?? null,
  };
}
