import { getPublicAbsenceReason } from "@/app/lib/absences-privacy";
import { normalizeAbsencePersonName } from "@/app/lib/absences-shared-utils";
import { resolveAbsenceScope, type AbsenceRecord } from "@/app/lib/absences-types";
import {
  formatParisTimeLabel,
  formatWallTimeLabel,
  listParisDateKeysFromTo,
  parisDateKey,
  parisWallTimeToDate,
} from "@/app/lib/paris-time";

export type CalendarEvent = {
  key: string;
  id: string;
  displayName: string;
  scope: "professeur" | "ogec";
  reason: string;
  /** Instant ISO (UTC). */
  startAt: string;
  endAt: string;
  /** Jour calendaire Europe/Paris `YYYY-MM-DD` (placement dans la grille). */
  dayKey: string;
  hasDocument: boolean;
  documentCount: number;
  displayTime: string;
  isOgec: boolean;
};

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
    const dayKey = event.dayKey || parisDateKey(event.startAt);
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

    const wallStart = String(item.data.startTime || "").trim().slice(0, 5);
    const wallEnd = String(item.data.endTime || "").trim().slice(0, 5);
    const hasWallTimes = /^\d{2}:\d{2}$/.test(wallStart) && /^\d{2}:\d{2}$/.test(wallEnd);

    const dayKeys = listParisDateKeysFromTo(item.data.startAt, item.data.endAt);
    for (let i = 0; i < dayKeys.length; i += 1) {
      const dayKey = dayKeys[i];
      const isFirstDay = i === 0;
      const isLastDay = i === dayKeys.length - 1;

      const dayStart =
        isFirstDay
          ? start
          : parisWallTimeToDate(dayKey, 0, 0, 0, 0) ?? start;
      const dayEnd =
        isLastDay
          ? end
          : parisWallTimeToDate(dayKey, 23, 59, 0, 0) ?? end;

      const displayTime =
        isFirstDay && isLastDay && hasWallTimes
          ? `${formatWallTimeLabel(wallStart)} - ${formatWallTimeLabel(wallEnd)}`
          : isFirstDay && isLastDay
            ? `${formatParisTimeLabel(start)} - ${formatParisTimeLabel(end)}`
            : isFirstDay
              ? hasWallTimes
                ? `à partir de ${formatWallTimeLabel(wallStart)}`
                : `à partir de ${formatParisTimeLabel(start)}`
              : isLastDay
                ? hasWallTimes
                  ? `jusqu'à ${formatWallTimeLabel(wallEnd)}`
                  : `jusqu'à ${formatParisTimeLabel(end)}`
                : "journée";

      out.push({
        key: `${item.id}_${dayKey}`,
        id: item.id,
        displayName: item.displayName,
        scope,
        reason: getPublicAbsenceReason(item),
        startAt: dayStart.toISOString(),
        endAt: dayEnd.toISOString(),
        dayKey,
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
