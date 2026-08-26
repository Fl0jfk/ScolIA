/** Fenêtre pendant laquelle un envoi est considéré « récent » (alignée sur la validité du lien). */
export const INVITATION_RECENT_MS = 12 * 60 * 60 * 1000;

export function invitationRecentlySent(
  iso: string | null | undefined,
  nowMs: number = Date.now(),
): boolean {
  if (!iso) return false;
  const t = new Date(iso).getTime();
  if (Number.isNaN(t)) return false;
  return nowMs - t < INVITATION_RECENT_MS;
}
