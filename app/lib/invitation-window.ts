/** Durée de validité du lien d’invitation / reset (alignée Better-Auth + UI). */
export const INVITATION_VALIDITY_HOURS = 24;

/** Fenêtre pendant laquelle un envoi est considéré « récent » (alignée sur la validité du lien). */
export const INVITATION_RECENT_MS = INVITATION_VALIDITY_HOURS * 60 * 60 * 1000;

/** Secondes — paramètre Better-Auth `resetPasswordTokenExpiresIn`. */
export const INVITATION_TOKEN_EXPIRES_IN_SEC = INVITATION_VALIDITY_HOURS * 60 * 60;

export function invitationRecentlySent(
  iso: string | null | undefined,
  nowMs: number = Date.now(),
): boolean {
  if (!iso) return false;
  const t = new Date(iso).getTime();
  if (Number.isNaN(t)) return false;
  return nowMs - t < INVITATION_RECENT_MS;
}
