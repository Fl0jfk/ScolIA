/** Fuseau institutionnel — source de vérité pour horaires muraux (absences, planning…). */

export const PARIS_TZ = "Europe/Paris";

export type ParisDateTimeParts = {
  year: number;
  month: number;
  day: number;
  hour: number;
  minute: number;
  second: number;
};

function pad2(n: number) {
  return String(n).padStart(2, "0");
}

function toDate(input: Date | string | number): Date {
  return input instanceof Date ? input : new Date(input);
}

/** Parties calendaires / horaires d’un instant, vues depuis Europe/Paris. */
export function getParisParts(input: Date | string | number): ParisDateTimeParts {
  const d = toDate(input);
  const fmt = new Intl.DateTimeFormat("en-GB", {
    timeZone: PARIS_TZ,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hourCycle: "h23",
  });
  const bag: Record<string, string> = {};
  for (const part of fmt.formatToParts(d)) {
    if (part.type !== "literal") bag[part.type] = part.value;
  }
  return {
    year: Number(bag.year),
    month: Number(bag.month),
    day: Number(bag.day),
    hour: Number(bag.hour),
    minute: Number(bag.minute),
    second: Number(bag.second),
  };
}

/** Clé jour `YYYY-MM-DD` en Europe/Paris. */
export function parisDateKey(input: Date | string | number): string {
  const p = getParisParts(input);
  return `${p.year}-${pad2(p.month)}-${pad2(p.day)}`;
}

/**
 * Heure murale Europe/Paris → Date (instant UTC).
 * Indépendant du fuseau du runtime (serveur UTC ou navigateur).
 */
export function parisWallTimeToDate(
  dateKey: string,
  hour: number,
  minute: number,
  second = 0,
  ms = 0,
): Date | null {
  const key = String(dateKey || "").trim();
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(key);
  if (!m) return null;
  const y = Number(m[1]);
  const mo = Number(m[2]);
  const d = Number(m[3]);
  if (![y, mo, d, hour, minute, second, ms].every((n) => Number.isFinite(n))) return null;
  if (mo < 1 || mo > 12 || d < 1 || d > 31) return null;
  if (hour < 0 || hour > 23 || minute < 0 || minute > 59 || second < 0 || second > 59) return null;

  const wantedAsUtc = Date.UTC(y, mo - 1, d, hour, minute, second, ms);
  let guess = wantedAsUtc;
  for (let i = 0; i < 4; i += 1) {
    const p = getParisParts(guess);
    const asUtcLike = Date.UTC(p.year, p.month - 1, p.day, p.hour, p.minute, p.second, ms);
    const diff = wantedAsUtc - asUtcLike;
    if (diff === 0) break;
    guess += diff;
  }

  const verified = getParisParts(guess);
  if (
    verified.year !== y ||
    verified.month !== mo ||
    verified.day !== d ||
    verified.hour !== hour ||
    verified.minute !== minute
  ) {
    // Transition DST (heure inexistante) : dernier essai via offsets FR.
    for (const offset of ["+02:00", "+01:00"] as const) {
      const iso = `${key}T${pad2(hour)}:${pad2(minute)}:${pad2(second)}.${String(ms).padStart(3, "0")}${offset}`;
      const t = +new Date(iso);
      if (Number.isNaN(t)) continue;
      const p = getParisParts(t);
      if (p.year === y && p.month === mo && p.day === d && p.hour === hour && p.minute === minute) {
        return new Date(t);
      }
    }
    return null;
  }
  return new Date(guess);
}

/** Parse `YYYY-MM-DD` + `HH:mm` / `HH:mm:ss` comme heure murale Paris. */
export function parseParisDateTime(dateStr: string, timeStr: string): Date | null {
  const ds = String(dateStr || "").trim();
  const ts = String(timeStr || "").trim();
  if (!ds || !ts) return null;
  const timeNorm = ts.length === 5 ? `${ts}:00` : ts;
  const tp = timeNorm.split(":");
  const h = Number(tp[0]);
  const mi = Number(tp[1] ?? 0);
  const sec = Number(tp[2] ?? 0);
  if (![h, mi, sec].every((n) => Number.isFinite(n))) return null;
  return parisWallTimeToDate(ds, h, mi, sec, 0);
}

/** `HH:mm` depuis un ISO, fuseau Paris. */
export function formatParisHm(input: Date | string | number): string {
  const p = getParisParts(input);
  return `${pad2(p.hour)}:${pad2(p.minute)}`;
}

/** Affichage FR type `10h` / `10h30`. */
export function formatParisTimeLabel(input: Date | string | number): string {
  const p = getParisParts(input);
  return p.minute === 0 ? `${pad2(p.hour)}h` : `${pad2(p.hour)}h${pad2(p.minute)}`;
}

/** Affichage FR depuis une heure murale `HH:mm`. */
export function formatWallTimeLabel(hhmm: string): string {
  const [hRaw, mRaw] = String(hhmm || "").split(":");
  const h = (hRaw ?? "00").padStart(2, "0");
  const m = (mRaw ?? "00").padStart(2, "0");
  return m === "00" ? `${h}h` : `${h}h${m}`;
}

/** Liste des clés jour Paris de startIso à endIso (inclus). */
export function listParisDateKeysFromTo(startIso: string, endIso: string): string[] {
  const keys: string[] = [];
  let k = parisDateKey(startIso);
  const last = parisDateKey(endIso);
  const guard = 400;
  for (let i = 0; i < guard; i += 1) {
    keys.push(k);
    if (k === last) break;
    const [y, m, d] = k.split("-").map(Number);
    k = new Date(Date.UTC(y, m - 1, d + 1)).toISOString().slice(0, 10);
  }
  return keys;
}
