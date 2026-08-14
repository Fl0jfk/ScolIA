import type { WeekDayKey, WeekSheetEvent } from "@/app/lib/dashboard-week-sheet-types";
import { WEEK_DAYS } from "@/app/lib/dashboard-week-sheet-types";

const DAY_ALIASES: Record<string, WeekDayKey> = {
  lundi: "mon",
  lun: "mon",
  monday: "mon",
  mon: "mon",
  mardi: "tue",
  mar: "tue",
  tuesday: "tue",
  tue: "tue",
  mercredi: "wed",
  mer: "wed",
  wednesday: "wed",
  wed: "wed",
  jeudi: "thu",
  jeu: "thu",
  thursday: "thu",
  thu: "thu",
  vendredi: "fri",
  ven: "fri",
  friday: "fri",
  fri: "fri",
};

export function normalizeWeekDay(raw: string): WeekDayKey | null {
  const key = raw.trim().toLowerCase().normalize("NFD").replace(/\p{M}/gu, "");
  return DAY_ALIASES[key] ?? null;
}

/** "8h30", "08:30", "8:30" → minutes depuis minuit */
export function parseTimeToMinutes(raw: string | undefined): number | null {
  if (!raw?.trim()) return null;
  const s = raw.trim().toLowerCase().replace(/\s+/g, "");
  const hMatch = s.match(/^(\d{1,2})h(\d{2})?$/);
  if (hMatch) {
    const h = Number(hMatch[1]);
    const m = Number(hMatch[2] ?? "0");
    if (h >= 0 && h <= 23 && m >= 0 && m <= 59) return h * 60 + m;
  }
  const colon = s.match(/^(\d{1,2}):(\d{2})$/);
  if (colon) {
    const h = Number(colon[1]);
    const m = Number(colon[2]);
    if (h >= 0 && h <= 23 && m >= 0 && m <= 59) return h * 60 + m;
  }
  return null;
}

function formatMinutesAsHour(minutes: number): string {
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return m === 0 ? `${h}h` : `${h}h${String(m).padStart(2, "0")}`;
}

function getEventEndMinutes(event: WeekSheetEvent): number {
  const start = parseTimeToMinutes(event.startTime);
  if (start === null) return 0;
  return parseTimeToMinutes(event.endTime) ?? start + 60;
}

function eventOverlapsHour(event: WeekSheetEvent, hour: number): boolean {
  const start = parseTimeToMinutes(event.startTime);
  if (start === null) return false;
  const end = parseTimeToMinutes(event.endTime) ?? start + 60;
  const hourStart = hour * 60;
  const hourEnd = (hour + 1) * 60;
  return start < hourEnd && end > hourStart;
}

function inferTimeBounds(events: WeekSheetEvent[]): { startMin: number; endMin: number } {
  return getWeekSheetDisplayBounds();
}

/** Grille affichée de 8h à 20h (fixe). */
const WEEK_SHEET_GRID_START_MIN = 8 * 60;
const WEEK_SHEET_GRID_END_MIN = 20 * 60;

function getWeekSheetDisplayBounds(): { startMin: number; endMin: number } {
  return { startMin: WEEK_SHEET_GRID_START_MIN, endMin: WEEK_SHEET_GRID_END_MIN };
}

function getWeekSheetDisplayHours(): number[] {
  const hours: number[] = [];
  for (let h = WEEK_SHEET_GRID_START_MIN / 60; h <= WEEK_SHEET_GRID_END_MIN / 60; h++) {
    hours.push(h);
  }
  return hours;
}

function inferHourRange(_events?: WeekSheetEvent[]): number[] {
  return getWeekSheetDisplayHours();
}

type DayEventLayout = {
  event: WeekSheetEvent;
  top: number;
  height: number;
  column: number;
  columnCount: number;
};

function eventsOverlap(startA: number, endA: number, startB: number, endB: number): boolean {
  return startA < endB && startB < endA;
}

/** Positionne les événements d'un jour façon Google Agenda (colonnes si chevauchement). */
function layoutDayEvents(
  events: WeekSheetEvent[],
  rangeStartMin: number,
  rangeEndMin: number,
  pxPerMinute: number,
  minHeightPx = 22,
): DayEventLayout[] {
  const items = events
    .map((event) => {
      const start = parseTimeToMinutes(event.startTime);
      if (start === null) return null;
      const rawEnd = getEventEndMinutes(event);
      const end = Math.max(rawEnd, start + (event.endTime ? 15 : 60));
      return {
        event,
        start: Math.max(start, rangeStartMin),
        end,
      };
    })
    .filter((item): item is NonNullable<typeof item> => item !== null && item.end > item.start)
    .sort((a, b) => a.start - b.start || b.end - a.end);

  const laneEnds: number[] = [];
  const layouts: DayEventLayout[] = [];

  for (const { event, start, end } of items) {
    let column = 0;
    while (laneEnds[column] !== undefined && laneEnds[column] > start) column++;
    laneEnds[column] = end;

    const timeTop = (start - rangeStartMin) * pxPerMinute;
    const timeHeight = Math.max((end - start) * pxPerMinute, minHeightPx);
    const height = timeHeight;

    layouts.push({
      event,
      top: timeTop,
      height,
      column,
      columnCount: 1,
    });
  }

  for (let i = 0; i < layouts.length; i++) {
    const a = layouts[i];
    const aStart = parseTimeToMinutes(a.event.startTime) ?? 0;
    const aEnd = getEventEndMinutes(a.event);
    let maxColumn = a.column;
    for (let j = 0; j < layouts.length; j++) {
      if (i === j) continue;
      const b = layouts[j];
      const bStart = parseTimeToMinutes(b.event.startTime) ?? 0;
      const bEnd = getEventEndMinutes(b.event);
      if (eventsOverlap(aStart, aEnd, bStart, bEnd)) {
        maxColumn = Math.max(maxColumn, b.column);
      }
    }
    a.columnCount = maxColumn + 1;
  }

  return layouts;
}

/** Hauteur totale de la grille (8h–20h + extension si un créneau dépasse). */
function getWeekSheetGridPixelHeight(
  events: WeekSheetEvent[],
  pxPerMinute: number,
  minHeightPx = 24,
): number {
  const { startMin, endMin } = getWeekSheetDisplayBounds();
  const base = (endMin - startMin) * pxPerMinute;
  let maxBottom = base;

  for (const day of WEEK_DAYS) {
    const dayEvents = events.filter((ev) => ev.day === day.key);
    const layouts = layoutDayEvents(dayEvents, startMin, endMin, pxPerMinute, minHeightPx);
    for (const layout of layouts) {
      maxBottom = Math.max(maxBottom, layout.top + layout.height + 6);
    }
  }

  return maxBottom;
}
