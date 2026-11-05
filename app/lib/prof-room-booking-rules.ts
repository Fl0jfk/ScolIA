/** Règles de réservation de salles partagées par la création et la modification. */

/** Fin de l'horizon de réservation (23:59:59 locale, aujourd'hui + N jours). */
export function bookingHorizonLimit(horizonDays: number, now: Date = new Date()): Date {
  const limit = new Date(now);
  limit.setHours(23, 59, 59, 999);
  limit.setDate(limit.getDate() + (horizonDays || 56));
  return limit;
}

/** Date « AAAA-MM-JJ » au-delà de l'horizon (même calcul que la création : midi local). */
export function isBeyondBookingHorizon(
  dateStr: string,
  horizonDays: number,
  now: Date = new Date(),
): boolean {
  const d = new Date(`${dateStr}T12:00:00`);
  if (Number.isNaN(d.getTime())) return true;
  return d > bookingHorizonLimit(horizonDays, now);
}
