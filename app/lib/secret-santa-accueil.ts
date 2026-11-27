/** Ce qui remonte sur la page d'accueil d'une personne invitée — règles pures, sans aucun nom tiré. */

import { isRevealDue, type SantaResponse } from "@/app/lib/secret-santa";

/** Au-delà, un Secret Santa sans date de rencontre (ou jamais tiré) quitte l'accueil. */
const KEEP_AFTER_REVEAL_MS = 30 * 24 * 3600 * 1000;
/** Après la rencontre, il reste affiché le temps de la journée. */
const KEEP_AFTER_EVENT_MS = 24 * 3600 * 1000;

export type SantaHomeStatus =
  /** Invitation à laquelle répondre. */
  | "answer"
  /** A dit oui, le tirage n'a pas encore eu lieu. */
  | "waiting"
  /** L'heure est passée : à aller découvrir. */
  | "reveal"
  /** Son nom tiré a changé après un désistement. */
  | "changed"
  /** Cadeau déjà ouvert. */
  | "opened";

export type SantaHomeSource = {
  revealAt: Date;
  drawnAt: Date | null;
  eventAt: Date | null;
  response: SantaResponse;
  openedAt: Date | null;
  changedAt: Date | null;
};

/** `null` : rien à montrer (refus, retrait, réponses closes sans « oui », ou Secret Santa terminé). */
export function santaHomeStatus(s: SantaHomeSource, now: Date): SantaHomeStatus | null {
  const end =
    s.drawnAt && s.eventAt ? s.eventAt.getTime() + KEEP_AFTER_EVENT_MS : s.revealAt.getTime() + KEEP_AFTER_REVEAL_MS;
  if (now.getTime() > end) return null;
  const closed = s.drawnAt !== null || isRevealDue(s.revealAt, now);
  if (s.response === "pending") return closed ? null : "answer";
  if (s.response !== "accepted") return null;
  if (!closed) return "waiting";
  if (s.changedAt) return "changed";
  return s.openedAt ? "opened" : "reveal";
}

export type SantaHomeCard = {
  id: string;
  title: string;
  organizerName: string;
  status: SantaHomeStatus;
  revealAt: string;
  eventAt: string | null;
  eventPlace: string;
};
