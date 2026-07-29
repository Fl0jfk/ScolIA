import { todaySchoolWeekDayIndex } from "@/app/lib/dashboard-week";
import { pickActiveWeekSheet } from "@/app/lib/dashboard-week-sheet-active";
import { loadWeekSheetData } from "@/app/lib/dashboard-week-sheet-storage";
import { WEEK_DAYS, type WeekDayKey, type WeekSheetEvent } from "@/app/lib/dashboard-week-sheet-types";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";

function weekDayFromDateKey(dateKey: string): WeekDayKey | null {
  const d = new Date(`${dateKey}T12:00:00`);
  const wd = new Intl.DateTimeFormat("en-US", {
    timeZone: "Europe/Paris",
    weekday: "short",
  }).format(d);
  const map: Record<string, WeekDayKey> = {
    Mon: "mon",
    Tue: "tue",
    Wed: "wed",
    Thu: "thu",
    Fri: "fri",
  };
  return map[wd] ?? null;
}

function formatEvent(ev: WeekSheetEvent) {
  return {
    title: ev.title,
    day: WEEK_DAYS.find((d) => d.key === ev.day)?.label || ev.day,
    startTime: ev.startTime,
    endTime: ev.endTime || null,
    location: ev.location || null,
  };
}

export async function handleGetWeekSheetToday(
  _ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const stored = await loadWeekSheetData();
  if (!stored) {
    return { ok: false, error: "Aucune feuille de semaine n'est publiée pour le moment." };
  }
  const dateArg = typeof args.date === "string" ? args.date.trim() : "";
  const dateKey = /^\d{4}-\d{2}-\d{2}$/.test(dateArg) ? dateArg : calendarDateKeyParis();
  const ref = new Date(`${dateKey}T12:00:00`);
  const active = pickActiveWeekSheet(stored, ref);
  const dayKey = weekDayFromDateKey(dateKey);
  if (!dayKey) {
    return {
      ok: true,
      data: {
        date: dateKey,
        weekLabel: active.weekLabel || null,
        weekStart: active.weekStart || null,
        events: [],
        note: "Hors jours scolaires (lundi–vendredi).",
      },
      summaryFr: `Pas d'événements scolaires le ${dateKey} (week-end).`,
    };
  }
  const events = (active.events || []).filter((e) => e.day === dayKey).map(formatEvent);
  const dayLabel = WEEK_DAYS.find((d) => d.key === dayKey)?.label || dayKey;
  return {
    ok: true,
    data: {
      date: dateKey,
      day: dayLabel,
      weekLabel: active.weekLabel || null,
      weekStart: active.weekStart || null,
      events,
    },
    summaryFr:
      events.length === 0
        ? `Rien de prévu le ${dayLabel} ${dateKey} sur la feuille de semaine.`
        : `${events.length} événement(s) le ${dayLabel} : ${events.map((e) => e.title).join(", ")}.`,
  };
}

export async function handleGetWeekSheetRange(
  _ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const from = typeof args.from === "string" ? args.from.trim() : "";
  const to = typeof args.to === "string" ? args.to.trim() : "";
  if (!/^\d{4}-\d{2}-\d{2}$/.test(from) || !/^\d{4}-\d{2}-\d{2}$/.test(to)) {
    return { ok: false, error: "Paramètres from/to requis au format YYYY-MM-DD." };
  }
  if (from > to) {
    return { ok: false, error: "La date de début doit être antérieure à la date de fin." };
  }

  const stored = await loadWeekSheetData();
  if (!stored) {
    return { ok: false, error: "Aucune feuille de semaine n'est publiée pour le moment." };
  }

  const days: Array<{ date: string; day: string; events: ReturnType<typeof formatEvent>[] }> = [];
  let cursor = from;
  let guard = 0;
  while (cursor <= to && guard < 31) {
    guard += 1;
    const active = pickActiveWeekSheet(stored, new Date(`${cursor}T12:00:00`));
    const dayKey = weekDayFromDateKey(cursor);
    if (dayKey) {
      const events = (active.events || []).filter((e) => e.day === dayKey).map(formatEvent);
      days.push({
        date: cursor,
        day: WEEK_DAYS.find((d) => d.key === dayKey)?.label || dayKey,
        events,
      });
    }
    const [y, m, d] = cursor.split("-").map(Number);
    const next = new Date(Date.UTC(y, m - 1, d + 1, 12, 0, 0));
    cursor = next.toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
  }

  const total = days.reduce((acc, d) => acc + d.events.length, 0);
  return {
    ok: true,
    data: { from, to, days, totalEvents: total },
    summaryFr: `${total} événement(s) entre le ${from} et le ${to}.`,
  };
}

/** Exposé pour sync knowledge. */
export async function weekSheetTodayTitles(): Promise<{
  weekLabel?: string;
  weekStart?: string;
  eventCount: number;
  todayTitles: string[];
} | null> {
  const stored = await loadWeekSheetData();
  if (!stored) return null;
  const active = pickActiveWeekSheet(stored);
  const idx = todaySchoolWeekDayIndex();
  const keys: WeekDayKey[] = ["mon", "tue", "wed", "thu", "fri"];
  const dayKey = idx >= 0 ? keys[idx] : null;
  const todayEvents = dayKey ? (active.events || []).filter((e) => e.day === dayKey) : [];
  return {
    weekLabel: active.weekLabel,
    weekStart: active.weekStart,
    eventCount: (active.events || []).length,
    todayTitles: todayEvents.map((e) => e.title),
  };
}
