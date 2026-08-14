import { schoolWeekDaysParis } from "./dashboard-week";
import {
  absencesToCalendarEvents,
  dedupeCalendarEventsForDisplay,
  type CalendarEvent,
} from "./absences-calendar";
import type { AbsenceRecord } from "./absences-types";

type AbsenceDashboardRow = {
  id: string;
  displayName: string;
  reason: string;
  data: {
    scope?: AbsenceRecord["data"]["scope"];
    startAt: string;
    endAt: string;
  };
  privacyReasonRedacted?: boolean;
};

type AbsenceTodayRow = {
  id: string;
  teacherName: string;
  examType: string;
  timeLabel: string;
};

type AbsenceWeekRow = AbsenceTodayRow & { dayKey: string; dayLabel: string };

function formatDayLabel(dayKey: string): string {
  const [y, m, day] = dayKey.split("-").map(Number);
  const d = new Date(y, m - 1, day);
  return d.toLocaleDateString("fr-FR", { weekday: "short", day: "numeric", month: "short" });
}

function eventOnDayKey(event: CalendarEvent, dayKey: string): boolean {
  return event.startAt.slice(0, 10) === dayKey;
}

function calendarEventToTodayRow(event: CalendarEvent): AbsenceTodayRow {
  return {
    id: event.key,
    teacherName: event.displayName,
    examType: event.reason,
    timeLabel: event.displayTime.replace(" - ", " – "),
  };
}

function absencesInWeek(items: AbsenceRecord[]): AbsenceWeekRow[] {
  const events = absencesToCalendarEvents(items);
  const out: AbsenceWeekRow[] = [];

  for (const { key: dayKey } of schoolWeekDaysParis()) {
    const dayEvents = events.filter((event) => eventOnDayKey(event, dayKey));
    for (const row of dedupeCalendarEventsForDisplay(dayEvents).map(calendarEventToTodayRow)) {
      out.push({ ...row, dayKey, dayLabel: formatDayLabel(dayKey) });
    }
  }

  return out.sort((a, b) => a.dayKey.localeCompare(b.dayKey));
}

export function absencesToday(items: AbsenceRecord[]): AbsenceTodayRow[] {
  const todayKey = new Date().toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
  const events = absencesToCalendarEvents(items);
  const dayEvents = events.filter((event) => eventOnDayKey(event, todayKey));
  return dedupeCalendarEventsForDisplay(dayEvents).map(calendarEventToTodayRow);
}
