/** Calendrier parents (.ics) pour sorties scolaires — multi-événements dans un seul fichier. */

import { buildCalendarEventsIcs, type CalendarIcsEvent } from "@/app/lib/calendar-ics";
import type {
  TravelsCalendarPoint,
  TravelsParentCalendar,
  TravelsParentMeeting,
  TravelsTripData,
} from "@/app/lib/travels-types";

export function newCalendarPointId() {
  return `pt_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`;
}

export function defaultParentCalendarFromTrip(data: TravelsTripData): TravelsParentCalendar {
  if (data.parentCalendar?.points?.length) {
    return {
      includeTripSpan: data.parentCalendar.includeTripSpan !== false,
      points: data.parentCalendar.points,
    };
  }

  const startDate = String(data.startDate || data.date || "").slice(0, 10);
  const endDate = String(data.endDate || data.startDate || data.date || "").slice(0, 10);
  const startTime = normalizeTime(data.startTime) || "08:00";
  const endTime = normalizeTime(data.endTime) || "18:00";
  const place =
    typeof data.transportRequest?.pickupPoint === "string"
      ? data.transportRequest.pickupPoint
      : data.parentMeeting?.place || "";

  const points: TravelsCalendarPoint[] = [];

  if (data.parentMeeting?.date && data.parentMeeting?.time) {
    points.push({
      id: newCalendarPointId(),
      kind: "depot",
      label: "Dépôt / départ",
      date: data.parentMeeting.date,
      time: normalizeTime(data.parentMeeting.time) || startTime,
      durationMinutes: data.parentMeeting.durationMinutes || 30,
      place: data.parentMeeting.place || place || undefined,
      note: data.parentMeeting.note,
    });
  } else if (/^\d{4}-\d{2}-\d{2}$/.test(startDate)) {
    points.push({
      id: newCalendarPointId(),
      kind: "depot",
      label: "Dépôt / départ",
      date: startDate,
      time: startTime,
      durationMinutes: 30,
      place: place || undefined,
    });
  }

  if (/^\d{4}-\d{2}-\d{2}$/.test(endDate) && (endDate !== startDate || endTime !== startTime)) {
    points.push({
      id: newCalendarPointId(),
      kind: "recuperation",
      label: "Récupération / retour",
      date: endDate,
      time: endTime,
      durationMinutes: 30,
      place: place || undefined,
    });
  }

  return { includeTripSpan: true, points };
}

function normalizeTime(t: unknown): string | undefined {
  const s = String(t || "").trim();
  if (/^\d{2}:\d{2}$/.test(s)) return s;
  if (/^\d{2}:\d{2}:\d{2}$/.test(s)) return s.slice(0, 5);
  return undefined;
}

function isValidCalendarPoint(p: TravelsCalendarPoint | undefined): p is TravelsCalendarPoint {
  return Boolean(
    p &&
      /^\d{4}-\d{2}-\d{2}$/.test(p.date) &&
      /^\d{2}:\d{2}$/.test(normalizeTime(p.time) || "") &&
      (p.kind === "depot" || p.kind === "recuperation" || p.kind === "autre"),
  );
}

const KIND_LABEL: Record<TravelsCalendarPoint["kind"], string> = {
  depot: "Dépôt / départ",
  recuperation: "Récupération / retour",
  autre: "Point d’attention",
};

/** Construit un .ics unique : séjour (optionnel) + points dépôt / récupération / autres. */
export function buildTravelsParentsTripIcs(input: {
  tripId: string;
  tripTitle: string;
  destination?: string;
  data: TravelsTripData;
  calendar?: TravelsParentCalendar | null;
}): string {
  const cal = input.calendar || defaultParentCalendarFromTrip(input.data);
  const events: CalendarIcsEvent[] = [];
  const title = input.tripTitle;

  if (cal.includeTripSpan !== false) {
    const startDate = String(input.data.startDate || input.data.date || "").slice(0, 10);
    const endDate = String(input.data.endDate || startDate).slice(0, 10);
    const startTime = normalizeTime(input.data.startTime) || "08:00";
    const endTime = normalizeTime(input.data.endTime) || "18:00";
    if (/^\d{4}-\d{2}-\d{2}$/.test(startDate) && /^\d{4}-\d{2}-\d{2}$/.test(endDate)) {
      events.push({
        title: `Séjour — ${title}`,
        description: [
          `Séjour scolaire « ${title} ».`,
          input.destination ? `Destination : ${input.destination}` : "",
          "Événement couvrant toute la durée du voyage.",
        ]
          .filter(Boolean)
          .join("\n"),
        location: input.destination,
        startDate,
        startTime,
        endDate,
        endTime,
        uid: `travels-sejour-${input.tripId}@scola`,
      });
    }
  }

  for (const p of cal.points || []) {
    if (!isValidCalendarPoint(p)) continue;
    const time = normalizeTime(p.time)!;
    const kindLabel = p.label?.trim() || KIND_LABEL[p.kind];
    events.push({
      title: `${kindLabel} — ${title}`,
      description: [
        `${kindLabel} pour la sortie « ${title} ».`,
        p.note?.trim() || "",
      ]
        .filter(Boolean)
        .join("\n"),
      location: p.place?.trim() || undefined,
      startDate: p.date,
      startTime: time,
      durationMinutes: p.durationMinutes || 30,
      uid: `travels-${p.kind}-${p.id}-${input.tripId}@scola`,
    });
  }

  return buildCalendarEventsIcs({
    events,
    prodId: "-//Scola//Sorties scolaires//FR",
  });
}

/** Compat : ancien champ parentMeeting → calendrier. */
function parentMeetingToCalendar(m: TravelsParentMeeting | undefined): TravelsParentCalendar | null {
  if (!m?.date || !m?.time) return null;
  return {
    includeTripSpan: true,
    points: [
      {
        id: newCalendarPointId(),
        kind: "depot",
        label: "Dépôt / départ",
        date: m.date,
        time: normalizeTime(m.time) || m.time,
        durationMinutes: m.durationMinutes || 30,
        place: m.place,
        note: m.note,
      },
    ],
  };
}

/** Normalise un patch UI / API vers un calendrier propre. */
export function sanitizeParentCalendar(
  raw: TravelsParentCalendar | null | undefined,
  fallbackFrom?: TravelsTripData,
): TravelsParentCalendar {
  const base = raw?.points?.length
    ? raw
    : fallbackFrom
      ? defaultParentCalendarFromTrip(fallbackFrom)
      : { includeTripSpan: true, points: [] as TravelsCalendarPoint[] };

  const points = (base.points || [])
    .filter((p) => p && typeof p === "object")
    .map((p) => ({
      id: String(p.id || newCalendarPointId()),
      kind:
        p.kind === "recuperation" || p.kind === "autre" || p.kind === "depot"
          ? p.kind
          : ("autre" as const),
      label: p.label ? String(p.label).trim() : undefined,
      date: String(p.date || "").slice(0, 10),
      time: normalizeTime(p.time) || String(p.time || "").slice(0, 5),
      durationMinutes:
        typeof p.durationMinutes === "number" && p.durationMinutes > 0
          ? Math.min(240, Math.round(p.durationMinutes))
          : 30,
      place: p.place ? String(p.place).trim() : undefined,
      note: p.note ? String(p.note).trim() : undefined,
    }));

  return {
    includeTripSpan: base.includeTripSpan !== false,
    points,
  };
}

/** Dépôt + récupération avec date/heure valides (requis avant envoi parents). */
export function calendarHasDepotAndRecuperation(cal: TravelsParentCalendar | null | undefined): boolean {
  const points = cal?.points || [];
  const hasDepot = points.some((p) => p.kind === "depot" && isValidCalendarPoint(p));
  const hasRecup = points.some((p) => p.kind === "recuperation" && isValidCalendarPoint(p));
  return hasDepot && hasRecup;
}

function formatFrDate(iso: string): string {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(iso)) return iso;
  const [y, m, d] = iso.split("-");
  return `${d}/${m}/${y}`;
}

function isTripSameDay(data: TravelsTripData): boolean {
  const start = String(data.startDate || data.date || "").slice(0, 10);
  const end = String(data.endDate || start).slice(0, 10);
  return Boolean(start && start === end);
}

/** Texte mail parents : journée vs séjour + heures dépôt / reprise. */
export function buildParentsCalendarMailCopy(input: {
  tripTitle: string;
  data: TravelsTripData;
  calendar: TravelsParentCalendar;
}): { intro: string; pointsLines: string[]; pointsBlock: string } {
  const sameDay = isTripSameDay(input.data);
  const start = String(input.data.startDate || input.data.date || "").slice(0, 10);
  const end = String(input.data.endDate || start).slice(0, 10);
  const datesLabel = sameDay
    ? `le ${formatFrDate(start)}`
    : `du ${formatFrDate(start)} au ${formatFrDate(end)}`;

  const intro = sameDay
    ? `Votre enfant part en journée scolaire « ${input.tripTitle} » (${datesLabel}).`
    : `Votre enfant part en séjour scolaire « ${input.tripTitle} » (${datesLabel}).`;

  const pointsLines = (input.calendar.points || [])
    .filter(isValidCalendarPoint)
    .map((pt) => {
      const label =
        pt.label?.trim() ||
        (pt.kind === "depot"
          ? "Heure de dépôt / départ"
          : pt.kind === "recuperation"
            ? "Heure de reprise / récupération"
            : "Point d’attention");
      const when = `${formatFrDate(pt.date)} à ${normalizeTime(pt.time) || pt.time}`;
      const place = pt.place?.trim() ? ` — ${pt.place.trim()}` : "";
      return `• ${label} : ${when}${place}`;
    });

  return {
    intro,
    pointsLines,
    pointsBlock: pointsLines.length ? `Horaires à retenir :\n${pointsLines.join("\n")}` : "",
  };
}
