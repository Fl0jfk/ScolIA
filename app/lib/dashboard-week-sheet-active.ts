import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import type { WeekSheetData, WeekSheetEvent, WeekSheetWeek } from "@/app/lib/dashboard-week-sheet-types";
import { enrichWeekSheetWeek } from "@/app/lib/dashboard-week-sheet-week";

function stripEventNotes(events: WeekSheetEvent[]): WeekSheetEvent[] {
  return events.map(({ notes: _n, ...ev }) => ev);
}

function stripEventNotesFromData(data: WeekSheetData): WeekSheetData {
  return {
    ...data,
    events: stripEventNotes(data.events),
    weeks: data.weeks?.map((w) => ({ ...w, events: stripEventNotes(w.events) })),
  };
}

function applyWeek(data: WeekSheetData, week: WeekSheetWeek): WeekSheetData {
  return {
    ...data,
    weekLabel: week.weekLabel ?? data.weekLabel,
    weekStart: week.weekStart ?? data.weekStart,
    events: stripEventNotes(week.events),
    weeks: data.weeks?.map((w) => ({ ...w, events: stripEventNotes(w.events) })),
  };
}

function addCalendarDays(key: string, days: number): string {
  const [y, m, d] = key.split("-").map(Number);
  const next = new Date(Date.UTC(y, m - 1, d + days, 12, 0, 0));
  return next.toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
}

function weekEndFriday(weekStart: string): string {
  return addCalendarDays(weekStart, 4);
}

function weekContainsDate(week: WeekSheetWeek, dateKey: string): boolean {
  if (!week.weekStart) return false;
  return dateKey >= week.weekStart && dateKey <= weekEndFriday(week.weekStart);
}

function listWeekSheetWeeks(data: WeekSheetData): WeekSheetWeek[] {  const raw = data.weeks?.length
    ? data.weeks
    : [{ weekLabel: data.weekLabel, weekStart: data.weekStart, events: data.events }];
  return raw.map(enrichWeekSheetWeek);
}

/** Semaine PDF qui contient strictement la date (pas de fallback). */
export function pickExactCurrentWeekSheet(
  data: WeekSheetData,
  refDate: Date = new Date(),
): WeekSheetData | null {
  const today = calendarDateKeyParis(refDate);
  const weeks = listWeekSheetWeeks(data).filter((w) => w.events.length > 0);
  const exact = weeks.find((w) => weekContainsDate(w, today));
  return exact ? applyWeek(data, exact) : null;
}

/** Choisit la semaine du PDF qui correspond au jour courant (fuseau Paris). */
export function pickActiveWeekSheet(
  data: WeekSheetData,
  refDate: Date = new Date(),
): WeekSheetData {
  const exact = pickExactCurrentWeekSheet(data, refDate);
  if (exact) return exact;

  const today = calendarDateKeyParis(refDate);
  const weeks = listWeekSheetWeeks(data).filter((w) => w.events.length > 0);
  if (weeks.length === 0) return stripEventNotesFromData(data);

  const dated = weeks
    .filter((w): w is WeekSheetWeek & { weekStart: string } => Boolean(w.weekStart))
    .sort((a, b) => a.weekStart.localeCompare(b.weekStart));

  if (dated.length === 0) return applyWeek(data, weeks[0]);

  const future = dated.filter((w) => w.weekStart > today);
  if (future.length > 0) return applyWeek(data, future[0]);

  const currentOrLatest = [...dated]
    .filter((w) => w.weekStart <= today)
    .sort((a, b) => b.weekStart.localeCompare(a.weekStart));

  if (currentOrLatest.length > 0) {
    const ongoing = currentOrLatest.find((w) => weekEndFriday(w.weekStart) >= today);
    return applyWeek(data, ongoing ?? currentOrLatest[0]);
  }

  return applyWeek(data, dated[0]);
}
