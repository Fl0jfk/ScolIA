/** Utilitaires purs des notifications navigateur (testables sans DOM). */

export type SignalCounts = Record<string, number>;

export type NotifiableSignal = { id: string; label: string; count: number; href: string; detail: string };

/** Signaux dont le compteur a augmenté par rapport à la référence connue. */
export function findIncreasedSignals<T extends NotifiableSignal>(
  previous: SignalCounts | null,
  current: T[],
): T[] {
  if (!previous) return [];
  return current.filter((n) => n.count > 0 && n.count > (previous[n.id] ?? 0));
}

export function toSignalCounts(list: { id: string; count: number }[]): SignalCounts {
  const out: SignalCounts = {};
  for (const n of list) out[n.id] = Math.max(0, n.count);
  return out;
}

export function isInQuietHours(
  minute: number,
  start: number | null,
  end: number | null,
): boolean {
  if (start == null || end == null || start === end) return false;
  if (start < end) return minute >= start && minute < end;
  return minute >= start || minute < end;
}

export function parisMinuteNow(date: Date = new Date()): number {
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

export function minuteToTime(minute: number): string {
  const h = Math.floor(minute / 60);
  const m = minute % 60;
  return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}`;
}

export function timeToMinute(value: string): number | null {
  const m = /^(\d{1,2}):(\d{2})$/.exec(value);
  if (!m) return null;
  const h = Number(m[1]);
  const mi = Number(m[2]);
  if (h > 23 || mi > 59) return null;
  return h * 60 + mi;
}

export function urlBase64ToUint8Array(base64: string): Uint8Array<ArrayBuffer> {
  const padding = "=".repeat((4 - (base64.length % 4)) % 4);
  const raw = atob((base64 + padding).replace(/-/g, "+").replace(/_/g, "/"));
  const out = new Uint8Array(new ArrayBuffer(raw.length));
  for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i);
  return out;
}

/** Garde les URL internes uniquement. */
export function safeInternalUrl(raw: string): string {
  return raw.startsWith("/") && !raw.startsWith("//") ? raw : "/";
}
