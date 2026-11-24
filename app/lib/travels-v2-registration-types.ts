/**
 * Inscriptions des familles (circuit v2, lot 2) — types et règles pures, sans Node ni accès aux données.
 * Utilisable côté serveur ET côté navigateur (aucun import de `crypto`).
 */

import { z } from "zod";
import type { TravelsFamilyDocRequest } from "@/app/lib/travels-types";
import { isIsoDate } from "@/app/lib/travels-v2";

/* ─── Constantes ─── */

export const CODE_TTL_MS = 10 * 60 * 1000;
export const CODE_MAX_ATTEMPTS = 5;
/** Délai minimal entre deux envois de code pour une même invitation. */
export const CODE_MIN_INTERVAL_MS = 30 * 1000;
/** Nombre maximal d'envois de code par invitation sur une heure glissante. */
export const CODE_MAX_SENDS_PER_HOUR = 5;
export const MAX_TOKEN_HASHES = 5;
export const DOC_MAX_BYTES = 8 * 1024 * 1024;
export const SIGNATURE_MAX_BYTES = 200 * 1024;
export const PHONE_RE = /^[+0-9 ().-]{6,20}$/;

export const MEAL_TYPE_VALUES = ["classique", "sans_porc", "vegetarien"] as const;
export type MealType = (typeof MEAL_TYPE_VALUES)[number];
export const MEAL_TYPE_LABELS: Record<MealType, string> = {
  classique: "Classique",
  sans_porc: "Sans porc",
  vegetarien: "Végétarien",
};

/* ─── Modèle d'une invitation ─── */

export type MealChoice = { wants: "oui" | "non"; type?: MealType };

export type FamilyAnswers = {
  participates?: "oui" | "non";
  /** Clé = jour de pique-nique fixé par l'organisateur. */
  meals?: Record<string, MealChoice>;
  phone?: string;
  updatedAt?: string;
};

export type StoredFamilyDocument = {
  fileName: string;
  /** Clé objet PRIVÉE (jamais renvoyée à la famille). */
  key: string;
  size: number;
  contentType: string;
  uploadedAt: string;
};

export type SigningCodeState = {
  /** sha-256(invitationId:code) — le code n'est jamais stocké en clair. */
  hash: string;
  expiresAt: string;
  attempts: number;
  sentAt: string;
};

export type FamilySignature = {
  decision: "oui" | "non";
  signedAt: string;
  signerName: string;
  /** Clé objet privée du tracé PNG (oui seulement). */
  signatureKey?: string;
  /** Clé objet privée du PDF de l'accusé (oui seulement). */
  pdfKey?: string;
  ipHash?: string;
  /** Réponse enregistrée par l'organisateur à partir d'un coupon papier (le scan sert alors d'engagement). */
  paper?: { recordedBy: string; recordedAt: string; scanFileName: string; scanKey: string };
};

/** Désaccord d'un responsable (confirmé par code e-mail) après le « oui » de l'autre. */
export type FamilyObjection = {
  at: string;
  signerName: string;
  /** Visible de la direction seulement. */
  reason?: string;
};

export type FamilyEngagement = {
  ine: string;
  studentName: string;
  signerName: string;
  invitationId: string;
  amount: number;
  firstMonth?: string;
  lastMonth?: string;
  months: { label: string; amount: number }[];
  signedAt: string;
  pdfKey: string;
};

export type FamilyInvitation = {
  id: string;
  tripId: string;
  ine: string;
  studentName: string;
  classe?: string;
  /** E-mail du responsable (normalisé) — jamais renvoyé par les API publiques. */
  email: string;
  createdAt: string;
  /** Empreintes sha-256 des jetons valides (le jeton en clair n'est jamais stocké). */
  tokenHashes: string[];
  sendCount: number;
  firstSentAt?: string;
  lastSentAt?: string;
  lastSendError?: string;
  firstOpenedAt?: string;
  lastSeenAt?: string;
  answers: FamilyAnswers;
  documents: Record<string, StoredFamilyDocument>;
  /** Dispenses de pièce décidées par l'organisateur (par pièce). */
  exemptions: Record<string, { at: string; by: string }>;
  /** Prolongation de la date limite pour cette famille (YYYY-MM-DD). */
  extendedDeadline?: string;
  code?: SigningCodeState;
  /** Horodatages des envois de code (pour la limite horaire). */
  codeSends: string[];
  signature?: FamilySignature;
  engagement?: FamilyEngagement;
  confirmationMailSentAt?: string;
  confirmationMailError?: string;
  objection?: FamilyObjection;
  /** Désistement noté par l'organisateur APRÈS la clôture : l'élève sort de la liste, l'engagement reste archivé. */
  withdrawal?: { at: string; by: string; note?: string; kind?: "retire_direction" };
  /** Posé par la purge du lendemain du retour : l'enregistrement ne garde plus que l'engagement. */
  purgedAt?: string;
  /** Relances automatiques : clé -> jour (YYYY-MM-DD) d'envoi (idempotence). */
  reminders?: Record<string, string>;
};

/* ─── État d'une invitation ─── */

export type InvitationStatus =
  | "pas_ouvert"
  | "ouvert"
  | "en_cours"
  | "bloque_piece"
  | "pret"
  | "signe_oui"
  | "refus"
  | "desaccord"
  | "suspens"
  | "retire"
  | "desiste";

export const INVITATION_STATUS_LABELS: Record<InvitationStatus, string> = {
  pas_ouvert: "Pas ouvert",
  ouvert: "Ouvert",
  en_cours: "En cours",
  bloque_piece: "Bloqué par une pièce",
  pret: "Prêt à signer",
  signe_oui: "Signé — participe",
  refus: "Signé — ne participe pas",
  desaccord: "Désaccord déclaré",
  suspens: "En suspens (décision de la direction)",
  retire: "Retiré par la direction",
  desiste: "Désisté après la clôture",
};

export function isSigned(inv: Pick<FamilyInvitation, "signature">): boolean {
  return Boolean(inv.signature);
}

/** Après signature, la réponse est verrouillée. */
export function isLocked(inv: Pick<FamilyInvitation, "signature">): boolean {
  return isSigned(inv);
}

/** Pièces « avant signature » encore manquantes (ni déposées ni dispensées). */
export function missingBlockingDocs(
  docs: Pick<TravelsFamilyDocRequest, "id" | "kind">[] | undefined | null,
  inv: Pick<FamilyInvitation, "documents" | "exemptions">,
): string[] {
  return (docs || [])
    .filter((d) => d.kind === "avant_signature")
    .filter((d) => !inv.documents?.[d.id] && !inv.exemptions?.[d.id])
    .map((d) => d.id);
}

/** Jours de panier non encore renseignés (« oui » sans type n'existe pas : le type a un défaut serveur). */
export function missingMealDays(picnicDayKeys: string[], answers: FamilyAnswers): string[] {
  return picnicDayKeys.filter((k) => {
    const m = answers.meals?.[k];
    return !m || (m.wants !== "oui" && m.wants !== "non");
  });
}

export type SigningProblem = string;

/**
 * Ce qui empêche (ou non) de signer. La décision (oui/non) vient des réponses STOCKÉES côté serveur.
 * - « non » : aucune autre condition que d'avoir répondu ;
 * - « oui » : téléphone, jours de panier, pièces « avant signature ».
 */
export function signingProblems(opts: {
  answers: FamilyAnswers;
  picnicDayKeys: string[];
  docs: Pick<TravelsFamilyDocRequest, "id" | "kind" | "label">[] | undefined;
  inv: Pick<FamilyInvitation, "documents" | "exemptions">;
}): SigningProblem[] {
  const { answers, picnicDayKeys, docs, inv } = opts;
  if (answers.participates !== "oui" && answers.participates !== "non") {
    return ["Indiquez d'abord si votre enfant participe au voyage."];
  }
  if (answers.participates === "non") return [];
  const out: string[] = [];
  const phone = String(answers.phone || "").trim();
  if (!PHONE_RE.test(phone)) out.push("Indiquez un numéro de téléphone joignable pendant le voyage.");
  if (missingMealDays(picnicDayKeys, answers).length > 0) {
    out.push("Répondez pour chaque jour de panier repas.");
  }
  const missing = missingBlockingDocs(docs, inv);
  for (const id of missing) {
    const label = docs?.find((d) => d.id === id)?.label || "pièce";
    out.push(`Pièce obligatoire manquante : ${label}.`);
  }
  return out;
}

export function computeInvitationStatus(opts: {
  inv: FamilyInvitation;
  docs: Pick<TravelsFamilyDocRequest, "id" | "kind" | "label">[] | undefined;
  picnicDayKeys: string[];
}): InvitationStatus {
  const { inv, docs, picnicDayKeys } = opts;
  if (inv.signature) return inv.signature.decision === "oui" ? "signe_oui" : "refus";
  if (!inv.firstOpenedAt) return "pas_ouvert";
  const a = inv.answers || {};
  const touched =
    Boolean(a.participates) ||
    Boolean(a.phone) ||
    Object.keys(a.meals || {}).length > 0 ||
    Object.keys(inv.documents || {}).length > 0;
  if (!touched) return "ouvert";
  if (a.participates === "oui" && missingBlockingDocs(docs, inv).length > 0) return "bloque_piece";
  if (signingProblems({ answers: a, picnicDayKeys, docs, inv }).length === 0) return "pret";
  return "en_cours";
}

/* ─── Fermeture de la page ─── */

export type RegistrationWindow =
  | { open: true; deadline: string | null }
  | { open: false; reason: "status" | "deadline" | "locked" | "not_opened"; deadline: string | null };

/** Date limite effective : la prolongation de la famille l'emporte si elle est plus tardive. */
export function effectiveDeadline(
  deadline: string | null | undefined,
  extendedDeadline: string | null | undefined,
): string | null {
  const base = isIsoDate(deadline) ? deadline : null;
  const ext = isIsoDate(extendedDeadline) ? extendedDeadline : null;
  if (base && ext) return ext > base ? ext : base;
  return ext || base;
}

/**
 * Page ouverte aux réponses si : voyage v2 ouvert aux inscriptions (statut), pas verrouillée par une signature,
 * et date limite non dépassée (le jour de la date limite est inclus).
 * Une page sans date limite ne s'ouvre pas (la direction ne peut pas ouvrir sans date limite).
 */
export function registrationWindow(opts: {
  tripStatus: string | null | undefined;
  openedAt: string | null | undefined;
  deadline: string | null | undefined;
  extendedDeadline?: string | null;
  signed: boolean;
  todayIso: string;
}): RegistrationWindow {
  const deadline = effectiveDeadline(opts.deadline, opts.extendedDeadline);
  if (!opts.openedAt) return { open: false, reason: "not_opened", deadline };
  if (opts.signed) return { open: false, reason: "locked", deadline };
  if (opts.tripStatus !== "INSCRIPTIONS_OUVERTES") return { open: false, reason: "status", deadline };
  if (!deadline || opts.todayIso > deadline) return { open: false, reason: "deadline", deadline };
  return { open: true, deadline };
}

/* ─── Réponses : validation stricte des entrées ─── */

export const answersInputSchema = z
  .object({
    participates: z.enum(["oui", "non"]).optional(),
    meals: z
      .record(
        z.string().min(1).max(40),
        z.object({
          wants: z.enum(["oui", "non"]),
          type: z.enum(MEAL_TYPE_VALUES).optional(),
        }),
      )
      .optional(),
    phone: z.string().trim().max(20).optional(),
  })
  .strict();

export type AnswersInput = z.infer<typeof answersInputSchema>;

export type AnswersMergeResult = { ok: true; answers: FamilyAnswers } | { ok: false; error: string };

/**
 * Fusionne une saisie partielle dans les réponses existantes.
 * Les jours de panier inconnus (non fixés par l'organisateur) sont REFUSÉS : la famille ne peut pas en ajouter.
 */
export function mergeAnswers(opts: {
  current: FamilyAnswers;
  input: unknown;
  picnicDayKeys: string[];
  nowIso: string;
}): AnswersMergeResult {
  const parsed = answersInputSchema.safeParse(opts.input);
  if (!parsed.success) return { ok: false, error: "Réponse invalide." };
  const v = parsed.data;
  const next: FamilyAnswers = { ...opts.current };
  if (v.participates) next.participates = v.participates;
  if (v.phone !== undefined) {
    if (v.phone !== "" && !PHONE_RE.test(v.phone)) return { ok: false, error: "Numéro de téléphone invalide." };
    next.phone = v.phone;
  }
  if (v.meals) {
    const allowed = new Set(opts.picnicDayKeys);
    const meals: Record<string, MealChoice> = { ...(opts.current.meals || {}) };
    for (const [day, choice] of Object.entries(v.meals)) {
      if (!allowed.has(day)) return { ok: false, error: "Ce jour ne fait pas partie des paniers proposés." };
      meals[day] =
        choice.wants === "oui" ? { wants: "oui", type: choice.type ?? "classique" } : { wants: "non" };
    }
    next.meals = meals;
  }
  next.updatedAt = opts.nowIso;
  return { ok: true, answers: next };
}

/* ─── Signature : entrées ─── */

export const signInputSchema = z
  .object({
    signerName: z.string().trim().min(2).max(100),
    code: z.string().regex(/^\d{6}$/),
    /** Tracé : data URL PNG (obligatoire pour « oui », ignoré pour « non »). */
    signaturePng: z.string().max(400_000).optional(),
  })
  .strict();

/** Décode un data URL PNG ; vérifie en-tête PNG, taille et dimensions raisonnables. */
export function decodeSignaturePng(
  dataUrl: string | undefined,
): { ok: true; bytes: Buffer } | { ok: false; error: string } {
  const m = /^data:image\/png;base64,([A-Za-z0-9+/=]+)$/.exec(String(dataUrl || ""));
  if (!m) return { ok: false, error: "Tracez votre signature." };
  const bytes = Buffer.from(m[1], "base64");
  if (bytes.length < 100 || bytes.length > SIGNATURE_MAX_BYTES) {
    return { ok: false, error: "Signature trop volumineuse ou vide." };
  }
  const PNG_MAGIC = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
  if (!PNG_MAGIC.every((b, i) => bytes[i] === b)) return { ok: false, error: "Signature invalide." };
  const w = bytes.readUInt32BE(16);
  const h = bytes.readUInt32BE(20);
  if (!(w > 0 && h > 0 && w <= 1600 && h <= 800)) return { ok: false, error: "Signature invalide." };
  return { ok: true, bytes };
}

/* ─── Pièces déposées : type réel du fichier (octets d'en-tête) ─── */

export type AllowedDocType = { ext: string; contentType: string };

/** PDF ou image uniquement, reconnus par leurs premiers octets (le type annoncé par le navigateur n'est pas cru). */
export function sniffAllowedDocType(bytes: Uint8Array): AllowedDocType | null {
  const b = bytes;
  const at = (i: number, s: string) => s.split("").every((c, k) => b[i + k] === c.charCodeAt(0));
  if (b.length < 12) return null;
  if (at(0, "%PDF")) return { ext: "pdf", contentType: "application/pdf" };
  if (b[0] === 0x89 && at(1, "PNG")) return { ext: "png", contentType: "image/png" };
  if (b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff) return { ext: "jpg", contentType: "image/jpeg" };
  if (at(0, "RIFF") && at(8, "WEBP")) return { ext: "webp", contentType: "image/webp" };
  if (at(4, "ftyp") && (at(8, "heic") || at(8, "heix") || at(8, "mif1") || at(8, "heif"))) {
    return { ext: "heic", contentType: "image/heic" };
  }
  return null;
}

/* ─── Code de signature : limites d'envoi ─── */

export function canSendCode(
  inv: Pick<FamilyInvitation, "codeSends" | "code">,
  now: Date,
): { ok: true } | { ok: false; retryAfterSec: number } {
  const sends = (inv.codeSends || []).map((s) => new Date(s).getTime()).filter((t) => Number.isFinite(t));
  const last = sends.length ? Math.max(...sends) : 0;
  const sinceLast = now.getTime() - last;
  if (last && sinceLast < CODE_MIN_INTERVAL_MS) {
    return { ok: false, retryAfterSec: Math.ceil((CODE_MIN_INTERVAL_MS - sinceLast) / 1000) };
  }
  const hourAgo = now.getTime() - 60 * 60 * 1000;
  const recent = sends.filter((t) => t > hourAgo).sort((a, b) => a - b);
  if (recent.length >= CODE_MAX_SENDS_PER_HOUR) {
    return { ok: false, retryAfterSec: Math.max(1, Math.ceil((recent[0] + 60 * 60 * 1000 - now.getTime()) / 1000)) };
  }
  return { ok: true };
}

/** Horodatages des envois conservés : uniquement ceux de la dernière heure. */
export function pruneCodeSends(sends: string[] | undefined, now: Date): string[] {
  const hourAgo = now.getTime() - 60 * 60 * 1000;
  return (sends || []).filter((s) => new Date(s).getTime() > hourAgo);
}
