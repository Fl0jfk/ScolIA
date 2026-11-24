/**
 * Circuit voyages v2, lot 3 — logique PURE (sans accès aux données, sans Node) :
 * désaccord entre responsables, unicité de l'engagement, relances, statuts du suivi, filtres, export.
 * Les aides financières sont gérées HORS de l'application (décision du 24/11) : rien ici à ce sujet.
 */

import type { TravelsFamilyDocRequest } from "@/app/lib/travels-types";
import { diffDaysIso, isIsoDate } from "@/app/lib/travels-v2";
import {
  effectiveDeadline,
  type FamilyInvitation,
  type InvitationStatus,
} from "@/app/lib/travels-v2-registration-types";

/* ─── Dossier élève : désaccord et décision de la direction ─── */

export type CaseEvent = {
  at: string;
  by: string;
  action: "objection" | "maintenir" | "retirer";
  note?: string;
};

export type StudentCase = {
  id: string;
  tripId: string;
  ine: string;
  decision?: { kind: "maintenir" | "retirer"; at: string; by: string; note?: string };
  history: CaseEvent[];
};

export type StudentState = "aucun" | "inscrit" | "suspens" | "retire" | "desiste";

/**
 * Situation d'un élève (toutes ses invitations + dossier de décision) :
 * - aucun « oui » signé → aucun ;
 * - « retirer » décidé → retiré ;
 * - un désaccord non tranché → en suspens (PAS dans la liste officielle) ;
 * - sinon → inscrit.
 */
export function studentState(
  invs: Pick<FamilyInvitation, "signature" | "objection" | "withdrawal">[],
  caseRec: Pick<StudentCase, "decision"> | null | undefined,
): StudentState {
  if (!invs.some((i) => i.signature?.decision === "oui")) return "aucun";
  if (invs.some((i) => i.withdrawal)) return "desiste";
  if (caseRec?.decision?.kind === "retirer") return "retire";
  if (!caseRec?.decision && invs.some((i) => i.objection)) return "suspens";
  return "inscrit";
}

export type ObjectionCheck = { ok: true } | { ok: false; error: string };

/**
 * Un responsable peut s'opposer si : l'autre a signé « oui », lui-même n'a pas signé « oui »,
 * n'a pas déjà fait opposition, et la direction n'a pas encore tranché.
 */
export function canObject(opts: {
  inv: Pick<FamilyInvitation, "id" | "ine" | "signature" | "objection">;
  siblings: Pick<FamilyInvitation, "id" | "ine" | "signature" | "objection">[];
  caseRec: Pick<StudentCase, "decision"> | null | undefined;
}): ObjectionCheck {
  const { inv, siblings, caseRec } = opts;
  const others = siblings.filter((s) => s.ine === inv.ine && s.id !== inv.id);
  if (!others.some((s) => s.signature?.decision === "oui")) {
    return { ok: false, error: "Aucune inscription de l'autre responsable à contester." };
  }
  if (inv.signature?.decision === "oui") return { ok: false, error: "Vous avez vous-même inscrit cet élève." };
  if (inv.objection) return { ok: false, error: "Votre désaccord a déjà été transmis à la direction." };
  if (caseRec?.decision) return { ok: false, error: "La direction a déjà tranché." };
  return { ok: true };
}

export function applyObjection(
  caseRec: StudentCase | null,
  base: { tripId: string; ine: string },
  by: string,
  atIso: string,
  note?: string,
): StudentCase {
  const rec: StudentCase = caseRec ?? { id: caseId(base.ine), tripId: base.tripId, ine: base.ine, history: [] };
  return { ...rec, history: [...rec.history, { at: atIso, by, action: "objection", ...(note ? { note } : {}) }] };
}

export type DecisionCheck = { ok: true; next: StudentCase } | { ok: false; error: string };

/** Décision de la direction : seulement s'il y a un désaccord non tranché ; historisée. */
export function applyDecision(opts: {
  caseRec: StudentCase | null;
  invs: Pick<FamilyInvitation, "signature" | "objection">[];
  kind: "maintenir" | "retirer";
  by: string;
  atIso: string;
  note?: string;
}): DecisionCheck {
  const { caseRec, invs, kind, by, atIso } = opts;
  if (studentState(invs, caseRec) !== "suspens" || !caseRec) {
    return { ok: false, error: "Cet élève n'est pas en suspens." };
  }
  const note = opts.note?.trim().slice(0, 500) || undefined;
  return {
    ok: true,
    next: {
      ...caseRec,
      decision: { kind, at: atIso, by, ...(note ? { note } : {}) },
      history: [...caseRec.history, { at: atIso, by, action: kind, ...(note ? { note } : {}) }],
    },
  };
}

/** Identifiant d'enregistrement stable et sûr (INE → majuscules alphanumériques). */
export function caseId(ine: string): string {
  return String(ine || "").toUpperCase().replace(/[^A-Z0-9]/g, "");
}

/** Liste officielle provisoire : INE inscrits (hors suspens et retirés). */
export function officialInes(invs: FamilyInvitation[], cases: StudentCase[]): string[] {
  const byCase = new Map(cases.map((c) => [caseId(c.ine), c]));
  const byIne = new Map<string, FamilyInvitation[]>();
  for (const i of invs) byIne.set(i.ine, [...(byIne.get(i.ine) || []), i]);
  return [...byIne.entries()]
    .filter(([ine, list]) => studentState(list, byCase.get(caseId(ine))) === "inscrit")
    .map(([ine]) => ine);
}

/* ─── Unicité de l'engagement (un seul « oui » par élève) ─── */

export type EngagementClaim = { id: string; tripId: string; ine: string; invitationId: string; claimedAt: string };

export type ClaimOutcome = "free" | "mine" | "taken" | "stale";

/**
 * Verdict sur le jeton d'engagement existant d'un élève.
 * - aucun jeton → free (à prendre) ;
 * - jeton de cette invitation → mine (reprise idempotente) ;
 * - jeton d'une autre invitation qui a signé « oui » → taken ;
 * - jeton d'une autre invitation qui n'a PAS signé (signature interrompue) → stale (à reprendre).
 */
export function evaluateClaim(
  existing: Pick<EngagementClaim, "invitationId"> | null | undefined,
  invitationId: string,
  holderSignedYes: boolean,
): ClaimOutcome {
  if (!existing) return "free";
  if (existing.invitationId === invitationId) return "mine";
  return holderSignedYes ? "taken" : "stale";
}

/* ─── Relances automatiques ─── */

export type ReminderKind = "deadline_j3" | "deadline_j1" | `doc_${string}_j${7 | 3 | 1}`;

export type ReminderPlan = {
  invitationId: string;
  /** Clés d'idempotence à marquer « envoyé aujourd'hui ». */
  keys: string[];
  /** Un seul e-mail par famille et par jour : motifs regroupés. */
  deadline?: { daysLeft: number; deadline: string };
  docs: { id: string; label: string; dueDate: string }[];
};

const DOC_OFFSETS = [7, 3, 1] as const;

/**
 * Relances à envoyer AUJOURD'HUI pour une invitation. Pure et idempotente : une clé déjà notée dans
 * `inv.reminders` n'est jamais rejouée, et au plus un e-mail par jour (`reminders.lastDay`).
 * Rattrapage si le serveur a manqué le jour exact (J-3 envoyé à J-2, J-1 le jour même…).
 */
export function planInvitationReminders(opts: {
  inv: FamilyInvitation;
  deadline: string | null | undefined;
  docs: Pick<TravelsFamilyDocRequest, "id" | "kind" | "label" | "dueDate">[];
  todayIso: string;
  windowOpen: boolean;
}): ReminderPlan | null {
  const { inv, todayIso } = opts;
  const done = inv.reminders || {};
  if (done.lastDay === todayIso) return null;
  if (!inv.firstSentAt) return null; // lien jamais parti : pas de relance
  const keys: string[] = [];
  let deadlinePlan: ReminderPlan["deadline"];
  const docsPlan: ReminderPlan["docs"] = [];

  if (!inv.signature && opts.windowOpen) {
    const dl = effectiveDeadline(opts.deadline, inv.extendedDeadline);
    if (dl) {
      const left = diffDaysIso(todayIso, dl);
      if (left >= 0 && left <= 1 && !done.deadline_j1) {
        keys.push("deadline_j1");
        deadlinePlan = { daysLeft: left, deadline: dl };
      } else if (left > 1 && left <= 3 && !done.deadline_j3) {
        keys.push("deadline_j3");
        deadlinePlan = { daysLeft: left, deadline: dl };
      }
    }
  }

  if (inv.signature?.decision === "oui") {
    for (const d of opts.docs) {
      if (d.kind !== "avant_depart" || !isIsoDate(d.dueDate)) continue;
      if (inv.documents?.[d.id] || inv.exemptions?.[d.id]) continue;
      const left = diffDaysIso(todayIso, d.dueDate);
      if (left < 0) continue;
      const offset = DOC_OFFSETS.find((o, i) => left <= o && (i === DOC_OFFSETS.length - 1 || left > DOC_OFFSETS[i + 1]));
      if (offset === undefined) continue;
      const key = `doc_${d.id}_j${offset}`;
      if (done[key]) continue;
      keys.push(key);
      docsPlan.push({ id: d.id, label: d.label, dueDate: d.dueDate });
    }
  }
  if (keys.length === 0) return null;
  return { invitationId: inv.id, keys, deadline: deadlinePlan, docs: docsPlan };
}

/** Marque les clés envoyées (et le jour) ; ne mute pas l'entrée. */
export function markRemindersSent(inv: Pick<FamilyInvitation, "reminders">, keys: string[], todayIso: string) {
  const next: Record<string, string> = { ...(inv.reminders || {}) };
  for (const k of keys) next[k] = todayIso;
  next.lastDay = todayIso;
  return next;
}

/**
 * Alerte organisateur à J-7 du départ (ou moins) : au moins une pièce « avant départ » manquante
 * pour un élève de la liste officielle. Retourne la liste des manques, vide si pas d'alerte.
 */
export function docsMissingAlert(opts: {
  departureIso: string | null;
  todayIso: string;
  docs: Pick<TravelsFamilyDocRequest, "id" | "kind" | "label">[];
  invs: FamilyInvitation[];
  officialIneList: string[];
}): { ine: string; studentName: string; missing: string[] }[] {
  if (!opts.departureIso) return [];
  const left = diffDaysIso(opts.todayIso, opts.departureIso);
  if (left > 7 || left < 0) return [];
  const official = new Set(opts.officialIneList);
  const out: { ine: string; studentName: string; missing: string[] }[] = [];
  const seen = new Set<string>();
  for (const inv of opts.invs) {
    if (!official.has(inv.ine) || seen.has(inv.ine) || inv.signature?.decision !== "oui") continue;
    seen.add(inv.ine);
    const missing = opts.docs
      .filter((d) => d.kind === "avant_depart" && !inv.documents?.[d.id] && !inv.exemptions?.[d.id])
      .map((d) => d.label);
    if (missing.length) out.push({ ine: inv.ine, studentName: inv.studentName, missing });
  }
  return out;
}

/** Pièces « avant départ » : dépôt encore possible après signature, jusqu'à la date propre de la pièce. */
export function canUploadAfterSignature(
  doc: Pick<TravelsFamilyDocRequest, "kind" | "dueDate"> | undefined,
  inv: Pick<FamilyInvitation, "signature">,
  todayIso: string,
): boolean {
  if (!doc || doc.kind !== "avant_depart" || inv.signature?.decision !== "oui") return false;
  return !isIsoDate(doc.dueDate) || todayIso <= doc.dueDate;
}

/* ─── Suivi établissement : filtres, ciblage des relances ─── */

export const FOLLOWUP_FILTERS = [
  "pas_ouvert",
  "ouvert",
  "en_cours",
  "bloque_piece",
  "pret",
  "signe_oui",
  "refus",
  "suspens",
  "desaccord",
  "retire",
  "desiste",
  "dispense",
  "papier",
  "sans_email",
] as const;
export type FollowupFilter = (typeof FOLLOWUP_FILTERS)[number];

export type FollowupRow = {
  status: InvitationStatus | "sans_email";
  exempted: boolean;
  paper: boolean;
  invitationId: string | null;
  signed: boolean;
};

export function rowMatchesFilter(row: Pick<FollowupRow, "status" | "exempted" | "paper">, f: FollowupFilter | "all"): boolean {
  if (f === "all") return true;
  if (f === "dispense") return row.exempted;
  if (f === "papier") return row.paper;
  return row.status === f;
}

/** Statuts qu'une relance a un sens de cibler (la famille peut encore agir). */
export const REMINDABLE_STATUSES: InvitationStatus[] = ["pas_ouvert", "ouvert", "en_cours", "bloque_piece", "pret"];

export function isRemindable(row: Pick<FollowupRow, "status" | "invitationId" | "signed">): boolean {
  return Boolean(row.invitationId) && !row.signed && (REMINDABLE_STATUSES as string[]).includes(row.status);
}

/** Cibles d'une relance : une invitation, ou toutes celles d'un statut. */
export function selectReminderTargets<T extends FollowupRow>(
  rows: T[],
  target: { invitationId: string } | { status: FollowupFilter },
): T[] {
  if ("invitationId" in target) return rows.filter((r) => r.invitationId === target.invitationId && isRemindable(r));
  return rows.filter((r) => rowMatchesFilter(r, target.status) && isRemindable(r));
}

/* ─── Export CSV (protégé contre l'injection de formules) ─── */

export function csvCell(v: unknown): string {
  let s = v == null ? "" : String(v);
  if (/^[=+\-@\t\r]/.test(s)) s = `'${s}`;
  return `"${s.replace(/"/g, '""')}"`;
}

export function toCsv(header: string[], rows: unknown[][]): string {
  return "﻿" + [header, ...rows].map((r) => r.map(csvCell).join(";")).join("\r\n");
}
