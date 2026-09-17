import "server-only";

import { randomBytes, randomUUID } from "crypto";
import {
  findActiveBookingByGoogleEvent,
  getRdvInscriptionConfig,
  getRdvInscriptionDirectionBySlug,
  insertRdvInscriptionBooking,
  listExpiredPendingBookings,
  markRdvInscriptionBookingConfirmed,
  markRdvInscriptionBookingExpired,
  findBookingByConfirmToken,
} from "@/app/lib/rdv-inscription-db";
import {
  confirmInscriptionCalendarEvent,
  holdInscriptionCalendarEvent,
  listAvailableInscriptionSlots,
  releaseInscriptionCalendarHold,
} from "@/app/lib/rdv-inscription-gcal";
import {
  sendRdvInscriptionConfirmationMails,
  sendRdvInscriptionValidationMail,
} from "@/app/lib/rdv-inscription-mail";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import type {
  RdvInscriptionBookInput,
  RdvInscriptionBookingRow,
  RdvInscriptionSlot,
} from "@/app/lib/rdv-inscription-types";

/** Délai pour cliquer le lien de validation (anti-spam). */
export const RDV_CONFIRM_TTL_MS = 2 * 60 * 60 * 1000;

async function releaseExpiredPendings(etablissementId?: string): Promise<void> {
  const expired = await listExpiredPendingBookings({ etablissementId, limit: 40 });
  for (const b of expired) {
    try {
      await releaseInscriptionCalendarHold({
        calendarId: b.googleCalendarId,
        eventId: b.googleEventId,
        bookingId: b.id,
      });
    } catch (e) {
      console.error("[rdv-inscription] release hold expiré:", e);
    }
    await markRdvInscriptionBookingExpired({
      bookingId: b.id,
      etablissementId: b.etablissementId,
    });
  }
}

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
  const direction = await getRdvInscriptionDirectionBySlug(slug, { activeOnly: true });
  if (!direction) {
    return { ok: false, status: 404, error: "Direction introuvable." };
  }
  if (!config.googleLinked) {
    return {
      ok: false,
      status: 503,
      error: "Agenda non connecté — contactez l’établissement.",
    };
  }
  if (!direction.googleCalendarId.trim()) {
    return {
      ok: false,
      status: 503,
      error: "Agenda de cette direction non configuré.",
    };
  }

  try {
    await releaseExpiredPendings();
    const slots = await listAvailableInscriptionSlots({
      calendarId: direction.googleCalendarId,
      titlePattern: direction.eventTitlePattern,
      horizonDays: direction.horizonDays,
    });
    return {
      ok: true,
      configTitle: direction.title,
      intro: direction.intro,
      consentLabel: direction.consentLabel,
      location: direction.location,
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
  | { ok: true; pending: true; booking: RdvInscriptionBookingRow; mailWarning?: string }
  | { ok: false; status: number; error: string }
> {
  const config = await getRdvInscriptionConfig();
  const direction = await getRdvInscriptionDirectionBySlug(slug, { activeOnly: true });
  if (!direction?.googleCalendarId.trim()) {
    return { ok: false, status: 404, error: "Direction introuvable." };
  }
  if (!config.googleLinked) {
    return { ok: false, status: 503, error: "Agenda non connecté." };
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

  await releaseExpiredPendings();

  const existing = await findActiveBookingByGoogleEvent({
    calendarId: direction.googleCalendarId,
    eventId,
  });
  if (existing) {
    return { ok: false, status: 409, error: "Ce créneau vient d’être pris." };
  }

  const bookingId = randomUUID();
  const confirmToken = randomBytes(32).toString("hex");
  const confirmExpiresAt = new Date(Date.now() + RDV_CONFIRM_TTL_MS);

  const hold = await holdInscriptionCalendarEvent({
    calendarId: direction.googleCalendarId,
    eventId,
    titlePattern: direction.eventTitlePattern,
    bookingId,
  });

  if (!hold.ok) {
    const status =
      hold.reason === "already_booked"
        ? 409
        : hold.reason === "not_found" || hold.reason === "past" || hold.reason === "title_mismatch"
          ? 410
          : 502;
    return { ok: false, status, error: hold.message };
  }

  let booking: RdvInscriptionBookingRow;
  try {
    booking = await insertRdvInscriptionBooking({
      bookingId,
      directionId: direction.id,
      directionSlug: direction.slug,
      googleEventId: eventId,
      googleCalendarId: direction.googleCalendarId,
      googleHtmlLink: hold.htmlLink,
      startAt: new Date(hold.startAt),
      endAt: new Date(hold.endAt),
      studentFirstName,
      studentLastName,
      parentEmail,
      parentPhone,
      status: "pending",
      confirmToken,
      confirmExpiresAt,
    });
  } catch (e) {
    try {
      await releaseInscriptionCalendarHold({
        calendarId: direction.googleCalendarId,
        eventId,
        bookingId,
      });
    } catch {
      /* ignore */
    }
    const msg = e instanceof Error ? e.message : String(e);
    if (/unique|duplicate/i.test(msg)) {
      return { ok: false, status: 409, error: "Ce créneau vient d’être pris." };
    }
    throw e;
  }

  const confirmUrl = await tenantAbsolutePath(
    `/api/rdv-inscription/confirm?token=${encodeURIComponent(confirmToken)}`,
  );

  const mail = await sendRdvInscriptionValidationMail({
    page: {
      title: direction.title,
      intro: direction.intro,
      eventTitlePattern: direction.eventTitlePattern,
      notifyEmail: direction.notifyEmail,
      location: direction.location,
      consentLabel: direction.consentLabel,
      horizonDays: direction.horizonDays,
    },
    booking,
    directionLabel: direction.label,
    directriceName: direction.directriceDisplayName,
    confirmUrl,
    expiresAt: confirmExpiresAt,
  });

  return {
    ok: true,
    pending: true,
    booking,
    mailWarning: mail.error,
  };
}

export async function confirmPublicRdvInscription(token: string): Promise<
  | { ok: true; booking: RdvInscriptionBookingRow; already?: boolean; mailWarning?: string }
  | { ok: false; error: "invalid" | "expired" | "taken" | "error"; message: string }
> {
  const found = await findBookingByConfirmToken(token);
  if (!found) {
    return { ok: false, error: "invalid", message: "Lien de validation invalide ou déjà utilisé." };
  }

  if (found.status === "confirmed") {
    return { ok: true, booking: found, already: true };
  }

  if (found.status !== "pending") {
    return { ok: false, error: "invalid", message: "Cette demande n’est plus valide." };
  }

  if (found.confirmExpiresAt && new Date(found.confirmExpiresAt).getTime() <= Date.now()) {
    try {
      await releaseInscriptionCalendarHold({
        calendarId: found.googleCalendarId,
        eventId: found.googleEventId,
        bookingId: found.id,
      });
    } catch {
      /* ignore */
    }
    await markRdvInscriptionBookingExpired({
      bookingId: found.id,
      etablissementId: found.etablissementId,
    });
    return {
      ok: false,
      error: "expired",
      message: "Le lien a expiré. Merci de reprendre un créneau.",
    };
  }

  const direction = await getRdvInscriptionDirectionBySlug(found.directionSlug, {
    etablissementId: found.etablissementId,
  });
  if (!direction) {
    return { ok: false, error: "error", message: "Direction introuvable." };
  }

  const gcal = await confirmInscriptionCalendarEvent({
    calendarId: found.googleCalendarId,
    eventId: found.googleEventId,
    bookingId: found.id,
    studentFirstName: found.studentFirstName,
    studentLastName: found.studentLastName,
    parentEmail: found.parentEmail,
    parentPhone: found.parentPhone,
  });

  if (!gcal.ok) {
    if (gcal.reason === "already_booked") {
      return { ok: false, error: "taken", message: gcal.message };
    }
    return { ok: false, error: "error", message: gcal.message };
  }

  const booking = await markRdvInscriptionBookingConfirmed({
    bookingId: found.id,
    etablissementId: found.etablissementId,
    googleHtmlLink: gcal.htmlLink,
  });
  if (!booking) {
    return { ok: false, error: "error", message: "Confirmation impossible." };
  }

  const mail = await sendRdvInscriptionConfirmationMails({
    page: {
      title: direction.title,
      intro: direction.intro,
      eventTitlePattern: direction.eventTitlePattern,
      notifyEmail: direction.notifyEmail,
      location: direction.location,
      consentLabel: direction.consentLabel,
      horizonDays: direction.horizonDays,
    },
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
