import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import type { WeekSheetWeek } from "@/app/lib/dashboard-week-sheet-types";

const FRENCH_MONTHS: Record<string, number> = {
  janvier: 1,
  fevrier: 2,
  mars: 3,
  avril: 4,
  mai: 5,
  juin: 6,
  juillet: 7,
  aout: 8,
  septembre: 9,
  octobre: 10,
  novembre: 11,
  decembre: 12,
};

function normalizeLabel(label: string): string {
  return label
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase();
}

function daysBetweenKeys(a: string, b: string): number {
  const [ay, am, ad] = a.split("-").map(Number);
  const [by, bm, bd] = b.split("-").map(Number);
  const ms =
    Date.UTC(by, bm - 1, bd, 12, 0, 0) - Date.UTC(ay, am - 1, ad, 12, 0, 0);
  return Math.round(ms / 86_400_000);
}

/**
 * Si l'année Mistral/OCR est décalée (ex. 2025 alors qu'on est en 2026),
 * on recentre sur l'année civile la plus proche de la date de référence.
 */
export function coerceWeekStartNearRef(
  weekStart: string | undefined,
  refDate: Date = new Date(),
): string | undefined {
  const raw = weekStart?.trim();
  if (!raw || !/^\d{4}-\d{2}-\d{2}$/.test(raw)) return raw || undefined;

  const today = calendarDateKeyParis(refDate);
  const year = Number(raw.slice(0, 4));
  const md = raw.slice(4); // -MM-DD
  const candidates = [
    raw,
    `${year - 1}${md}`,
    `${year + 1}${md}`,
    `${refDate.getFullYear()}${md}`,
  ];
  let best = raw;
  let bestDist = Math.abs(daysBetweenKeys(today, raw));
  for (const c of candidates) {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(c)) continue;
    const dist = Math.abs(daysBetweenKeys(today, c));
    if (dist < bestDist) {
      best = c;
      bestDist = dist;
    }
  }
  return best;
}

/** Déduit le lundi (YYYY-MM-DD) depuis un libellé « Semaine du 8 au 14 juin ». */
export function inferWeekStartFromLabel(label?: string, refYear?: number): string | undefined {
  if (!label?.trim()) return undefined;
  const text = normalizeLabel(label);
  const match = text.match(/du\s+(\d{1,2})\s+au\s+(\d{1,2})\s+([a-z]+)(?:\s+(\d{4}))?/);
  if (!match) return undefined;

  const day = Number(match[1]);
  const month = FRENCH_MONTHS[match[3]];
  if (!month || day < 1 || day > 31) return undefined;

  const year = match[4] ? Number(match[4]) : (refYear ?? new Date().getFullYear());
  const raw = `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
  return coerceWeekStartNearRef(raw);
}

export function enrichWeekSheetWeek(week: WeekSheetWeek): WeekSheetWeek {
  const weekStart = coerceWeekStartNearRef(
    week.weekStart?.trim() || inferWeekStartFromLabel(week.weekLabel),
  );
  return {
    ...week,
    weekStart,
  };
}

export function splitOcrByWeekSections(ocrText: string): string[] {
  const markers = [...ocrText.matchAll(/Semaine du[^\n]*/gi)];
  if (markers.length < 2) return [ocrText];

  const sections: string[] = [];
  for (let i = 0; i < markers.length; i++) {
    const start = markers[i].index ?? 0;
    const end = markers[i + 1]?.index ?? ocrText.length;
    const chunk = ocrText.slice(start, end).trim();
    if (chunk) sections.push(chunk);
  }
  return sections.length > 0 ? sections : [ocrText];
}
