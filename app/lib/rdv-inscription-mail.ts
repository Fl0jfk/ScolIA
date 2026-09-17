import "server-only";

import { buildCalendarEventIcs } from "@/app/lib/calendar-ics";
import { escapeHtml } from "@/app/lib/escape-html";
import type {
  RdvInscriptionBookingRow,
  RdvInscriptionDirectionPageSettings,
} from "@/app/lib/rdv-inscription-types";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";

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
      error: "SMTP non configuré — réservation enregistrée sans e-mail.",
    };
  }

  const slotLabel = formatSlotFr(opts.booking.startAt, opts.booking.endAt);
  const student = `${opts.booking.studentFirstName} ${opts.booking.studentLastName}`;
  const location = opts.page.location.trim();
  const title = `${opts.page.title} — ${opts.directionLabel}`;

  const ics = buildCalendarEventIcs({
    title: `${title} — ${student}`,
    description: [
      `Rendez-vous d’inscription (${opts.directionLabel}).`,
      opts.directriceName ? `Avec : ${opts.directriceName}` : "",
      `Élève : ${student}`,
      `Téléphone : ${opts.booking.parentPhone}`,
      location ? `Lieu : ${location}` : "",
    ]
      .filter(Boolean)
      .join("\n"),
    location: location || undefined,
    startAt: opts.booking.startAt,
    endAt: opts.booking.endAt,
    uid: `rdv-inscription-${opts.booking.id}@scola`,
    prodId: "-//Scola//RDV inscription//FR",
  });

  let parentSent = false;
  let notifySent = false;

  try {
    await transporter.sendMail({
      from: smtp.user,
      to: opts.booking.parentEmail,
      subject: `Confirmation — ${title}`,
      html: `
        <p>Bonjour,</p>
        <p>Votre rendez-vous d’inscription est confirmé.</p>
        <p><strong>Élève :</strong> ${escapeHtml(student)}<br/>
        <strong>Direction :</strong> ${escapeHtml(opts.directionLabel)}<br/>
        <strong>Créneau :</strong> ${escapeHtml(slotLabel)}
        ${location ? `<br/><strong>Lieu :</strong> ${escapeHtml(location)}` : ""}
        ${opts.directriceName ? `<br/><strong>Avec :</strong> ${escapeHtml(opts.directriceName)}` : ""}
        </p>
        <p>Un fichier calendrier (.ics) est joint à cet e-mail.</p>
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
          <p>Nouvelle prise de rendez-vous d’inscription.</p>
          <ul>
            <li><strong>Élève :</strong> ${escapeHtml(student)}</li>
            <li><strong>Direction :</strong> ${escapeHtml(opts.directionLabel)}</li>
            <li><strong>Créneau :</strong> ${escapeHtml(slotLabel)}</li>
            <li><strong>E-mail :</strong> ${escapeHtml(opts.booking.parentEmail)}</li>
            <li><strong>Tél. :</strong> ${escapeHtml(opts.booking.parentPhone)}</li>
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
