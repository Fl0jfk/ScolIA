import "server-only";

import { randomBytes, randomUUID } from "crypto";
import {
  findActiveBookingByGoogleEvent,
  getRdvInscriptionConfig,
  getRdvInscriptionDirectionBySlug,
  insertRdvInscriptionBooking,
  listExpiredPendingBookings,
  listActiveBookingsForEleve,
  markRdvInscriptionBookingCancelled,
  markRdvInscriptionBookingConfirmed,
  markRdvInscriptionBookingExpired,
  findBookingByConfirmToken,
  findBookingByReconfirmToken,
  findRdvInscriptionBookingById,
  markRdvInscriptionReconfirm,
  listBookingsDueForReconfirmMail,
  markRdvInscriptionReconfirmMailSent,
} from "@/app/lib/rdv-inscription-db";
import {
  confirmInscriptionCalendarEvent,
  holdInscriptionCalendarEvent,
  listAvailableInscriptionSlots,
  releaseInscriptionCalendarHold,
  restoreInscriptionCalendarSlot,
  markParentReconfirmedOnCalendar,
  cancelConfirmedInscriptionEvent,
} from "@/app/lib/rdv-inscription-gcal";
import {
  sendRdvInscriptionConfirmationMails,
  sendRdvInscriptionReconfirmMail,
  sendRdvInscriptionCreatedPreinscritNotify,
} from "@/app/lib/rdv-inscription-mail";
import {
  assertEleveAllowedForRdvParent,
  assertEleveStillMatchesRdvBooking,
  createPreinscritFromRdvBooking,
  searchRdvInscriptionByIdentity,
  searchRdvInscriptionMatchCandidates,
} from "@/app/lib/rdv-inscription-eleve";
import { normalizeEleveDateNaissance } from "@/app/lib/eleves-config";
import {
  getInscriptionLevelMeta,
  isInscriptionLevelId,
  inscriptionLevelsForDirectionSlug,
} from "@/app/lib/document-templates/inscription-levels";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import type {
  RdvInscriptionBookInput,
  RdvInscriptionBookingRow,
  RdvInscriptionSlot,
} from "@/app/lib/rdv-inscription-types";
import {
  DEFAULT_RDV_INSCRIPTION_TITLE,
  RDV_BOOK_CONFIRM_PHRASE,
  splitRdvTitlePatterns,
} from "@/app/lib/rdv-inscription-types";
import type { RdvMatchCandidate } from "@/app/lib/rdv-inscription-match";
import { isValidParentEmail } from "@/app/lib/eleves-parent-emails";

/** Délai de filet pour un hold pending non finalisé (échec technique). */
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

function directionPageSettings(direction: {
  title: string;
  intro: string;
  eventTitlePattern: string;
  notifyEmail: string | null;
  location: string;
  consentLabel: string;
  horizonDays: number;
}) {
  return {
    title: direction.title,
    intro: direction.intro,
    eventTitlePattern: direction.eventTitlePattern,
    notifyEmail: direction.notifyEmail,
    location: direction.location,
    consentLabel: direction.consentLabel,
    horizonDays: direction.horizonDays,
  };
}

export async function listPublicSlotsForDirection(slug: string): Promise<{
  ok: true;
  configTitle: string;
  intro: string;
  consentLabel: string;
  location: string;
  directionLabel: string;
  directriceDisplayName: string | null;
  levels: Array<{ id: string; label: string }>;
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

  const levels = inscriptionLevelsForDirectionSlug(slug).map((l) => ({
    id: l.id,
    label: l.label,
  }));

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
      levels,
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

export async function matchPublicRdvInscription(opts: {
  slug: string;
  parentEmail: string;
  parentPhone?: string;
  studentFirstName: string;
  studentLastName: string;
  /** Obligatoire pour le matching identité (foyers séparés). */
  dateNaissance: string;
}): Promise<
  | {
      ok: true;
      candidates: RdvMatchCandidate[];
      homeEtablissement: { codeRne: string; label: string; adresse: string | null } | null;
      mode: "contact" | "identity";
    }
  | { ok: false; status: number; error: string }
> {
  const direction = await getRdvInscriptionDirectionBySlug(opts.slug, { activeOnly: true });
  if (!direction) {
    return { ok: false, status: 404, error: "Direction introuvable." };
  }
  const parentEmail = opts.parentEmail.trim().toLowerCase();
  if (!isValidParentEmail(parentEmail)) {
    return { ok: false, status: 400, error: "E-mail parent requis et valide." };
  }
  const studentFirstName = opts.studentFirstName.trim();
  const studentLastName = opts.studentLastName.trim();
  if (!studentFirstName && !studentLastName) {
    return {
      ok: false,
      status: 400,
      error: "Indiquez au moins le nom ou le prénom de l’élève (comme sur École Directe).",
    };
  }
  const dateNaissance = normalizeEleveDateNaissance(opts.dateNaissance);
  if (!dateNaissance) {
    return {
      ok: false,
      status: 400,
      error: "Date de naissance de l’élève requise (JJ/MM/AAAA).",
    };
  }

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return { ok: false, status: 503, error: "Établissement introuvable." };
  }

  const { getHomeEtablissementForRdv } = await import("@/app/lib/rdv-inscription-eleve");
  const homeEtablissement = await getHomeEtablissementForRdv(etabId);

  // 1) Si l’e-mail est déjà connu sur un foyer : matching contact + nom (pas d’identité globale).
  const byContact = await searchRdvInscriptionMatchCandidates({
    etablissementId: etabId,
    parentEmail,
    parentPhone: opts.parentPhone,
    studentFirstName,
    studentLastName,
  });
  if (byContact.length) {
    return { ok: true, candidates: byContact, homeEtablissement, mode: "contact" };
  }

  // 2) E-mail non reconnu : identité (naissance + nom OU prénom), jamais de création de dossier.
  const byIdentity = await searchRdvInscriptionByIdentity({
    etablissementId: etabId,
    studentFirstName,
    studentLastName,
    dateNaissance,
  });
  return { ok: true, candidates: byIdentity, homeEtablissement, mode: "identity" };
}

export function normalizeRdvBookConfirmPhrase(value: string): string {
  return value
    .trim()
    .toUpperCase()
    .normalize("NFD")
    .replace(/\p{M}/gu, "");
}

export function isValidRdvBookConfirmPhrase(value: string | undefined | null): boolean {
  return normalizeRdvBookConfirmPhrase(value || "") === RDV_BOOK_CONFIRM_PHRASE;
}

function restoreTitleForDirection(eventTitlePattern: string): string {
  const first = splitRdvTitlePatterns(eventTitlePattern)[0]?.trim();
  return first || DEFAULT_RDV_INSCRIPTION_TITLE;
}

/**
 * Libère les RDV actifs du même élève (même direction) pour permettre un changement de créneau.
 * Remet l’événement Google à l’état libre (titre motif).
 */
async function supersedeActiveBookingsForEleve(opts: {
  eleveId: string;
  directionSlug: string;
  etablissementId: string;
  titlePattern: string;
  keepEventId?: string | null;
}): Promise<{ supersededIds: string[]; warnings: string[] }> {
  const active = await listActiveBookingsForEleve({
    eleveId: opts.eleveId,
    directionSlug: opts.directionSlug,
    etablissementId: opts.etablissementId,
  });
  const supersededIds: string[] = [];
  const warnings: string[] = [];
  const restoreTitle = restoreTitleForDirection(opts.titlePattern);
  const keepEventId = opts.keepEventId?.trim() || "";

  for (const prev of active) {
    if (keepEventId && prev.googleEventId === keepEventId) {
      continue;
    }
    try {
      if (prev.status === "pending") {
        await releaseInscriptionCalendarHold({
          calendarId: prev.googleCalendarId,
          eventId: prev.googleEventId,
          bookingId: prev.id,
        });
      } else if (prev.status === "confirmed") {
        const restored = await restoreInscriptionCalendarSlot({
          calendarId: prev.googleCalendarId,
          eventId: prev.googleEventId,
          bookingId: prev.id,
          restoreTitle,
        });
        if (!restored.ok) {
          warnings.push(`${prev.id}: ${restored.message}`);
        }
      }
      await markRdvInscriptionBookingCancelled({
        bookingId: prev.id,
        etablissementId: opts.etablissementId,
      });
      supersededIds.push(prev.id);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      warnings.push(`${prev.id}: ${msg}`);
      console.error("[rdv-inscription] supersede:", e);
    }
  }

  return { supersededIds, warnings };
}

/** RDV actif déjà pris pour cet élève (affichage public « vous avez déjà un rendez-vous »). */
export async function getActiveRdvBookingForEleve(opts: {
  directionSlug: string;
  eleveId: string;
  parentEmail: string;
}): Promise<
  | { ok: true; booking: RdvInscriptionBookingRow | null }
  | { ok: false; status: number; error: string }
> {
  const slug = opts.directionSlug.trim().toLowerCase();
  const eleveId = opts.eleveId.trim();
  const parentEmail = opts.parentEmail.trim().toLowerCase();
  if (!slug || !eleveId || !parentEmail) {
    return { ok: false, status: 400, error: "Paramètres manquants." };
  }
  if (!isValidParentEmail(parentEmail)) {
    return { ok: false, status: 400, error: "E-mail invalide." };
  }

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return { ok: false, status: 503, error: "Établissement introuvable." };
  }

  const active = await listActiveBookingsForEleve({
    eleveId,
    directionSlug: slug,
    etablissementId: etabId,
  });
  const booking = active.find((b) => b.parentEmail === parentEmail) || null;
  if (active.length > 0 && !booking) {
    return { ok: false, status: 403, error: "Élève non rattaché à ces coordonnées parent." };
  }
  return { ok: true, booking };
}

export async function bookPublicRdvInscription(
  slug: string,
  input: RdvInscriptionBookInput,
): Promise<
  | { ok: true; pending: false; booking: RdvInscriptionBookingRow; mailWarning?: string }
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

  if (!isValidRdvBookConfirmPhrase(input.confirmTyped)) {
    return {
      ok: false,
      status: 400,
      error: `Pour confirmer, saisissez ${RDV_BOOK_CONFIRM_PHRASE} dans le champ prévu.`,
    };
  }

  const studentFirstName = input.studentFirstName.trim();
  const studentLastName = input.studentLastName.trim();
  const parentEmail = input.parentEmail.trim().toLowerCase();
  const parentPhone = input.parentPhone.trim();
  const parentFirstName = input.parentFirstName?.trim() || "";
  const parentLastName = input.parentLastName?.trim() || "";
  const rdvAttendee =
    input.rdvAttendee === "madame" ||
    input.rdvAttendee === "monsieur" ||
    input.rdvAttendee === "les_deux"
      ? input.rdvAttendee
      : null;
  const eventId = input.eventId.trim();
  const niveauId = input.niveauId.trim();
  const createNew = Boolean(input.createNew);
  const eleveId = input.eleveId?.trim() || null;

  if (!eventId || !studentFirstName || !studentLastName || !parentEmail || !parentPhone) {
    return {
      ok: false,
      status: 400,
      error: "Créneau, élève, e-mail et téléphone sont requis.",
    };
  }
  if (!parentFirstName || !parentLastName) {
    return {
      ok: false,
      status: 400,
      error: "Indiquez le prénom et le nom du parent qui prend rendez-vous.",
    };
  }
  // rdvAttendee optionnel : utile seulement quand on déduit les parents du dossier ;
  // si le parent saisit son nom, on sait déjà qui vient.
  if (parentFirstName.length > 80 || parentLastName.length > 80) {
    return { ok: false, status: 400, error: "Nom / prénom du parent trop longs." };
  }
  if (!isValidParentEmail(parentEmail)) {
    return { ok: false, status: 400, error: "E-mail invalide." };
  }
  if (!isInscriptionLevelId(niveauId)) {
    return { ok: false, status: 400, error: "Niveau demandé invalide." };
  }
  const allowedLevels = inscriptionLevelsForDirectionSlug(slug);
  if (!allowedLevels.some((l) => l.id === niveauId)) {
    return { ok: false, status: 400, error: "Niveau non proposé pour cette direction." };
  }
  const niveauMeta = getInscriptionLevelMeta(niveauId);
  if (!niveauMeta) {
    return { ok: false, status: 400, error: "Niveau inconnu." };
  }
  if (createNew) {
    return {
      ok: false,
      status: 400,
      error: "La création d’un nouveau dossier n’est pas disponible ici. Retrouvez l’élève déjà préinscrit.",
    };
  }
  if (!eleveId) {
    return {
      ok: false,
      status: 400,
      error: "Confirmez l’élève trouvé avant de réserver.",
    };
  }
  if (studentFirstName.length > 80 || studentLastName.length > 80) {
    return { ok: false, status: 400, error: "Nom / prénom trop longs." };
  }
  if (parentPhone.length > 40) {
    return { ok: false, status: 400, error: "Téléphone trop long." };
  }

  const hasPap = input.hasPap === "yes" || input.hasPap === "no" ? input.hasPap : null;
  if (!hasPap) {
    return { ok: false, status: 400, error: "Indiquez si l’élève a un PAP." };
  }
  const papS3Key = input.papS3Key?.trim() || null;
  const papFileName = input.papFileName?.trim() || null;
  const papMimeType = input.papMimeType?.trim() || null;
  const papBringToRdv = Boolean(input.papBringToRdv);
  if (hasPap === "yes" && !papS3Key && !papBringToRdv) {
    return {
      ok: false,
      status: 400,
      error:
        "Déposez le PAP ou confirmez que vous l’apporterez au rendez-vous (obligatoire le jour J).",
    };
  }

  const etablissementOrigineRne = input.etablissementOrigineRne?.trim() || null;
  const etablissementOrigineLabel = input.etablissementOrigineLabel?.trim() || null;
  const etablissementOrigineAdresse = input.etablissementOrigineAdresse?.trim() || null;
  if (!etablissementOrigineLabel) {
    return { ok: false, status: 400, error: "Sélectionnez l’établissement d’origine." };
  }

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return { ok: false, status: 503, error: "Établissement introuvable." };
  }

  if (eleveId) {
    const ok = await assertEleveAllowedForRdvParent({
      etablissementId: etabId,
      eleveId,
      parentEmail,
      studentFirstName,
      studentLastName,
      dateNaissance: input.studentDateNaissance,
    });
    if (!ok) {
      return {
        ok: false,
        status: 403,
        error: "Élève non rattaché à ces coordonnées parent.",
      };
    }
  }

  const resolvedEleveId = eleveId as string;

  await releaseExpiredPendings();

  // Changement de créneau : libère / remet à l’origine les RDV actifs du même élève.
  const supersede = await supersedeActiveBookingsForEleve({
    eleveId: resolvedEleveId,
    directionSlug: direction.slug,
    etablissementId: etabId,
    titlePattern: direction.eventTitlePattern,
    keepEventId: eventId,
  });
  if (supersede.warnings.length) {
    console.warn("[rdv-inscription] supersede warnings:", supersede.warnings);
  }

  const existing = await findActiveBookingByGoogleEvent({
    calendarId: direction.googleCalendarId,
    eventId,
  });
  if (existing) {
    return { ok: false, status: 409, error: "Ce créneau vient d’être pris." };
  }

  const bookingId = randomUUID();
  const holdExpiresAt = new Date(Date.now() + RDV_CONFIRM_TTL_MS);

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

  let pendingBooking: RdvInscriptionBookingRow;
  try {
    pendingBooking = await insertRdvInscriptionBooking({
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
      parentFirstName,
      parentLastName,
      rdvAttendee,
      niveauId: niveauMeta.id,
      niveauLabel: niveauMeta.label,
      eleveId,
      createNew,
      hasPap,
      papS3Key,
      papFileName,
      papMimeType,
      papBringToRdv: hasPap === "yes" && !papS3Key ? true : papBringToRdv,
      etablissementOrigineRne,
      etablissementOrigineLabel,
      etablissementOrigineAdresse,
      status: "pending",
      confirmToken: null,
      // Filet si la finalisation échoue : libère le créneau après TTL.
      confirmExpiresAt: holdExpiresAt,
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

  const confirmed = await finalizeRdvInscriptionBookingConfirmation({
    ...pendingBooking,
    etablissementId: etabId,
  });
  if (!confirmed.ok) {
    return {
      ok: false,
      status:
        confirmed.error === "taken"
          ? 409
          : confirmed.error === "expired"
            ? 410
            : 500,
      error: confirmed.message,
    };
  }

  return {
    ok: true,
    pending: false,
    booking: confirmed.booking,
    mailWarning: confirmed.mailWarning,
  };
}

type BookingWithEtab = RdvInscriptionBookingRow & { etablissementId: string };

/**
 * Finalise une réservation pending : agenda Google, statut confirmed, mails récap + ICS.
 * Utilisé à la réservation publique, via lien legacy, ou confirmation admin.
 */
async function finalizeRdvInscriptionBookingConfirmation(
  found: BookingWithEtab,
): Promise<
  | { ok: true; booking: RdvInscriptionBookingRow; already?: boolean; mailWarning?: string }
  | { ok: false; error: "invalid" | "expired" | "taken" | "error"; message: string }
> {
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

  let eleveId = found.eleveId;
  let matchStatus: "confirmed" | "created" = found.eleveId ? "confirmed" : "created";

  if (found.createNew || !eleveId) {
    try {
      eleveId = await createPreinscritFromRdvBooking({
        etablissementId: found.etablissementId,
        nom: found.studentLastName,
        prenom: found.studentFirstName,
        parentEmail: found.parentEmail,
        parentPhone: found.parentPhone,
        parentFirstName: found.parentFirstName,
        parentLastName: found.parentLastName,
        niveauLabel: found.niveauLabel,
        directionSlug: found.directionSlug,
      });
      matchStatus = "created";
    } catch (e) {
      console.error("[rdv-inscription] création préinscrit:", e);
      return {
        ok: false,
        error: "error",
        message: "Impossible de créer le dossier élève.",
      };
    }
  } else {
    const ok = await assertEleveStillMatchesRdvBooking({
      etablissementId: found.etablissementId,
      eleveId,
      parentEmail: found.parentEmail,
      studentFirstName: found.studentFirstName,
      studentLastName: found.studentLastName,
    });
    if (!ok) {
      return {
        ok: false,
        error: "error",
        message: "Élève non rattaché à ces coordonnées.",
      };
    }
    matchStatus = "confirmed";
  }

  const dossierInscriptionUrl = await tenantAbsolutePath(
    `/eleves/dossier/${encodeURIComponent(eleveId)}/inscription`,
  );
  const reconfirmToken = randomBytes(32).toString("hex");

  const gcal = await confirmInscriptionCalendarEvent({
    calendarId: found.googleCalendarId,
    eventId: found.googleEventId,
    bookingId: found.id,
    studentFirstName: found.studentFirstName,
    studentLastName: found.studentLastName,
    parentEmail: found.parentEmail,
    parentPhone: found.parentPhone,
    parentFirstName: found.parentFirstName,
    parentLastName: found.parentLastName,
    rdvAttendee: found.rdvAttendee,
    niveauLabel: found.niveauLabel,
    dossierInscriptionUrl,
    hasPap: found.hasPap,
    papBringToRdv: found.papBringToRdv,
    papUploaded: Boolean(found.papS3Key),
    etablissementOrigineLabel: found.etablissementOrigineLabel,
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
    eleveId,
    matchStatus,
    reconfirmToken,
  });
  if (!booking) {
    return { ok: false, error: "error", message: "Confirmation impossible." };
  }

  try {
    const { attachRdvBookingExtrasToEleve } = await import(
      "@/app/lib/rdv-inscription-eleve-extras"
    );
    await attachRdvBookingExtrasToEleve({
      etablissementId: found.etablissementId,
      eleveId,
      booking: found,
    });
  } catch (e) {
    console.error("[rdv-inscription] attach extras:", e);
  }

  if (matchStatus === "created" && direction.notifyEmail) {
    await sendRdvInscriptionCreatedPreinscritNotify({
      page: directionPageSettings(direction),
      booking,
      directionLabel: direction.label,
      dossierUrl: dossierInscriptionUrl,
    });
  }

  const mail = await sendRdvInscriptionConfirmationMails({
    page: directionPageSettings(direction),
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

/** Lien e-mail legacy (anciennes réservations pending). */
export async function confirmPublicRdvInscription(token: string): Promise<
  | { ok: true; booking: RdvInscriptionBookingRow; already?: boolean; mailWarning?: string }
  | { ok: false; error: "invalid" | "expired" | "taken" | "error"; message: string }
> {
  const found = await findBookingByConfirmToken(token);
  if (!found) {
    return { ok: false, error: "invalid", message: "Lien de validation invalide ou déjà utilisé." };
  }
  return finalizeRdvInscriptionBookingConfirmation(found);
}

/** Confirmation manuelle depuis l’admin (dégager un « En attente mail »). */
export async function confirmRdvInscriptionBookingAsAdmin(bookingId: string): Promise<
  | { ok: true; booking: RdvInscriptionBookingRow; already?: boolean; mailWarning?: string }
  | { ok: false; status: number; error: string }
> {
  const found = await findRdvInscriptionBookingById({ bookingId });
  if (!found) {
    return { ok: false, status: 404, error: "Réservation introuvable." };
  }
  if (found.status === "confirmed") {
    return { ok: true, booking: found, already: true };
  }
  if (found.status !== "pending") {
    return {
      ok: false,
      status: 400,
      error: "Seules les réservations « En attente mail » peuvent être confirmées manuellement.",
    };
  }

  // Contourne l’expiration du lien : la direction valide explicitement.
  const result = await finalizeRdvInscriptionBookingConfirmation({
    ...found,
    confirmExpiresAt: null,
  });
  if (!result.ok) {
    const status =
      result.error === "taken" ? 409 : result.error === "expired" ? 410 : 500;
    return { ok: false, status, error: result.message };
  }
  return {
    ok: true,
    booking: result.booking,
    already: result.already,
    mailWarning: result.mailWarning,
  };
}

export async function reconfirmPublicRdvInscription(opts: {
  token: string;
  action: "ok" | "cancel";
}): Promise<
  | { ok: true; booking: RdvInscriptionBookingRow; action: "ok" | "cancel" }
  | { ok: false; error: string; message: string }
> {
  const found = await findBookingByReconfirmToken(opts.token);
  if (!found || found.status !== "confirmed") {
    return {
      ok: false,
      error: "invalid",
      message: "Lien de reconfirmation invalide.",
    };
  }
  if (found.reconfirmStatus === "ok" || found.reconfirmStatus === "cancelled") {
    return { ok: true, booking: found, action: found.reconfirmStatus === "ok" ? "ok" : "cancel" };
  }

  const now = new Date();
  if (opts.action === "ok") {
    const booking = await markRdvInscriptionReconfirm({
      bookingId: found.id,
      etablissementId: found.etablissementId,
      status: "ok",
    });
    if (!booking) {
      return { ok: false, error: "error", message: "Mise à jour impossible." };
    }
    try {
      await markParentReconfirmedOnCalendar({
        calendarId: found.googleCalendarId,
        eventId: found.googleEventId,
        bookingId: found.id,
        reconfirmedAt: now,
      });
    } catch (e) {
      console.error("[rdv-inscription] reconfirm GCal:", e);
    }
    return { ok: true, booking, action: "ok" };
  }

  try {
    await cancelConfirmedInscriptionEvent({
      calendarId: found.googleCalendarId,
      eventId: found.googleEventId,
      bookingId: found.id,
    });
  } catch (e) {
    console.error("[rdv-inscription] cancel GCal:", e);
  }

  const booking = await markRdvInscriptionReconfirm({
    bookingId: found.id,
    etablissementId: found.etablissementId,
    status: "cancelled",
  });
  if (!booking) {
    return { ok: false, error: "error", message: "Annulation impossible." };
  }

  const direction = await getRdvInscriptionDirectionBySlug(found.directionSlug, {
    etablissementId: found.etablissementId,
  });
  if (direction?.notifyEmail) {
    const { sendRdvInscriptionCancelledByParentNotify } = await import(
      "@/app/lib/rdv-inscription-mail"
    );
    await sendRdvInscriptionCancelledByParentNotify({
      page: directionPageSettings(direction),
      booking,
      directionLabel: direction.label,
    });
  }

  return { ok: true, booking, action: "cancel" };
}

/** Cron : envoie les mails J-7 « toujours OK ? ». Silence parent = RDV conservé. */
export async function processRdvInscriptionReconfirmMails(opts?: {
  etablissementId?: string;
}): Promise<{ sent: number; errors: string[] }> {
  const due = await listBookingsDueForReconfirmMail({
    etablissementId: opts?.etablissementId,
    limit: 40,
  });
  let sent = 0;
  const errors: string[] = [];

  for (const b of due) {
    const direction = await getRdvInscriptionDirectionBySlug(b.directionSlug, {
      etablissementId: b.etablissementId,
    });
    if (!direction) {
      errors.push(`${b.id}: direction introuvable`);
      continue;
    }
    const okUrl = await tenantAbsolutePath(
      `/rdv-inscription/reconfirm?token=${encodeURIComponent(b.reconfirmToken)}&action=ok`,
    );
    const cancelUrl = await tenantAbsolutePath(
      `/rdv-inscription/reconfirm?token=${encodeURIComponent(b.reconfirmToken)}&action=cancel`,
    );
    const mail = await sendRdvInscriptionReconfirmMail({
      page: directionPageSettings(direction),
      booking: b,
      directionLabel: direction.label,
      directriceName: direction.directriceDisplayName,
      okUrl,
      cancelUrl,
    });
    if (!mail.sent) {
      errors.push(`${b.id}: ${mail.error || "envoi échoué"}`);
      continue;
    }
    await markRdvInscriptionReconfirmMailSent({
      bookingId: b.id,
      etablissementId: b.etablissementId,
    });
    sent += 1;
  }

  return { sent, errors };
}
