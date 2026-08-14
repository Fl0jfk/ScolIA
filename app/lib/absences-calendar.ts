import { getPublicAbsenceReason } from "@/app/lib/absences-privacy";
import { normalizeAbsencePersonName } from "@/app/lib/absences-shared-utils";
import { resolveAbsenceScope, type AbsenceRecord } from "@/app/lib/absences-types";

export type CalendarEvent = {
  key: string;
  id: string;
  displayName: string;
  scope: "professeur" | "ogec";
  reason: string;
  startAt: string;
  endAt: string;
  hasDocument: boolean;
  documentCount: number;
  displayTime: string;
  isOgec: boolean;
};

function pad2(n: number) {
  return String(n).padStart(2, "0");
}

function formatTimeFR(date: Date) {
  return `${pad2(date.getHours())}h${pad2(date.getMinutes())}`;
}

function sameDay(date: Date, y: number, m: number, d: number) {
  return date.getFullYear() === y && date.getMonth() === m && date.getDate() === d;
}

export function sortCalendarEvents(events: CalendarEvent[]) {
  return [...events].sort((a, b) => {
    if (a.isOgec !== b.isOgec) return a.isOgec ? -1 : 1;
    return a.displayName.localeCompare(b.displayName, "fr", { sensitivity: "base" });
  });
}

/** Affichage calendrier : une seule tuile par personne et par jour (sans réécrire la base). */
export function dedupeCalendarEventsForDisplay(events: CalendarEvent[]): CalendarEvent[] {
  const map = new Map<string, CalendarEvent>();

  for (const event of events) {
    const dayKey = event.startAt.slice(0, 10);
    const personKey = normalizeAbsencePersonName(event.displayName);
    const key = `${personKey}|${dayKey}|${event.scope}`;
    const existing = map.get(key);
    if (!existing) {
      map.set(key, event);
      continue;
    }

    map.set(key, {
      ...existing,
      hasDocument: existing.hasDocument || event.hasDocument,
      documentCount: existing.documentCount + event.documentCount,
      displayTime:
        existing.displayTime === event.displayTime
          ? existing.displayTime
          : `${existing.displayTime} · ${event.displayTime}`,
      reason: existing.reason === event.reason ? existing.reason : existing.reason,
    });
  }

  return sortCalendarEvents([...map.values()]);
}

export function absencesToCalendarEvents(
  items: AbsenceRecord[],
  opts?: { includeDocumentsFor?: (item: AbsenceRecord) => boolean },
): CalendarEvent[] {
  const out: CalendarEvent[] = [];
  const includeDocumentsFor = opts?.includeDocumentsFor ?? (() => true);

  for (const item of items) {
    const start = new Date(item.data.startAt);
    const end = new Date(item.data.endAt);
    if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime()) || +end <= +start) continue;

    const showDocuments = includeDocumentsFor(item);
    const documentCount = showDocuments
      ? (item.data.documentKeys?.length ?? 0) + (item.justification?.fileUrl ? 1 : 0)
      : 0;
    const scope = resolveAbsenceScope(item);
    const isOgec = scope === "ogec";

    const day = new Date(start.getFullYear(), start.getMonth(), start.getDate());
    const last = new Date(end.getFullYear(), end.getMonth(), end.getDate());
    for (let cursor = new Date(day); cursor <= last; cursor.setDate(cursor.getDate() + 1)) {
      const y = cursor.getFullYear();
      const m = cursor.getMonth();
      const d = cursor.getDate();
      const isFirstDay = sameDay(cursor, start.getFullYear(), start.getMonth(), start.getDate());
      const isLastDay = sameDay(cursor, end.getFullYear(), end.getMonth(), end.getDate());
      const startAt = isFirstDay ? start.toISOString() : new Date(y, m, d, 0, 0, 0, 0).toISOString();
      const endAt = isLastDay ? end.toISOString() : new Date(y, m, d, 23, 59, 0, 0).toISOString();
      const displayTime =
        isFirstDay && isLastDay
          ? `${formatTimeFR(start)} - ${formatTimeFR(end)}`
          : isFirstDay
            ? `à partir de ${formatTimeFR(start)}`
            : isLastDay
              ? `jusqu'à ${formatTimeFR(end)}`
              : "journée";

      out.push({
        key: `${item.id}_${y}-${pad2(m + 1)}-${pad2(d)}`,
        id: item.id,
        displayName: item.displayName,
        scope,
        reason: getPublicAbsenceReason(item),
        startAt,
        endAt,
        hasDocument: documentCount > 0,
        documentCount,
        displayTime,
        isOgec,
      });
    }
  }

  return out.sort((a, b) => +new Date(a.startAt) - +new Date(b.startAt));
}

function teacherColorKey(name: string) {
  return String(name || "")
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

function appearanceForTeacherIndex(index: number) {
  const hue = Math.round((index * 137.508) % 360);
  return {
    cardStyle: {
      backgroundColor: `hsl(${hue} 62% 91%)`,
      color: `hsl(${hue} 42% 24%)`,
      borderColor: `hsl(${hue} 38% 78%)`,
    },
    print: {
      bg: `hsl(${hue} 62% 91%)`,
      text: `hsl(${hue} 42% 24%)`,
      border: `hsl(${hue} 38% 78%)`,
    },
  };
}

const OGEC_APPEARANCE = {
  cardStyle: {
    backgroundColor: "#fee2e2",
    color: "#991b1b",
    borderColor: "#f87171",
  },
  print: {
    bg: "#fee2e2",
    text: "#991b1b",
    border: "#f87171",
  },
};

export function buildTeacherColorIndexMap(names: string[]) {
  const sortedKeys = [...new Set(names.map((n) => teacherColorKey(n)).filter(Boolean))].sort((a, b) =>
    a.localeCompare(b, "fr", { sensitivity: "base" }),
  );
  const map = new Map<string, number>();
  sortedKeys.forEach((key, index) => map.set(key, index));
  return map;
}

export function appearanceForEvent(
  event: CalendarEvent,
  teacherColorIndexMap: Map<string, number>,
) {
  if (event.isOgec) return OGEC_APPEARANCE;
  const key = teacherColorKey(event.displayName);
  const index = key ? (teacherColorIndexMap.get(key) ?? 0) : 0;
  return appearanceForTeacherIndex(index);
}
