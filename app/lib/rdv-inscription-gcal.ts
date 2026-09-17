import "server-only";

import {
  eventTitleMatchesPattern,
  type RdvInscriptionSlot,
} from "@/app/lib/rdv-inscription-types";
import { getRdvInscriptionGoogleAccessToken } from "@/app/lib/rdv-inscription-oauth";

const GCAL_BASE = "https://www.googleapis.com/calendar/v3";

/** Propriété privée pour marquer un créneau déjà réservé via ScolIA. */
export const SCOLA_BOOKED_PROP = "scolaBooked";
export const SCOLA_PENDING_PROP = "scolaPending";
export const SCOLA_BOOKING_ID_PROP = "scolaBookingId";
/** Couleur Google Agenda « Paon » (bleu paon). */
export const GCAL_COLOR_PAON = "7";

type GCalEventDate = {
  dateTime?: string;
  date?: string;
  timeZone?: string;
};

type GCalEvent = {
  id?: string;
  status?: string;
  summary?: string;
  description?: string;
  htmlLink?: string;
  start?: GCalEventDate;
  end?: GCalEventDate;
  extendedProperties?: {
    private?: Record<string, string>;
    shared?: Record<string, string>;
  };
  attendees?: Array<{ email?: string; displayName?: string }>;
  transparency?: string;
  etag?: string;
};

function encodeCalendarId(calendarId: string): string {
  return encodeURIComponent(calendarId);
}

async function gcalFetch(
  accessToken: string,
  path: string,
  init?: RequestInit,
): Promise<Response> {
  return fetch(`${GCAL_BASE}${path}`, {
    ...init,
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
      ...(init?.headers || {}),
    },
  });
}

function eventStartEnd(ev: GCalEvent): { startAt: string; endAt: string } | null {
  const startRaw = ev.start?.dateTime || ev.start?.date;
  const endRaw = ev.end?.dateTime || ev.end?.date;
  if (!startRaw || !endRaw) return null;
  const start = new Date(startRaw);
  const end = new Date(endRaw);
  if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) return null;
  return { startAt: start.toISOString(), endAt: end.toISOString() };
}

export function isScolaBookedEvent(ev: GCalEvent): boolean {
  const priv = ev.extendedProperties?.private || {};
  return priv[SCOLA_BOOKED_PROP] === "true";
}

export function isScolaPendingEvent(ev: GCalEvent): boolean {
  const priv = ev.extendedProperties?.private || {};
  return priv[SCOLA_PENDING_PROP] === "true";
}

export function isScolaHeldEvent(ev: GCalEvent): boolean {
  return isScolaBookedEvent(ev) || isScolaPendingEvent(ev);
}

export type ListInscriptionSlotsResult = {
  slots: RdvInscriptionSlot[];
  /** Titres d’événements à venir (hors annulés), pour diagnostiquer un motif incorrect. */
  sampleTitles: string[];
  /** Nombre d’événements à venir lus sur l’agenda (avant filtre motif). */
  upcomingEventCount: number;
};

async function listCalendarEventsInHorizon(opts: {
  calendarId: string;
  horizonDays: number;
  accessToken: string;
}): Promise<{ now: Date; items: GCalEvent[] }> {
  const calendarId = opts.calendarId.trim();
  const now = new Date();
  if (!calendarId) return { now, items: [] };

  const horizon = new Date(now.getTime() + Math.max(1, opts.horizonDays) * 24 * 60 * 60 * 1000);
  const params = new URLSearchParams({
    timeMin: now.toISOString(),
    timeMax: horizon.toISOString(),
    singleEvents: "true",
    orderBy: "startTime",
    maxResults: "250",
  });

  const res = await gcalFetch(
    opts.accessToken,
    `/calendars/${encodeCalendarId(calendarId)}/events?${params.toString()}`,
  );
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`Google Calendar list events (${res.status}) : ${body.slice(0, 400)}`);
  }

  const data = (await res.json()) as { items?: GCalEvent[] };
  return { now, items: data.items || [] };
}

export async function listAvailableInscriptionSlots(opts: {
  calendarId: string;
  titlePattern: string;
  horizonDays: number;
  accessToken?: string;
}): Promise<RdvInscriptionSlot[]> {
  const detailed = await listAvailableInscriptionSlotsDetailed(opts);
  return detailed.slots;
}

export async function listAvailableInscriptionSlotsDetailed(opts: {
  calendarId: string;
  titlePattern: string;
  horizonDays: number;
  accessToken?: string;
}): Promise<ListInscriptionSlotsResult> {
  const accessToken = opts.accessToken || (await getRdvInscriptionGoogleAccessToken());
  const calendarId = opts.calendarId.trim();
  if (!calendarId) {
    return { slots: [], sampleTitles: [], upcomingEventCount: 0 };
  }

  const { now, items } = await listCalendarEventsInHorizon({
    calendarId,
    horizonDays: opts.horizonDays,
    accessToken,
  });

  const slots: RdvInscriptionSlot[] = [];
  const sampleTitles: string[] = [];
  let upcomingEventCount = 0;

  for (const ev of items) {
    if (!ev.id || ev.status === "cancelled") continue;
    const summary = (ev.summary || "").trim() || "(sans titre)";
    const bounds = eventStartEnd(ev);
    if (!bounds) continue;
    if (new Date(bounds.startAt).getTime() < now.getTime() - 60_000) continue;

    upcomingEventCount += 1;
    if (sampleTitles.length < 12 && !sampleTitles.includes(summary)) {
      sampleTitles.push(summary);
    }

    if (isScolaHeldEvent(ev)) continue;
    if (!eventTitleMatchesPattern(summary, opts.titlePattern)) continue;

    slots.push({
      eventId: ev.id,
      calendarId,
      title: summary,
      startAt: bounds.startAt,
      endAt: bounds.endAt,
      htmlLink: ev.htmlLink?.trim() || null,
    });
  }

  return { slots, sampleTitles, upcomingEventCount };
}

export async function getCalendarEvent(opts: {
  calendarId: string;
  eventId: string;
  accessToken?: string;
}): Promise<GCalEvent | null> {
  const accessToken = opts.accessToken || (await getRdvInscriptionGoogleAccessToken());
  const res = await gcalFetch(
    accessToken,
    `/calendars/${encodeCalendarId(opts.calendarId)}/events/${encodeURIComponent(opts.eventId)}`,
  );
  if (res.status === 404) return null;
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`Google Calendar get event (${res.status}) : ${body.slice(0, 400)}`);
  }
  return (await res.json()) as GCalEvent;
}

export type HoldCalendarEventResult =
  | {
      ok: true;
      startAt: string;
      endAt: string;
      htmlLink: string | null;
      originalTitle: string;
    }
  | { ok: false; reason: "not_found" | "already_booked" | "title_mismatch" | "past" | "error"; message: string };

/** Réserve temporairement le créneau (en attente de clic sur le lien mail). */
export async function holdInscriptionCalendarEvent(opts: {
  calendarId: string;
  eventId: string;
  titlePattern: string;
  bookingId: string;
  accessToken?: string;
}): Promise<HoldCalendarEventResult> {
  const accessToken = opts.accessToken || (await getRdvInscriptionGoogleAccessToken());
  const current = await getCalendarEvent({
    calendarId: opts.calendarId,
    eventId: opts.eventId,
    accessToken,
  });
  if (!current?.id) {
    return { ok: false, reason: "not_found", message: "Créneau introuvable." };
  }
  if (current.status === "cancelled") {
    return { ok: false, reason: "not_found", message: "Créneau annulé." };
  }
  if (isScolaHeldEvent(current)) {
    return { ok: false, reason: "already_booked", message: "Ce créneau vient d’être pris." };
  }
  const summary = (current.summary || "").trim();
  if (!eventTitleMatchesPattern(summary, opts.titlePattern)) {
    return {
      ok: false,
      reason: "title_mismatch",
      message: "Ce créneau n’est plus proposé pour les inscriptions.",
    };
  }
  const bounds = eventStartEnd(current);
  if (!bounds) {
    return { ok: false, reason: "error", message: "Horaires d’événement invalides." };
  }
  if (new Date(bounds.startAt).getTime() < Date.now() - 60_000) {
    return { ok: false, reason: "past", message: "Ce créneau est déjà passé." };
  }

  const priv = {
    ...(current.extendedProperties?.private || {}),
    [SCOLA_PENDING_PROP]: "true",
    [SCOLA_BOOKING_ID_PROP]: opts.bookingId,
  };

  const ifMatch = current.etag?.trim();
  const res = await gcalFetch(
    accessToken,
    `/calendars/${encodeCalendarId(opts.calendarId)}/events/${encodeURIComponent(opts.eventId)}`,
    {
      method: "PATCH",
      headers: ifMatch ? { "If-Match": ifMatch } : undefined,
      body: JSON.stringify({
        extendedProperties: {
          private: priv,
          shared: current.extendedProperties?.shared || undefined,
        },
      }),
    },
  );

  if (res.status === 412) {
    return {
      ok: false,
      reason: "already_booked",
      message: "Ce créneau vient d’être pris par quelqu’un d’autre.",
    };
  }
  if (!res.ok) {
    const body = await res.text();
    return {
      ok: false,
      reason: "error",
      message: `Impossible de réserver le créneau (${res.status}) : ${body.slice(0, 300)}`,
    };
  }

  const updated = (await res.json()) as GCalEvent;
  return {
    ok: true,
    startAt: bounds.startAt,
    endAt: bounds.endAt,
    htmlLink: updated.htmlLink?.trim() || current.htmlLink?.trim() || null,
    originalTitle: summary,
  };
}

/** Libère un hold pending (lien expiré / abandon). */
export async function releaseInscriptionCalendarHold(opts: {
  calendarId: string;
  eventId: string;
  bookingId: string;
  accessToken?: string;
}): Promise<void> {
  const accessToken = opts.accessToken || (await getRdvInscriptionGoogleAccessToken());
  const current = await getCalendarEvent({
    calendarId: opts.calendarId,
    eventId: opts.eventId,
    accessToken,
  });
  if (!current?.id) return;
  if (isScolaBookedEvent(current)) return;
  const priv = { ...(current.extendedProperties?.private || {}) };
  if (priv[SCOLA_BOOKING_ID_PROP] && priv[SCOLA_BOOKING_ID_PROP] !== opts.bookingId) return;
  delete priv[SCOLA_PENDING_PROP];
  if (priv[SCOLA_BOOKING_ID_PROP] === opts.bookingId) delete priv[SCOLA_BOOKING_ID_PROP];

  await gcalFetch(
    accessToken,
    `/calendars/${encodeCalendarId(opts.calendarId)}/events/${encodeURIComponent(opts.eventId)}`,
    {
      method: "PATCH",
      body: JSON.stringify({
        extendedProperties: {
          private: priv,
          shared: current.extendedProperties?.shared || undefined,
        },
      }),
    },
  );
}

export type BookCalendarEventResult =
  | {
      ok: true;
      event: GCalEvent;
      startAt: string;
      endAt: string;
      htmlLink: string | null;
    }
  | { ok: false; reason: "not_found" | "already_booked" | "title_mismatch" | "past" | "error"; message: string };

/** Confirme définitivement le créneau après validation e-mail. */
export async function confirmInscriptionCalendarEvent(opts: {
  calendarId: string;
  eventId: string;
  bookingId: string;
  studentFirstName: string;
  studentLastName: string;
  parentEmail: string;
  parentPhone: string;
  accessToken?: string;
}): Promise<BookCalendarEventResult> {
  const accessToken = opts.accessToken || (await getRdvInscriptionGoogleAccessToken());

  const current = await getCalendarEvent({
    calendarId: opts.calendarId,
    eventId: opts.eventId,
    accessToken,
  });
  if (!current?.id) {
    return { ok: false, reason: "not_found", message: "Créneau introuvable." };
  }
  if (current.status === "cancelled") {
    return { ok: false, reason: "not_found", message: "Créneau annulé." };
  }
  if (isScolaBookedEvent(current)) {
    const priv = current.extendedProperties?.private || {};
    if (priv[SCOLA_BOOKING_ID_PROP] === opts.bookingId) {
      const bounds = eventStartEnd(current);
      if (!bounds) {
        return { ok: false, reason: "error", message: "Horaires d’événement invalides." };
      }
      return {
        ok: true,
        event: current,
        startAt: bounds.startAt,
        endAt: bounds.endAt,
        htmlLink: current.htmlLink?.trim() || null,
      };
    }
    return { ok: false, reason: "already_booked", message: "Ce créneau vient d’être pris." };
  }

  const bounds = eventStartEnd(current);
  if (!bounds) {
    return { ok: false, reason: "error", message: "Horaires d’événement invalides." };
  }
  if (new Date(bounds.startAt).getTime() < Date.now() - 60_000) {
    return { ok: false, reason: "past", message: "Ce créneau est déjà passé." };
  }

  const studentLabel = `${opts.studentLastName.trim().toUpperCase()} ${opts.studentFirstName.trim()}`;
  const newTitle = `Rendez-vous — ${studentLabel}`;
  const descriptionLines = [
    `Élève : ${opts.studentFirstName.trim()} ${opts.studentLastName.trim()}`,
    `E-mail parent : ${opts.parentEmail.trim()}`,
    `Téléphone parent : ${opts.parentPhone.trim()}`,
    `Réf. : ${opts.bookingId}`,
  ];

  const priv: Record<string, string> = {
    ...(current.extendedProperties?.private || {}),
    [SCOLA_BOOKED_PROP]: "true",
    [SCOLA_BOOKING_ID_PROP]: opts.bookingId,
  };
  delete priv[SCOLA_PENDING_PROP];

  const patchBody: Record<string, unknown> = {
    summary: newTitle,
    description: descriptionLines.join("\n"),
    colorId: GCAL_COLOR_PAON,
    transparency: "opaque",
    extendedProperties: {
      private: priv,
      shared: current.extendedProperties?.shared || undefined,
    },
    attendees: [
      ...(current.attendees || []).filter(
        (a) => a.email && a.email.toLowerCase() !== opts.parentEmail.trim().toLowerCase(),
      ),
      {
        email: opts.parentEmail.trim().toLowerCase(),
        displayName: `Parent — ${studentLabel}`,
      },
    ],
  };

  const ifMatch = current.etag?.trim();
  const res = await gcalFetch(
    accessToken,
    `/calendars/${encodeCalendarId(opts.calendarId)}/events/${encodeURIComponent(opts.eventId)}?sendUpdates=all`,
    {
      method: "PATCH",
      headers: ifMatch ? { "If-Match": ifMatch } : undefined,
      body: JSON.stringify(patchBody),
    },
  );

  if (res.status === 412) {
    return {
      ok: false,
      reason: "already_booked",
      message: "Ce créneau vient d’être pris par quelqu’un d’autre.",
    };
  }
  if (!res.ok) {
    const body = await res.text();
    return {
      ok: false,
      reason: "error",
      message: `Impossible de confirmer sur Google Agenda (${res.status}) : ${body.slice(0, 300)}`,
    };
  }

  const updated = (await res.json()) as GCalEvent;
  const updatedBounds = eventStartEnd(updated) || bounds;
  return {
    ok: true,
    event: updated,
    startAt: updatedBounds.startAt,
    endAt: updatedBounds.endAt,
    htmlLink: updated.htmlLink?.trim() || current.htmlLink?.trim() || null,
  };
}
