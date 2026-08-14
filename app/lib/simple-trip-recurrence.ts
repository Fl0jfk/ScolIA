import { isJourFerieFranceMetropole } from "@/app/lib/fr-public-holidays";

/** getDay() JS : 0 = dimanche … 6 = samedi */
export const WEEKDAY_JS_OPTIONS: { value: number; label: string }[] = [
  { value: 1, label: "Lundi" },
  { value: 2, label: "Mardi" },
  { value: 3, label: "Mercredi" },
  { value: 4, label: "Jeudi" },
  { value: 5, label: "Vendredi" },
  { value: 6, label: "Samedi" },
  { value: 0, label: "Dimanche" },
];

function parseISODateLocal(ymd: string): Date | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(ymd.trim());
  if (!m) return null;
  const y = Number(m[1]);
  const mo = Number(m[2]);
  const d = Number(m[3]);
  if (!y || mo < 1 || mo > 12 || d < 1 || d > 31) return null;
  const dt = new Date(y, mo - 1, d, 12, 0, 0, 0);
  if (dt.getFullYear() !== y || dt.getMonth() !== mo - 1 || dt.getDate() !== d) return null;
  return dt;
}

function formatYMDLocal(d: Date): string {
  const y = d.getFullYear();
  const mo = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${mo}-${day}`;
}

/**
 * Toutes les dates du jour de la semaine `weekdayJs` entre inclus `rangeStart` et `rangeEnd` (YYYY-MM-DD, fuseau local).
 * Si `skipPublicHolidays`, exclut les jours fériés métropole.
 */
export function enumerateWeeklyDatesInRange(
  weekdayJs: number,
  rangeStart: string,
  rangeEnd: string,
  skipPublicHolidays: boolean
): string[] {
  const start = parseISODateLocal(rangeStart);
  const end = parseISODateLocal(rangeEnd);
  if (!start || !end || start > end) return [];
  const out: string[] = [];
  const cur = new Date(start.getTime());
  while (cur <= end) {
    if (cur.getDay() === weekdayJs) {
      if (!skipPublicHolidays || !isJourFerieFranceMetropole(cur)) {
        out.push(formatYMDLocal(cur));
      }
    }
    cur.setDate(cur.getDate() + 1);
  }
  return out;
}
