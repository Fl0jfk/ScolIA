import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { buildCalendarEventIcs } from "@/app/lib/calendar-ics";
import {
  formatInvitationWhen,
  invitationDiplomaDisplay,
} from "@/app/lib/invitation-db";
import {
  formatInvitationEleveLabel,
  type InvitationPageRecord,
  type InvitationRsvpRecord,
} from "@/app/lib/invitation-types";
import { createTenantTransporter, getTenantSmtpConfig, sendMailWithTimeout } from "@/app/lib/tenant-mail";

async function sendInvitationMail(params: {
  to: string;
  subject: string;
  html: string;
  ics?: string;
}): Promise<boolean> {
  try {
    const smtp = await getTenantSmtpConfig();
    const transporter = await createTenantTransporter();
    if (!smtp || !transporter) return false;
    const bundle = await loadAppConfig();
    const school = bundle.identity.shortName || bundle.identity.name;
    await sendMailWithTimeout(transporter, {
      from: `"${school}" <${smtp.user}>`,
      to: params.to,
      subject: params.subject,
      html: params.html,
      attachments: params.ics
        ? [
            {
              filename: "invitation.ics",
              content: params.ics,
              contentType: "text/calendar",
            },
          ]
        : undefined,
    });
    return true;
  } catch (e) {
    console.error("[invitation] envoi mail échoué:", e instanceof Error ? e.message : e);
    return false;
  }
}

function buildIcs(page: InvitationPageRecord, schoolName: string): string | undefined {
  if (!page.startsAt) return undefined;
  const endAt =
    page.endsAt ||
    new Date(new Date(page.startsAt).getTime() + 2 * 60 * 60 * 1000).toISOString();
  return buildCalendarEventIcs({
    title: page.title,
    description: page.intro || `${page.title} — ${schoolName}`,
    location: page.location || undefined,
    startAt: page.startsAt,
    endAt,
    uid: `invitation-${page.id}@scolia`,
    prodId: "-//Scola//Invitation ceremonie//FR",
    alarms: [{ trigger: "-P1D", description: `Rappel : ${page.title}` }],
  });
}

export async function sendInvitationRsvpConfirmation(params: {
  page: InvitationPageRecord;
  rsvp: InvitationRsvpRecord;
}): Promise<{ mailSent: boolean }> {
  const { page, rsvp } = params;
  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name || "Établissement";
  const when = formatInvitationWhen(page);
  const diploma = invitationDiplomaDisplay(page, rsvp.diploma);
  const eleve = formatInvitationEleveLabel(rsvp.eleveFirstName, rsvp.eleveLastName);

  if (rsvp.response === "non") {
    const html = `
      <p>Bonjour,</p>
      <p>Nous avons bien enregistré votre réponse pour <strong>${escapeHtml(page.title)}</strong>.</p>
      <p>
        Élève : <strong>${escapeHtml(eleve)}</strong><br/>
        Réponse : <strong>Je ne pourrai pas venir</strong>
        ${diploma ? `<br/>Diplôme : ${escapeHtml(diploma)}` : ""}
      </p>
      <p>Cordialement,<br/>${escapeHtml(school)}</p>
    `;
    const mailSent = await sendInvitationMail({
      to: rsvp.parentEmail,
      subject: `${page.title} — réponse enregistrée`,
      html,
    });
    if (page.notifyEmail) {
      await sendInvitationMail({
        to: page.notifyEmail,
        subject: `[Invitation] Non — ${eleve} — ${page.title}`,
        html: `<p>Nouvelle réponse <strong>Non</strong> pour ${escapeHtml(eleve)} (${escapeHtml(rsvp.parentEmail)}).</p>`,
      });
    }
    return { mailSent };
  }

  const ics = buildIcs(page, school);
  const html = `
    <p>Bonjour,</p>
    <p>Votre présence est bien enregistrée pour <strong>${escapeHtml(page.title)}</strong>.</p>
    <p>
      Élève : <strong>${escapeHtml(eleve)}</strong><br/>
      Personnes présentes : <strong>${rsvp.presentCount}</strong>
      ${diploma ? `<br/>Diplôme : ${escapeHtml(diploma)}` : ""}
      ${when ? `<br/>Quand : ${escapeHtml(when)}` : ""}
      ${page.location ? `<br/>Où : ${escapeHtml(page.location)}` : ""}
    </p>
    ${
      ics
        ? `<p>Un fichier calendrier (<strong>.ics</strong>) est joint à cet e-mail.</p>`
        : ""
    }
    <p>Cordialement,<br/>${escapeHtml(school)}</p>
  `;
  const mailSent = await sendInvitationMail({
    to: rsvp.parentEmail,
    subject: `${page.title} — confirmation de présence`,
    html,
    ics,
  });
  if (page.notifyEmail) {
    await sendInvitationMail({
      to: page.notifyEmail,
      subject: `[Invitation] Oui (${rsvp.presentCount}) — ${eleve} — ${page.title}`,
      html: `<p>Nouvelle réponse <strong>Oui</strong> (${rsvp.presentCount} pers.) pour ${escapeHtml(eleve)} (${escapeHtml(rsvp.parentEmail)}).</p>`,
    });
  }
  return { mailSent };
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
