/**
 * Marques privées ScolIA posées sur les événements Google Agenda.
 *
 * Google applique un PATCH en fusionnant `extendedProperties.private` :
 * une clé absente du corps est CONSERVÉE. Pour l'effacer, il faut l'envoyer à `null`.
 * https://developers.google.com/calendar/api/guides/extended-properties
 */
export const SCOLA_BOOKED_PROP = "scolaBooked";
export const SCOLA_PENDING_PROP = "scolaPending";
export const SCOLA_BOOKING_ID_PROP = "scolaBookingId";

export type GCalPrivatePatch = Record<string, string | null>;

/** Corps `private` d'un PATCH qui efface les clés données (valeur `null`). */
export function privatePatchClearing(
  current: Record<string, string> | undefined,
  keys: readonly string[],
): GCalPrivatePatch {
  const out: GCalPrivatePatch = { ...(current || {}) };
  for (const k of keys) out[k] = null;
  return out;
}

/** Corps `private` d'un PATCH qui efface toutes les marques ScolIA. */
export function privatePatchClearingAllScola(
  current: Record<string, string> | undefined,
): GCalPrivatePatch {
  return privatePatchClearing(current, [
    SCOLA_BOOKED_PROP,
    SCOLA_PENDING_PROP,
    SCOLA_BOOKING_ID_PROP,
  ]);
}

/**
 * Après une annulation / modification par la famille : vrai = remettre le créneau libre,
 * faux = le retirer (trop proche du RDV, la direction doit le voir). 0 h = toujours libre.
 */
export function shouldReleaseFreedSlot(opts: {
  startAt: string | Date;
  releaseMinHours: number;
  now?: Date;
}): boolean {
  const start = new Date(opts.startAt).getTime();
  if (!Number.isFinite(start)) return false;
  const minHours = Number.isFinite(opts.releaseMinHours) ? Math.max(0, opts.releaseMinHours) : 48;
  const now = (opts.now ?? new Date()).getTime();
  return start - now >= minHours * 60 * 60 * 1000;
}

/** Vrai si l'événement porte encore une marque ScolIA (réservé ou en attente). */
export function hasScolaHoldMarker(priv: Record<string, string> | undefined): boolean {
  const p = priv || {};
  return p[SCOLA_BOOKED_PROP] === "true" || p[SCOLA_PENDING_PROP] === "true";
}
