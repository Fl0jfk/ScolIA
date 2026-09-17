import "server-only";

import { randomUUID } from "crypto";
import {
  findBookingByGoogleEvent,
  getRdvInscriptionConfig,
  getRdvInscriptionDirectionBySlug,
  insertRdvInscriptionBooking,
} from "@/app/lib/rdv-inscription-db";
import { bookInscriptionCalendarEvent, listAvailableInscriptionSlots } from "@/app/lib/rdv-inscription-gcal";
import { sendRdvInscriptionConfirmationMails } from "@/app/lib/rdv-inscription-mail";
import type {
  RdvInscriptionBookInput,
  RdvInscriptionBookingRow,
  RdvInscriptionSlot,
} from "@/app/lib/rdv-inscription-types";

export async function listPublicSlotsForDirection(slug: string): Promise<{
  ok: true;
  configTitle: string;
  intro: string;
  consentLabel: string;
  location: string;
  directionLabel: string;
  directriceDisplayName: string | null;
  slots: RdvInscriptionSlot[];
} | { ok: false; status: number; error: string }> {
  const config = await getRdvInscriptionConfig();
  if (!config.enabled) {
    return { ok: false, status: 404, error: "Prise de rendez-vous non disponible." };
  }
  if (!config.googleLinked) {
    return {
      ok: false,
      status: 503,
      error: "Agenda non connecté — contactez l’établissement.",
    };
  }

  const direction = await getRdvInscriptionDirectionBySlug(slug, { activeOnly: true });
  if (!direction) {
    return { ok: false, status: 404, error: "Direction introuvable." };
  }
  if (!direction.googleCalendarId.trim()) {
    return {
      ok: false,
      status: 503,
      error: "Agenda de cette direction non configuré.",
    };
  }

  try {
    const slots = await listAvailableInscriptionSlots({
      calendarId: direction.googleCalendarId,
      titlePattern: config.eventTitlePattern,
      horizonDays: config.horizonDays,
    });
    return {
      ok: true,
      configTitle: config.title,
      intro: config.intro,
      consentLabel: config.consentLabel,
      location: config.location,
      directionLabel: direction.label,
      directriceDisplayName: direction.directriceDisplayName,
      slots,
    };
  } catch (e) {
    return {
      ok: false,
      status: 502,
      error: e instanceof Error ? e.message : "Impossible de lire Google Agenda.",
    };
  }
}

export async function bookPublicRdvInscription(
  slug: string,
  input: RdvInscriptionBookInput,
): Promise<
  | { ok: true; booking: RdvInscriptionBookingRow; mailWarning?: string }
  | { ok: false; status: number; error: string }
> {
  const config = await getRdvInscriptionConfig();
  if (!config.enabled) {
    return { ok: false, status: 404, error: "Prise de rendez-vous non disponible." };
  }
  if (!config.googleLinked) {
    return { ok: false, status: 503, error: "Agenda non connecté." };
  }

  const direction = await getRdvInscriptionDirectionBySlug(slug, { activeOnly: true });
  if (!direction?.googleCalendarId.trim()) {
    return { ok: false, status: 404, error: "Direction introuvable." };
  }

  const studentFirstName = input.studentFirstName.trim();
  const studentLastName = input.studentLastName.trim();
  const parentEmail = input.parentEmail.trim().toLowerCase();
  const parentPhone = input.parentPhone.trim();
  const eventId = input.eventId.trim();

  if (!eventId || !studentFirstName || !studentLastName || !parentEmail || !parentPhone) {
    return {
      ok: false,
      status: 400,
      error: "Créneau, élève, e-mail et téléphone sont requis.",
    };
  }
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(parentEmail)) {
    return { ok: false, status: 400, error: "E-mail invalide." };
  }
  if (studentFirstName.length > 80 || studentLastName.length > 80) {
    return { ok: false, status: 400, error: "Nom / prénom trop longs." };
  }
  if (parentPhone.length > 40) {
    return { ok: false, status: 400, error: "Téléphone trop long." };
  }

  const existing = await findBookingByGoogleEvent({
    calendarId: direction.googleCalendarId,
    eventId,
  });
  if (existing) {
    return { ok: false, status: 409, error: "Ce créneau vient d’être pris." };
  }

  const bookingId = randomUUID();
  const gcal = await bookInscriptionCalendarEvent({
    calendarId: direction.googleCalendarId,
    eventId,
    titlePattern: config.eventTitlePattern,
    bookingId,
    studentFirstName,
    studentLastName,
    parentEmail,
    parentPhone,
  });

  if (!gcal.ok) {
    const status =
      gcal.reason === "already_booked"
        ? 409
        : gcal.reason === "not_found" || gcal.reason === "past" || gcal.reason === "title_mismatch"
          ? 410
          : 502;
    return { ok: false, status, error: gcal.message };
  }

  let booking: RdvInscriptionBookingRow;
  try {
    booking = await insertRdvInscriptionBooking({
      bookingId,
      directionId: direction.id,
      directionSlug: direction.slug,
      googleEventId: eventId,
      googleCalendarId: direction.googleCalendarId,
      googleHtmlLink: gcal.htmlLink,
      startAt: new Date(gcal.startAt),
      endAt: new Date(gcal.endAt),
      studentFirstName,
      studentLastName,
      parentEmail,
      parentPhone,
    });
  } catch (e) {
    // Concurrence unique index : le créneau a déjà une ligne
    const msg = e instanceof Error ? e.message : String(e);
    if (/unique|duplicate/i.test(msg)) {
      return { ok: false, status: 409, error: "Ce créneau vient d’être pris." };
    }
    throw e;
  }

  const mail = await sendRdvInscriptionConfirmationMails({
    config,
    booking,
    directionLabel: direction.label,
    directriceName: direction.directriceDisplayName,
  });

  return {
    ok: true,
    booking,
    mailWarning: mail.error,
  };
}
