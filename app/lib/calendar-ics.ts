/** Générateur .ics générique (événements établissement, sorties, etc.). */

export type CalendarIcsEvent = {
  title: string;
  description?: string;
  location?: string;
  /** ISO datetime (avec Z ou offset) — utilisé si fourni. */
  startAt?: string;
  endAt?: string;
  /**
   * Alternative : date + heure murale Europe/Paris (YYYY-MM-DD + HH:mm).
   * Preferé pour les RDV parents (évite les ambiguïtés serveur UTC).
   */
  startDate?: string;
  startTime?: string;
  /** Fin explicite (séjour multi-jours). */
  endDate?: string;
  endTime?: string;
  /** Durée en minutes si pas d’endDate/endTime (défaut 30). */
  durationMinutes?: number;
  uid?: string;
  prodId?: string;
};

/** Alias historique portes ouvertes. */
export function buildPortesOuvertesIcs(params: {
  title: string;
  description?: string;
  location?: string;
  startAt: string;
  endAt: string;
  uid?: string;
}): string {
  return buildCalendarEventIcs({
    ...params,
    prodId: "-//Scola//Portes ouvertes//FR",
  });
}

export function buildCalendarEventIcs(params: CalendarIcsEvent): string {
  return buildCalendarEventsIcs({
    events: [params],
    prodId: params.prodId,
  });
}

/** Un fichier .ics avec plusieurs VEVENT (dépôt + séjour + récupération…). */
export function buildCalendarEventsIcs(params: {
  events: CalendarIcsEvent[];
  prodId?: string;
}): string {
  const prodId = params.prodId || "-//Scola//Calendrier//FR";
  const stamp = formatIcsNowUtc();
  const blocks: string[] = [
    "BEGIN:VCALENDAR",
    "VERSION:2.0",
    `PRODID:${prodId}`,
    "CALSCALE:GREGORIAN",
    "METHOD:PUBLISH",
  ];

  for (const ev of params.events) {
    const uid = ev.uid || `scola-${Date.now()}-${Math.random().toString(36).slice(2, 7)}@scola`;
    const { dtStart, dtEnd, useParisTz } = resolveIcsBounds(ev);
    const desc = (ev.description || "").replace(/\n/g, "\\n");
    const loc = (ev.location || "").replace(/,/g, "\\,");
    const startLine = useParisTz ? `DTSTART;TZID=Europe/Paris:${dtStart}` : `DTSTART:${dtStart}`;
    const endLine = useParisTz ? `DTEND;TZID=Europe/Paris:${dtEnd}` : `DTEND:${dtEnd}`;
    blocks.push(
      "BEGIN:VEVENT",
      `UID:${uid}`,
      `DTSTAMP:${stamp}`,
      startLine,
      endLine,
      `SUMMARY:${escapeIcs(ev.title)}`,
    );
    if (desc) blocks.push(`DESCRIPTION:${escapeIcs(desc)}`);
    if (loc) blocks.push(`LOCATION:${escapeIcs(loc)}`);
    blocks.push("END:VEVENT");
  }

  blocks.push("END:VCALENDAR");
  return blocks.join("\r\n");
}

function resolveIcsBounds(params: CalendarIcsEvent): {
  dtStart: string;
  dtEnd: string;
  useParisTz: boolean;
} {
  if (params.startDate && params.startTime) {
    const start = `${params.startDate.replace(/-/g, "")}T${params.startTime.replace(":", "")}00`;
    if (params.endDate && params.endTime) {
      const end = `${params.endDate.replace(/-/g, "")}T${params.endTime.replace(":", "")}00`;
      return { dtStart: start, dtEnd: end, useParisTz: true };
    }
    const mins = Math.max(5, params.durationMinutes || 30);
    const end = addMinutesParisLocal(params.startDate, params.startTime, mins);
    return { dtStart: start, dtEnd: end, useParisTz: true };
  }

  const dtStart = toIcsUtc(params.startAt || new Date().toISOString());
  const dtEnd = toIcsUtc(params.endAt || params.startAt || new Date().toISOString());
  return { dtStart, dtEnd, useParisTz: false };
}

function addMinutesParisLocal(date: string, time: string, minutes: number): string {
  const [y, m, d] = date.split("-").map(Number);
  const [hh, mm] = time.split(":").map(Number);
  const total = (hh || 0) * 60 + (mm || 0) + minutes;
  const dayAdd = Math.floor(total / (24 * 60));
  const rem = ((total % (24 * 60)) + 24 * 60) % (24 * 60);
  const nh = Math.floor(rem / 60);
  const nm = rem % 60;
  const base = new Date(Date.UTC(y, m - 1, d));
  base.setUTCDate(base.getUTCDate() + dayAdd);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${base.getUTCFullYear()}${pad(base.getUTCMonth() + 1)}${pad(base.getUTCDate())}T${pad(nh)}${pad(nm)}00`;
}

function toIcsUtc(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return formatIcsNowUtc();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getUTCFullYear()}${pad(d.getUTCMonth() + 1)}${pad(d.getUTCDate())}T${pad(d.getUTCHours())}${pad(d.getUTCMinutes())}${pad(d.getUTCSeconds())}Z`;
}

function formatIcsNowUtc(): string {
  return toIcsUtc(new Date().toISOString());
}

function escapeIcs(s: string) {
  return s.replace(/\\/g, "\\\\").replace(/;/g, "\\;").replace(/,/g, "\\,").replace(/\n/g, "\\n");
}

/** @deprecated Préférer buildTravelsParentsTripIcs (multi-événements). */
export function buildTravelsParentMeetingIcs(input: {
  tripId: string;
  tripTitle: string;
  meeting: {
    date: string;
    time: string;
    durationMinutes?: number;
    place?: string;
    note?: string;
  };
}): string {
  return buildCalendarEventIcs({
    title: `RDV — ${input.tripTitle}`,
    description: [
      `Rendez-vous parents pour la sortie « ${input.tripTitle} ».`,
      input.meeting.note?.trim() || "",
      "Ajoutez cet événement à votre calendrier (fichier .ics).",
    ]
      .filter(Boolean)
      .join("\n"),
    location: input.meeting.place?.trim() || undefined,
    startDate: input.meeting.date,
    startTime: input.meeting.time,
    durationMinutes: input.meeting.durationMinutes || 30,
    uid: `travels-rdv-${input.tripId}@scola`,
    prodId: "-//Scola//Sorties scolaires//FR",
  });
}
