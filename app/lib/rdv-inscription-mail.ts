import "server-only";

import { buildCalendarEventIcs } from "@/app/lib/calendar-ics";
import { escapeHtml } from "@/app/lib/escape-html";
import { buildRdvInscriptionIcsLocation } from "@/app/lib/rdv-inscription-contact";
import { buildRdvInscriptionGcalSummary, formatRdvAttendeeLabel } from "@/app/lib/rdv-inscription-gcal-format";
import type {
  RdvInscriptionBookingRow,
  RdvInscriptionDirectionPageSettings,
} from "@/app/lib/rdv-inscription-types";
import { RDV_RESCHEDULE_PRESET_MOTIF } from "@/app/lib/rdv-inscription-types";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";

function parentDisplayName(booking: RdvInscriptionBookingRow): string {
  return [booking.parentFirstName, booking.parentLastName].filter(Boolean).join(" ");
}

function formatSlotFr(startAt: string, endAt: string): string {
  const start = new Date(startAt);
  const end = new Date(endAt);
  const day = start.toLocaleDateString("fr-FR", {
    timeZone: "Europe/Paris",
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
  const hm = (d: Date) =>
    d.toLocaleTimeString("fr-FR", {
      timeZone: "Europe/Paris",
      hour: "2-digit",
      minute: "2-digit",
    });
  return `${day} · ${hm(start)} – ${hm(end)}`;
}

/** Mail « cliquez pour valider » (double opt-in anti-spam). */
export async function sendRdvInscriptionValidationMail(opts: {
  page: RdvInscriptionDirectionPageSettings;
  booking: RdvInscriptionBookingRow;
  directionLabel: string;
  directriceName?: string | null;
  confirmUrl: string;
  expiresAt: Date;
}): Promise<{ sent: boolean; error?: string }> {
  const smtp = await getTenantSmtpConfig();
  const transporter = await createTenantTransporter();
  if (!smtp || !transporter) {
    return {
      sent: false,
      error: "SMTP non configuré — impossible d’envoyer le lien de validation.",
    };
  }

  const slotLabel = formatSlotFr(opts.booking.startAt, opts.booking.endAt);
  const student = `${opts.booking.studentFirstName} ${opts.booking.studentLastName}`;
  const title = `${opts.page.title} — ${opts.directionLabel}`;
  const expiresLabel = opts.expiresAt.toLocaleString("fr-FR", {
    timeZone: "Europe/Paris",
    day: "numeric",
    month: "long",
    hour: "2-digit",
    minute: "2-digit",
  });

  try {
    await transporter.sendMail({
      from: smtp.user,
      to: opts.booking.parentEmail,
      subject: `Validez votre créneau — ${title}`,
      html: `
        <p>Bonjour,</p>
        <p>Vous avez demandé un rendez-vous d’inscription. Pour confirmer le créneau
        (et éviter les réservations abusives), cliquez sur le bouton ci-dessous :</p>
        <p style="margin:24px 0;">
          <a href="${escapeHtml(opts.confirmUrl)}"
             style="display:inline-block;background:#0369a1;color:#fff;padding:12px 20px;border-radius:8px;text-decoration:none;font-weight:700;">
            Valider mon créneau
          </a>
        </p>
        <p><strong>Élève :</strong> ${escapeHtml(student)}<br/>
        <strong>Direction :</strong> ${escapeHtml(opts.directionLabel)}<br/>
        <strong>Créneau :</strong> ${escapeHtml(slotLabel)}
        ${opts.directriceName ? `<br/><strong>Avec :</strong> ${escapeHtml(opts.directriceName)}` : ""}
        </p>
        <p style="color:#64748b;font-size:13px;">Ce lien expire le ${escapeHtml(expiresLabel)}.
        Si vous n’êtes pas à l’origine de cette demande, ignorez cet e-mail.</p>
        <p style="color:#94a3b8;font-size:12px;">Lien : ${escapeHtml(opts.confirmUrl)}</p>
      `,
    });
    return { sent: true };
  } catch (e) {
    return {
      sent: false,
      error: e instanceof Error ? e.message : String(e),
    };
  }
}

/** Mail de confirmation finale (+ ICS) après clic sur le lien. */
export async function sendRdvInscriptionConfirmationMails(opts: {
  page: RdvInscriptionDirectionPageSettings;
  booking: RdvInscriptionBookingRow;
  directionLabel: string;
  directriceName?: string | null;
}): Promise<{ parentSent: boolean; notifySent: boolean; error?: string }> {
  const smtp = await getTenantSmtpConfig();
  const transporter = await createTenantTransporter();
  if (!smtp || !transporter) {
    return {
      parentSent: false,
      notifySent: false,
      error: "SMTP non configuré — réservation confirmée sans e-mail.",
    };
  }

  const slotLabel = formatSlotFr(opts.booking.startAt, opts.booking.endAt);
  const student = `${opts.booking.studentFirstName} ${opts.booking.studentLastName}`;
  const location = opts.page.location.trim();
  const icsLocation = buildRdvInscriptionIcsLocation(location);
  const parentName = parentDisplayName(opts.booking);
  const presentLabel = formatRdvAttendeeLabel(opts.booking.rdvAttendee);
  const mailTitle = `${opts.page.title} — ${opts.directionLabel}`;
  const gcalTitle = buildRdvInscriptionGcalSummary({
    studentLastName: opts.booking.studentLastName,
    studentFirstName: opts.booking.studentFirstName,
    niveauLabel: opts.booking.niveauLabel,
    regime: opts.booking.regime,
  });

  const ics = buildCalendarEventIcs({
    title: gcalTitle,
    description: [
      `Rendez-vous d’inscription (${opts.directionLabel}).`,
      opts.directriceName ? `Avec : ${opts.directriceName}` : "",
      `Élève : ${student}`,
      opts.booking.niveauLabel ? `Niveau demandé : ${opts.booking.niveauLabel}` : "",
      opts.booking.regime ? `Régime : ${opts.booking.regime}` : "",
      parentName ? `Parent : ${parentName}` : "",
      presentLabel ? `Présent au RDV : ${presentLabel}` : "",
      // Téléphone : uniquement dans LOCATION (icsLocation), pas dans la description.
    ]
      .filter(Boolean)
      .join("\n"),
    location: icsLocation,
    startAt: opts.booking.startAt,
    endAt: opts.booking.endAt,
    uid: `rdv-inscription-${opts.booking.id}@scola`,
    prodId: "-//Scola//RDV inscription//FR",
    alarms: [
      {
        trigger: "-P7D",
        description: `Rappel RDV inscription — ${student}`,
      },
    ],
  });

  let parentSent = false;
  let notifySent = false;

  try {
    await transporter.sendMail({
      from: smtp.user,
      to: opts.booking.parentEmail,
      subject: `Réservation validée — ${mailTitle}`,
      html: `
        <p>Bonjour,</p>
        <p>Votre rendez-vous d’inscription est <strong>validé</strong>.</p>
        <p><strong>Élève :</strong> ${escapeHtml(student)}<br/>
        <strong>Direction :</strong> ${escapeHtml(opts.directionLabel)}<br/>
        <strong>Créneau :</strong> ${escapeHtml(slotLabel)}
        <br/><strong>Lieu :</strong> ${escapeHtml(icsLocation)}
        ${opts.directriceName ? `<br/><strong>Avec :</strong> ${escapeHtml(opts.directriceName)}` : ""}
        </p>
        <p>Un fichier calendrier (.ics) est joint à cet e-mail.</p>
        <p>En cas de question sur le rendez-vous, ou si vous souhaitez le modifier voire l’annuler, contactez l’établissement.</p>
        <p>Cordialement,<br/>L’établissement</p>
      `,
      attachments: [
        {
          filename: "rdv-inscription.ics",
          content: ics,
          contentType: "text/calendar",
        },
      ],
    });
    parentSent = true;
  } catch (e) {
    return {
      parentSent: false,
      notifySent: false,
      error: e instanceof Error ? e.message : String(e),
    };
  }

  const notify = opts.page.notifyEmail?.trim();
  if (notify) {
    try {
      await transporter.sendMail({
        from: smtp.user,
        to: notify,
        subject: `[RDV inscription] ${student} — ${opts.directionLabel}`,
        html: `
          <p>Nouvelle prise de rendez-vous d’inscription (validée par le parent).</p>
          <ul>
            <li><strong>Élève :</strong> ${escapeHtml(student)}</li>
            <li><strong>Direction :</strong> ${escapeHtml(opts.directionLabel)}</li>
            <li><strong>Créneau :</strong> ${escapeHtml(slotLabel)}</li>
            <li><strong>E-mail :</strong> ${escapeHtml(opts.booking.parentEmail)}</li>
            <li><strong>Tél. :</strong> ${escapeHtml(opts.booking.parentPhone)}</li>
            ${
              parentDisplayName(opts.booking)
                ? `<li><strong>Parent :</strong> ${escapeHtml(parentDisplayName(opts.booking))}</li>`
                : ""
            }
            ${
              formatRdvAttendeeLabel(opts.booking.rdvAttendee)
                ? `<li><strong>Présent au RDV :</strong> ${escapeHtml(formatRdvAttendeeLabel(opts.booking.rdvAttendee))}</li>`
                : ""
            }
            ${
              opts.booking.niveauLabel
                ? `<li><strong>Niveau demandé :</strong> ${escapeHtml(opts.booking.niveauLabel)}</li>`
                : ""
            }
            ${
              opts.booking.regime
                ? `<li><strong>Régime :</strong> ${escapeHtml(opts.booking.regime)}</li>`
                : ""
            }
            ${
              opts.booking.eleveId
                ? `<li><strong>Dossier :</strong> /eleves/dossier/${escapeHtml(opts.booking.eleveId)}/inscription</li>`
                : ""
            }
          </ul>
          ${
            opts.booking.googleHtmlLink
              ? `<p><a href="${escapeHtml(opts.booking.googleHtmlLink)}">Ouvrir dans Google Agenda</a></p>`
              : ""
          }
        `,
      });
      notifySent = true;
    } catch {
      /* notif secrétariat non bloquante */
    }
  }

  return { parentSent, notifySent };
}

/** Relance J-7 — le silence ne supprime pas le RDV. */
export async function sendRdvInscriptionReconfirmMail(opts: {
  page: RdvInscriptionDirectionPageSettings;
  booking: RdvInscriptionBookingRow;
  directionLabel: string;
  directriceName?: string | null;
  okUrl: string;
  cancelUrl: string;
}): Promise<{ sent: boolean; error?: string }> {
  const smtp = await getTenantSmtpConfig();
  const transporter = await createTenantTransporter();
  if (!smtp || !transporter) {
    return { sent: false, error: "SMTP non configuré." };
  }

  const slotLabel = formatSlotFr(opts.booking.startAt, opts.booking.endAt);
  const student = `${opts.booking.studentFirstName} ${opts.booking.studentLastName}`;
  const title = `${opts.page.title} — ${opts.directionLabel}`;

  try {
    await transporter.sendMail({
      from: smtp.user,
      to: opts.booking.parentEmail,
      subject: `Toujours disponible ? — ${title}`,
      html: `
        <p>Bonjour,</p>
        <p>Votre rendez-vous d’inscription approche :</p>
        <p><strong>Élève :</strong> ${escapeHtml(student)}<br/>
        <strong>Créneau :</strong> ${escapeHtml(slotLabel)}
        ${opts.directriceName ? `<br/><strong>Avec :</strong> ${escapeHtml(opts.directriceName)}` : ""}
        </p>
        <p>Êtes-vous toujours disponible ?</p>
        <p style="margin:24px 0;">
          <a href="${escapeHtml(opts.okUrl)}"
             style="display:inline-block;background:#0369a1;color:#fff;padding:12px 20px;border-radius:8px;text-decoration:none;font-weight:700;margin-right:12px;">
            Oui, je confirme
          </a>
          <a href="${escapeHtml(opts.cancelUrl)}"
             style="display:inline-block;background:#fff;color:#b91c1c;padding:12px 20px;border-radius:8px;text-decoration:none;font-weight:700;border:1px solid #fecaca;">
            Annuler le rendez-vous
          </a>
        </p>
        <p style="color:#64748b;font-size:13px;">Si vous ne répondez pas, le rendez-vous reste maintenu.</p>
      `,
    });
    return { sent: true };
  } catch (e) {
    return { sent: false, error: e instanceof Error ? e.message : String(e) };
  }
}

export async function sendRdvInscriptionCreatedPreinscritNotify(opts: {
  page: RdvInscriptionDirectionPageSettings;
  booking: RdvInscriptionBookingRow;
  directionLabel: string;
  dossierUrl: string;
}): Promise<{ sent: boolean }> {
  const smtp = await getTenantSmtpConfig();
  const transporter = await createTenantTransporter();
  const to = opts.page.notifyEmail?.trim();
  if (!smtp || !transporter || !to) return { sent: false };

  const student = `${opts.booking.studentFirstName} ${opts.booking.studentLastName}`;
  try {
    await transporter.sendMail({
      from: smtp.user,
      to,
      subject: `[RDV] Nouveau préinscrit créé — ${student}`,
      html: `
        <p>Un dossier <strong>préinscrit</strong> a été créé automatiquement depuis un RDV d’inscription
        (aucune fiche élève trouvée pour ces coordonnées parent).</p>
        <ul>
          <li><strong>Élève :</strong> ${escapeHtml(student)}</li>
          <li><strong>Niveau demandé :</strong> ${escapeHtml(opts.booking.niveauLabel || "—")}</li>
          <li><strong>E-mail :</strong> ${escapeHtml(opts.booking.parentEmail)}</li>
          <li><strong>Tél. :</strong> ${escapeHtml(opts.booking.parentPhone)}</li>
          ${
            parentDisplayName(opts.booking)
              ? `<li><strong>Parent :</strong> ${escapeHtml(parentDisplayName(opts.booking))}</li>`
              : ""
          }
        </ul>
        <p><a href="${escapeHtml(opts.dossierUrl)}">Ouvrir les documents d’inscription</a></p>
      `,
    });
    return { sent: true };
  } catch {
    return { sent: false };
  }
}

export async function sendRdvInscriptionCancelledByParentNotify(opts: {
  page: RdvInscriptionDirectionPageSettings;
  booking: RdvInscriptionBookingRow;
  directionLabel: string;
}): Promise<{ sent: boolean }> {
  const smtp = await getTenantSmtpConfig();
  const transporter = await createTenantTransporter();
  const to = opts.page.notifyEmail?.trim();
  if (!smtp || !transporter || !to) return { sent: false };

  const student = `${opts.booking.studentFirstName} ${opts.booking.studentLastName}`;
  const slotLabel = formatSlotFr(opts.booking.startAt, opts.booking.endAt);
  try {
    await transporter.sendMail({
      from: smtp.user,
      to,
      subject: `[RDV] Annulation parent — ${student}`,
      html: `
        <p>Le parent a annulé le rendez-vous d’inscription (reconfirmation J-7).</p>
        <ul>
          <li><strong>Élève :</strong> ${escapeHtml(student)}</li>
          <li><strong>Direction :</strong> ${escapeHtml(opts.directionLabel)}</li>
          <li><strong>Créneau :</strong> ${escapeHtml(slotLabel)}</li>
          <li><strong>E-mail :</strong> ${escapeHtml(opts.booking.parentEmail)}</li>
        </ul>
      `,
    });
    return { sent: true };
  } catch {
    return { sent: false };
  }
}

/** Mail parent : créneau retiré par l’établissement + invitation à en choisir un autre. */
export async function sendRdvInscriptionRescheduleRequestMail(opts: {
  page: RdvInscriptionDirectionPageSettings;
  booking: RdvInscriptionBookingRow;
  directionLabel: string;
  rebookUrl: string;
  adminNote?: string | null;
}): Promise<{ sent: boolean; error?: string }> {
  const smtp = await getTenantSmtpConfig();
  const transporter = await createTenantTransporter();
  if (!smtp || !transporter) {
    return {
      sent: false,
      error: "SMTP non configuré — créneau retiré sans e-mail parent.",
    };
  }

  const student = `${opts.booking.studentFirstName} ${opts.booking.studentLastName}`;
  const cancelledSlot = formatSlotFr(opts.booking.startAt, opts.booking.endAt);
  const mailTitle = `${opts.page.title} — ${opts.directionLabel}`;
  const note = opts.adminNote?.trim() || "";
  const noteHtml = note
    ? `<p><strong>Précision de l’établissement :</strong> ${escapeHtml(note)}</p>`
    : "";

  try {
    await transporter.sendMail({
      from: smtp.user,
      to: opts.booking.parentEmail,
      subject: `Nouveau créneau à choisir — ${mailTitle}`,
      html: `
        <p>Bonjour,</p>
        <p>${escapeHtml(RDV_RESCHEDULE_PRESET_MOTIF)}</p>
        ${noteHtml}
        <p><strong>Élève :</strong> ${escapeHtml(student)}<br/>
        <strong>Direction :</strong> ${escapeHtml(opts.directionLabel)}<br/>
        <strong>Créneau précédent :</strong> ${escapeHtml(cancelledSlot)}</p>
        <p>Ce créneau n’est plus disponible. Merci de <strong>choisir un autre créneau</strong> via le bouton ci-dessous (lien valable 14 jours) :</p>
        <p style="margin:24px 0">
          <a href="${escapeHtml(opts.rebookUrl)}"
             style="display:inline-block;background:#0369a1;color:#fff;padding:12px 20px;border-radius:8px;text-decoration:none;font-weight:700">
            Choisir un autre créneau
          </a>
        </p>
        <p style="font-size:12px;color:#64748b">Si le bouton ne fonctionne pas, copiez ce lien :<br/>
          <a href="${escapeHtml(opts.rebookUrl)}">${escapeHtml(opts.rebookUrl)}</a>
        </p>
        <p>Cordialement,<br/>L’établissement</p>
      `,
    });
    return { sent: true };
  } catch (e) {
    return {
      sent: false,
      error: e instanceof Error ? e.message : String(e),
    };
  }
}

/** Mail parent : un créneau a été annulé par l’établissement + rappel des RDV encore actifs. */
export async function sendRdvInscriptionCancelledByAdminMail(opts: {
  page: RdvInscriptionDirectionPageSettings;
  cancelled: RdvInscriptionBookingRow;
  directionLabel: string;
  remaining: RdvInscriptionBookingRow[];
  remainingDirectionLabels?: Record<string, string>;
}): Promise<{ sent: boolean; error?: string }> {
  const smtp = await getTenantSmtpConfig();
  const transporter = await createTenantTransporter();
  if (!smtp || !transporter) {
    return {
      sent: false,
      error: "SMTP non configuré — créneau annulé sans e-mail parent.",
    };
  }

  const student = `${opts.cancelled.studentFirstName} ${opts.cancelled.studentLastName}`;
  const cancelledSlot = formatSlotFr(opts.cancelled.startAt, opts.cancelled.endAt);
  const mailTitle = `${opts.page.title} — ${opts.directionLabel}`;

  const remainingHtml =
    opts.remaining.length === 0
      ? `<p>Vous n’avez <strong>plus aucun</strong> rendez-vous d’inscription actif pour cet élève.</p>`
      : `<p><strong>Vos rendez-vous encore actifs :</strong></p>
        <ul>
          ${opts.remaining
            .map((b) => {
              const dir =
                opts.remainingDirectionLabels?.[b.directionSlug] || b.directionSlug;
              const slot = formatSlotFr(b.startAt, b.endAt);
              const niveau = b.niveauLabel ? ` · ${b.niveauLabel}` : "";
              return `<li><strong>${escapeHtml(dir)}</strong> — ${escapeHtml(slot)}${escapeHtml(niveau)}</li>`;
            })
            .join("")}
        </ul>`;

  try {
    await transporter.sendMail({
      from: smtp.user,
      to: opts.cancelled.parentEmail,
      subject: `Créneau annulé — ${mailTitle}`,
      html: `
        <p>Bonjour,</p>
        <p>Le créneau suivant a été <strong>supprimé</strong> par l’établissement :</p>
        <p><strong>Élève :</strong> ${escapeHtml(student)}<br/>
        <strong>Direction :</strong> ${escapeHtml(opts.directionLabel)}<br/>
        <strong>Créneau :</strong> ${escapeHtml(cancelledSlot)}</p>
        ${remainingHtml}
        <p>Si vous avez une question, contactez l’établissement.</p>
        <p>Cordialement,<br/>L’établissement</p>
      `,
    });
    return { sent: true };
  } catch (e) {
    return {
      sent: false,
      error: e instanceof Error ? e.message : String(e),
    };
  }
}

/** Mail parent : le créneau a été modifié par l’établissement (avec ICS à jour). */
export async function sendRdvInscriptionSlotChangedByAdminMail(opts: {
  page: RdvInscriptionDirectionPageSettings;
  booking: RdvInscriptionBookingRow;
  previousStartAt: string;
  previousEndAt: string;
  directionLabel: string;
  directriceName?: string | null;
  adminNote?: string | null;
}): Promise<{ sent: boolean; error?: string }> {
  const smtp = await getTenantSmtpConfig();
  const transporter = await createTenantTransporter();
  if (!smtp || !transporter) {
    return {
      sent: false,
      error: "SMTP non configuré — créneau modifié sans e-mail parent.",
    };
  }

  const student = `${opts.booking.studentFirstName} ${opts.booking.studentLastName}`;
  const previousSlot = formatSlotFr(opts.previousStartAt, opts.previousEndAt);
  const newSlot = formatSlotFr(opts.booking.startAt, opts.booking.endAt);
  const location = opts.page.location.trim();
  const icsLocation = buildRdvInscriptionIcsLocation(location);
  const parentName = parentDisplayName(opts.booking);
  const presentLabel = formatRdvAttendeeLabel(opts.booking.rdvAttendee);
  const mailTitle = `${opts.page.title} — ${opts.directionLabel}`;
  const gcalTitle = buildRdvInscriptionGcalSummary({
    studentLastName: opts.booking.studentLastName,
    studentFirstName: opts.booking.studentFirstName,
    niveauLabel: opts.booking.niveauLabel,
    regime: opts.booking.regime,
  });
  const note = opts.adminNote?.trim() || "";
  const noteHtml = note
    ? `<p><strong>Précision de l’établissement :</strong> ${escapeHtml(note)}</p>`
    : "";

  const ics = buildCalendarEventIcs({
    title: gcalTitle,
    description: [
      `Rendez-vous d’inscription (${opts.directionLabel}) — créneau modifié.`,
      opts.directriceName ? `Avec : ${opts.directriceName}` : "",
      `Élève : ${student}`,
      opts.booking.niveauLabel ? `Niveau demandé : ${opts.booking.niveauLabel}` : "",
      opts.booking.regime ? `Régime : ${opts.booking.regime}` : "",
      parentName ? `Parent : ${parentName}` : "",
      presentLabel ? `Présent au RDV : ${presentLabel}` : "",
    ]
      .filter(Boolean)
      .join("\n"),
    location: icsLocation,
    startAt: opts.booking.startAt,
    endAt: opts.booking.endAt,
    uid: `rdv-inscription-${opts.booking.id}@scola`,
    prodId: "-//Scola//RDV inscription//FR",
    alarms: [
      {
        trigger: "-P7D",
        description: `Rappel RDV inscription — ${student}`,
      },
    ],
  });

  try {
    await transporter.sendMail({
      from: smtp.user,
      to: opts.booking.parentEmail,
      subject: `Créneau modifié — ${mailTitle}`,
      html: `
        <p>Bonjour,</p>
        <p>Suite à un échange avec l’établissement, votre rendez-vous d’inscription a été
        <strong>modifié</strong>.</p>
        ${noteHtml}
        <p><strong>Élève :</strong> ${escapeHtml(student)}<br/>
        <strong>Direction :</strong> ${escapeHtml(opts.directionLabel)}<br/>
        <strong>Ancien créneau :</strong> ${escapeHtml(previousSlot)}<br/>
        <strong>Nouveau créneau :</strong> ${escapeHtml(newSlot)}
        <br/><strong>Lieu :</strong> ${escapeHtml(icsLocation)}
        ${opts.directriceName ? `<br/><strong>Avec :</strong> ${escapeHtml(opts.directriceName)}` : ""}
        </p>
        <p>Un fichier calendrier (.ics) à jour est joint à cet e-mail — remplacez l’ancien
        rendez-vous dans votre agenda si besoin.</p>
        <p>Cordialement,<br/>L’établissement</p>
      `,
      attachments: [
        {
          filename: "rdv-inscription.ics",
          content: ics,
          contentType: "text/calendar",
        },
      ],
    });
    return { sent: true };
  } catch (e) {
    return {
      sent: false,
      error: e instanceof Error ? e.message : String(e),
    };
  }
}
