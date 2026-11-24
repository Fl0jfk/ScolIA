/**
 * Circuit voyages v2 — mutations PURES d'une invitation, conçues pour s'exécuter DANS un verrou
 * (relecture de l'enregistrement + écriture dans la même transaction, voir `mutateInvitation` du store).
 *
 * Pourquoi : l'ancien schéma « lire au début de la requête, modifier en mémoire, tout réécrire » perdait des
 * mises à jour quand deux requêtes se croisaient (réponses écrasées, compteur d'essais du code contourné,
 * liens ou pièces effacés). Chaque fonction ci-dessous reçoit la version la PLUS RÉCENTE de l'invitation,
 * décide, et retourne `{ save, value }` : `save: false` = rien n'est écrit.
 * Aucune entrée-sortie ici (la transaction reste ouverte pendant l'exécution).
 */

import {
  addAuxTokenHash,
  addTokenHash,
  checkSigningCode,
  issueSigningCode,
  type CodeCheck,
} from "@/app/lib/travels-v2-registration";
import {
  canSendCode,
  isLocked,
  mergeAnswers,
  pruneCodeSends,
  registrationWindow,
  type FamilyInvitation,
  type RegistrationWindow,
  type SigningCodeState,
  type StoredFamilyDocument,
} from "@/app/lib/travels-v2-registration-types";

export type ClosedReason = "status" | "deadline" | "locked" | "not_opened";

export type Mut<R> = { save: boolean; value: R };

/** Ce qu'il faut savoir du voyage pour juger si la page est ouverte à CETTE invitation. */
export type WindowInput = {
  tripStatus: string | null | undefined;
  openedAt: string | null | undefined;
  deadline: string | null | undefined;
  todayIso: string;
};

export function windowOfInvitation(inv: FamilyInvitation, w: WindowInput): RegistrationWindow {
  return registrationWindow({
    tripStatus: w.tripStatus,
    openedAt: w.openedAt,
    deadline: w.deadline,
    extendedDeadline: inv.extendedDeadline,
    signed: isLocked(inv),
    todayIso: w.todayIso,
  });
}

/* ─── Réponses (brouillon) : fusion champ par champ sur la dernière version ─── */

export type AnswersOutcome =
  | { ok: true }
  | { ok: false; status: 409; reason: ClosedReason }
  | { ok: false; status: 400; error: string };

export function mutAnswers(
  inv: FamilyInvitation,
  o: { input: unknown; picnicDayKeys: string[]; nowIso: string; window: WindowInput },
): Mut<AnswersOutcome> {
  const win = windowOfInvitation(inv, o.window);
  if (!win.open) return { save: false, value: { ok: false, status: 409, reason: win.reason } };
  const merged = mergeAnswers({ current: inv.answers || {}, input: o.input, picnicDayKeys: o.picnicDayKeys, nowIso: o.nowIso });
  if (!merged.ok) return { save: false, value: { ok: false, status: 400, error: merged.error } };
  inv.answers = merged.answers;
  return { save: true, value: { ok: true } };
}

/* ─── Code de signature : envoi (réservation) et vérification (compteur d'essais) ─── */

export type CodeReservation = {
  code: string;
  state: SigningCodeState;
  /** État d'avant, pour défaire la réservation si l'e-mail n'est pas parti. */
  previous: { code: SigningCodeState | undefined; codeSends: string[] };
};

export type ReserveOutcome =
  | { ok: true; reservation: CodeReservation }
  | { ok: false; status: 409; reason: ClosedReason }
  | { ok: false; status: 400; error: string }
  | { ok: false; status: 429; retryAfterSec: number };

/**
 * Réserve un nouveau code : limites d'envoi (30 s, 5 / heure) contrôlées sur la dernière version, donc
 * impossibles à contourner par des requêtes parallèles. Le code n'est envoyé par e-mail qu'ENSUITE.
 */
export function mutReserveSigningCode(
  inv: FamilyInvitation,
  o: { now: Date; objection: boolean; window: WindowInput },
): Mut<ReserveOutcome> {
  if (!o.objection) {
    const win = windowOfInvitation(inv, o.window);
    if (!win.open) return { save: false, value: { ok: false, status: 409, reason: win.reason } };
    if (inv.answers?.participates !== "oui" && inv.answers?.participates !== "non") {
      return { save: false, value: { ok: false, status: 400, error: "Indiquez d'abord si votre enfant participe au voyage." } };
    }
  }
  const allowed = canSendCode(inv, o.now);
  if (!allowed.ok) return { save: false, value: { ok: false, status: 429, retryAfterSec: allowed.retryAfterSec } };
  const previous = { code: inv.code, codeSends: [...(inv.codeSends || [])] };
  const { code, state } = issueSigningCode(inv.id, o.now);
  inv.code = state;
  inv.codeSends = [...pruneCodeSends(inv.codeSends, o.now), o.now.toISOString()];
  return { save: true, value: { ok: true, reservation: { code, state, previous } } };
}

/** L'e-mail n'est pas parti : on défait la réservation (seulement si personne n'a redemandé un code entre-temps). */
export function mutRollbackSigningCode(inv: FamilyInvitation, r: CodeReservation): Mut<boolean> {
  if (inv.code?.hash !== r.state.hash) return { save: false, value: false };
  inv.code = r.previous.code;
  inv.codeSends = (inv.codeSends || []).filter((s) => s !== r.state.sentAt);
  return { save: true, value: true };
}

export type ConsumeOutcome =
  | { kind: "guard"; error: string }
  | { kind: "checked"; check: CodeCheck };

/**
 * Vérifie le code saisi sur la dernière version et PERSISTE le résultat dans le même verrou :
 * - faux → compteur d'essais +1 (5 essais au plus, quel que soit le nombre de requêtes parallèles) ;
 * - juste → code consommé (usage unique : un second envoi parallèle voit « aucun code ») ;
 * - expiré / trop d'essais → code supprimé.
 * `guard` (optionnel) est évalué AVANT : s'il refuse, aucun essai n'est consommé.
 */
export function mutConsumeSigningCode(
  inv: FamilyInvitation,
  o: { input: string; now: Date; guard?: (inv: FamilyInvitation) => string | null },
): Mut<ConsumeOutcome> {
  const refused = o.guard?.(inv);
  if (refused) return { save: false, value: { kind: "guard", error: refused } };
  const check = checkSigningCode(inv.id, inv.code, o.input, o.now);
  if (check.result === "none") return { save: false, value: { kind: "checked", check } };
  inv.code = check.result === "wrong" ? check.next : undefined;
  return { save: true, value: { kind: "checked", check } };
}

/* ─── Signature : pose atomique (une seule signature par invitation) ─── */

export type SignaturePatch = Pick<FamilyInvitation, "signature"> & Partial<Pick<FamilyInvitation, "engagement">>;

export function mutApplySignature(
  inv: FamilyInvitation,
  patch: SignaturePatch,
  win: WindowInput,
): Mut<{ ok: true } | { ok: false; error: string }> {
  if (inv.signature) return { save: false, value: { ok: false, error: "Cette réponse est déjà signée." } };
  const w = windowOfInvitation(inv, win);
  if (!w.open) return { save: false, value: { ok: false, error: "La page n'est plus ouverte aux réponses." } };
  inv.signature = patch.signature;
  if (patch.engagement) inv.engagement = patch.engagement;
  return { save: true, value: { ok: true } };
}

/* ─── Pièces déposées ─── */

export function mutSetDocument(
  inv: FamilyInvitation,
  docId: string,
  doc: StoredFamilyDocument,
  allowed: (inv: FamilyInvitation) => boolean,
): Mut<{ ok: true; previousKey?: string } | { ok: false }> {
  if (!allowed(inv)) return { save: false, value: { ok: false } };
  const previousKey = inv.documents?.[docId]?.key;
  inv.documents = { ...(inv.documents || {}), [docId]: doc };
  return { save: true, value: { ok: true, previousKey } };
}

export function mutRemoveDocument(
  inv: FamilyInvitation,
  docId: string,
  allowed: (inv: FamilyInvitation) => boolean,
): Mut<{ ok: true; previousKey?: string } | { ok: false }> {
  if (!allowed(inv)) return { save: false, value: { ok: false } };
  const previous = inv.documents?.[docId];
  if (!previous) return { save: false, value: { ok: true } };
  const rest = { ...inv.documents };
  delete rest[docId];
  inv.documents = rest;
  return { save: true, value: { ok: true, previousKey: previous.key } };
}

/* ─── Présence, liens, relances ─── */

/** Marque l'ouverture et une présence récente (au plus toutes les 5 minutes). */
export function mutTouchSeen(inv: FamilyInvitation, nowMs: number): Mut<void> {
  const last = inv.lastSeenAt ? new Date(inv.lastSeenAt).getTime() : 0;
  if (inv.firstOpenedAt && nowMs - last <= 5 * 60 * 1000) return { save: false, value: undefined };
  inv.firstOpenedAt = inv.firstOpenedAt || new Date(nowMs).toISOString();
  inv.lastSeenAt = new Date(nowMs).toISOString();
  return { save: true, value: undefined };
}

/** Ajoute l'empreinte d'un lien AVANT l'envoi de l'e-mail (le lien ne marche qu'une fois l'empreinte écrite). */
export function mutAddTokenHash(inv: FamilyInvitation, hash: string, aux: boolean): Mut<void> {
  if (aux) inv.auxTokenHashes = addAuxTokenHash(inv.auxTokenHashes, hash);
  else inv.tokenHashes = addTokenHash(inv.tokenHashes, hash);
  return { save: true, value: undefined };
}

/** L'e-mail n'est pas parti : on retire l'empreinte (pas de lien orphelin) et on note l'erreur. */
export function mutTokenSendFailed(inv: FamilyInvitation, hash: string, error: string | undefined): Mut<void> {
  inv.tokenHashes = (inv.tokenHashes || []).filter((h) => h !== hash);
  inv.auxTokenHashes = (inv.auxTokenHashes || []).filter((h) => h !== hash);
  if (error) inv.lastSendError = error;
  return { save: true, value: undefined };
}

export function mutTokenSendOk(inv: FamilyInvitation, nowIso: string, countAsSend: boolean): Mut<void> {
  if (countAsSend) {
    inv.sendCount = (inv.sendCount || 0) + 1;
    inv.firstSentAt = inv.firstSentAt || nowIso;
    inv.lastSentAt = nowIso;
    inv.lastSendError = undefined;
  }
  return { save: countAsSend, value: undefined };
}

/**
 * Réserve des clés de relance AVANT l'envoi (idempotence entre serveurs et entre passages) :
 * refuse si une des clés est déjà notée ou si une relance est déjà partie aujourd'hui.
 */
export function mutClaimReminders(
  inv: FamilyInvitation,
  keys: string[],
  todayIso: string,
): Mut<{ claimed: boolean; before: Record<string, string> | undefined }> {
  const done = inv.reminders || {};
  // Relance de date limite décidée sur une liste lue en début de passage : si la famille a signé entre-temps, on n'envoie rien.
  const staleDeadline = Boolean(inv.signature) && keys.some((k) => k.startsWith("deadline_"));
  if (staleDeadline || done.lastDay === todayIso || keys.some((k) => done[k])) {
    return { save: false, value: { claimed: false, before: inv.reminders } };
  }
  const before = inv.reminders;
  const next: Record<string, string> = { ...done };
  for (const k of keys) next[k] = todayIso;
  next.lastDay = todayIso;
  inv.reminders = next;
  return { save: true, value: { claimed: true, before } };
}

/** Défait une réservation de relance dont l'e-mail n'est pas parti (retire seulement NOS clés). */
export function mutUnclaimReminders(inv: FamilyInvitation, keys: string[], todayIso: string, before: Record<string, string> | undefined): Mut<void> {
  const cur = { ...(inv.reminders || {}) };
  for (const k of keys) if (cur[k] === todayIso) delete cur[k];
  if (before?.lastDay !== todayIso && cur.lastDay === todayIso) {
    if (before?.lastDay) cur.lastDay = before.lastDay;
    else delete cur.lastDay;
  }
  inv.reminders = cur;
  return { save: true, value: undefined };
}

/** Relance manuelle : au plus une par famille et par jour, réservée sous verrou. */
export function mutClaimManualReminder(inv: FamilyInvitation, todayIso: string): Mut<{ claimed: boolean; before: Record<string, string> | undefined }> {
  if (inv.reminders?.manual === todayIso) return { save: false, value: { claimed: false, before: inv.reminders } };
  const before = inv.reminders;
  inv.reminders = { ...(inv.reminders || {}), manual: todayIso };
  return { save: true, value: { claimed: true, before } };
}

export function mutRestoreReminders(inv: FamilyInvitation, before: Record<string, string> | undefined): Mut<void> {
  inv.reminders = before;
  return { save: true, value: undefined };
}
