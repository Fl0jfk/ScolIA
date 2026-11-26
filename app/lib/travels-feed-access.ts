/**
 * « Fil du voyage » — décision d'accès d'une FAMILLE (pure, testable) : jeton + accès + voyage + réglages.
 * Toute cause de refus produit le MÊME résultat (aucun oracle : jeton invalide, autre voyage, élève désisté,
 * fil non ouvert, voyage annulé, purge échue).
 */
import type { TravelsTrip } from "@/app/lib/travels-types";
import { todayIsoParis } from "@/app/lib/travels-v2";
import { parseInviteToken, verifyInviteSecret } from "@/app/lib/travels-v2-registration";
import {
  familyKeyOf,
  feedParticipants,
  feedPhase,
  feedReturnIso,
  stillParticipantInes,
  type FeedPhase,
} from "@/app/lib/travels-feed-logic";

export type FeedAccessLike = { id: string; tripId: string; tokenHashes: string[]; ines: string[]; familyKey: string };

export type FamilyAccessDecision =
  | { ok: true; ines: string[]; phase: Exclude<FeedPhase, "unavailable" | "purge_due">; familyKey: string }
  | { ok: false };

export function evaluateFamilyAccess(opts: {
  token: string | null | undefined;
  access: FeedAccessLike | null;
  trip: Pick<TravelsTrip, "id" | "status" | "data"> | null;
  feedOpen: boolean;
  now?: Date;
}): FamilyAccessDecision {
  const parsed = parseInviteToken(opts.token);
  if (!parsed || !opts.access || !opts.trip || !opts.feedOpen) return { ok: false };
  // L'accès doit appartenir EXACTEMENT à ce voyage : un jeton d'un autre voyage ne donne rien.
  if (opts.access.id !== parsed.invitationId || opts.access.tripId !== parsed.tripId || opts.trip.id !== parsed.tripId) return { ok: false };
  if (!verifyInviteSecret(opts.access.tokenHashes, parsed.secret)) return { ok: false };
  const ines = stillParticipantInes(opts.access.ines, feedParticipants(opts.trip));
  if (ines.length === 0) return { ok: false };
  const phase = feedPhase({ status: opts.trip.status, returnIso: feedReturnIso(opts.trip), todayIso: todayIsoParis(opts.now ?? new Date()) });
  if (phase === "unavailable" || phase === "purge_due") return { ok: false };
  return { ok: true, ines, phase, familyKey: opts.access.familyKey || familyKeyOf(ines) };
}
