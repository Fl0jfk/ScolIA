/** Heures calmes en heure d'Europe/Paris (minutes depuis minuit). Pur, sans dépendance serveur. */

export function parisMinuteOfDay(date: Date = new Date()): number {
  const parts = new Intl.DateTimeFormat("fr-FR", {
    timeZone: "Europe/Paris",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  }).formatToParts(date);
  const h = Number(parts.find((p) => p.type === "hour")?.value ?? "0");
  const m = Number(parts.find((p) => p.type === "minute")?.value ?? "0");
  return (h % 24) * 60 + m;
}

/** Vrai si `minute` est dans [start, end[ (plage pouvant passer minuit). */
export function isInQuietHours(
  minute: number,
  start: number | null | undefined,
  end: number | null | undefined,
): boolean {
  if (start == null || end == null || start === end) return false;
  if (start < end) return minute >= start && minute < end;
  return minute >= start || minute < end;
}
