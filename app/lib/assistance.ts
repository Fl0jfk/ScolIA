import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { PLATFORM_ASSISTANCE_EMAIL } from "@/app/lib/platform-assistance-email";
import { getDashboardCategories } from "@/app/lib/intranet-modules";
import {
  assertEligibleRequestAttachment,
  MAX_REQUEST_ATTACHMENTS_PER_UPLOAD,
} from "@/app/lib/requests";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";

const ASSISTANCE_DASHBOARD_SCOPE = "dashboard";

type AssistanceScopeOption = {
  id: string;
  label: string;
};

/** Destinataire des tickets assistance (config tenant puis env). */
async function getAssistanceTargetEmail(): Promise<string> {
  try {
    const config = await loadAppConfig();
    const fromSite = config.identity.assistanceEmail?.trim();
    if (fromSite) return fromSite;
  } catch {
    /* fallback plateforme */
  }
  return PLATFORM_ASSISTANCE_EMAIL;
}

export function getAssistanceScopeOptions(): AssistanceScopeOption[] {
  const modules = getDashboardCategories()
    .filter((c) => c.moduleId !== "assistance" && !c.orgAdminOnly)
    .map((c) => ({ id: c.moduleId, label: c.name }));

  return [{ id: ASSISTANCE_DASHBOARD_SCOPE, label: "Tableau de bord" }, ...modules];
}

export function assistanceScopeLabel(scopeId: string): string {
  if (scopeId === ASSISTANCE_DASHBOARD_SCOPE) return "Tableau de bord";
  return getAssistanceScopeOptions().find((o) => o.id === scopeId)?.label || scopeId;
}

export function isValidAssistanceScope(scopeId: string): boolean {
  return getAssistanceScopeOptions().some((o) => o.id === scopeId);
}

export function newAssistanceTicketId(): string {
  return `AST-${Date.now().toString(36).toUpperCase()}-${Math.random().toString(36).slice(2, 6).toUpperCase()}`;
}

export { MAX_REQUEST_ATTACHMENTS_PER_UPLOAD, assertEligibleRequestAttachment };

export async function sendAssistanceTicketEmails(input: {
  ticketId: string;
  scopeId: string;
  scopeLabel: string;
  description: string;
  userName: string;
  userEmail: string;
  attachments: { filename: string; content: Buffer; contentType: string }[];
}): Promise<void> {
  const smtp = await getTenantSmtpConfig();
  if (!smtp) throw new Error("SMTP non configuré — impossible d'envoyer le ticket.");

  const transporter = await createTenantTransporter();
  if (!transporter) throw new Error("SMTP non configuré — impossible d'envoyer le ticket.");

  const to = await getAssistanceTargetEmail();
  if (!to) throw new Error("Email assistance non configuré.");
  const when = new Date().toLocaleString("fr-FR", { timeZone: "Europe/Paris" });
  const safeDesc = input.description
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/\n/g, "<br/>");

  const html = `
    <div style="font-family:Arial,sans-serif;line-height:1.55;color:#1e293b;max-width:640px">
      <h2 style="margin:0 0 12px;font-size:18px;color:#0f172a">Nouveau ticket assistance</h2>
      <p style="margin:0 0 8px"><strong>Référence :</strong> ${input.ticketId}</p>
      <p style="margin:0 0 8px"><strong>Zone concernée :</strong> ${input.scopeLabel}</p>
      <p style="margin:0 0 8px"><strong>Utilisateur :</strong> ${input.userName}</p>
      <p style="margin:0 0 8px"><strong>E-mail :</strong> <a href="mailto:${input.userEmail}">${input.userEmail}</a></p>
      <p style="margin:0 0 16px"><strong>Date :</strong> ${when}</p>
      <div style="padding:14px 16px;border-radius:12px;background:#f8fafc;border:1px solid #e2e8f0">
        <p style="margin:0 0 6px;font-size:12px;font-weight:bold;color:#64748b;text-transform:uppercase">Description</p>
        <p style="margin:0;font-size:14px">${safeDesc}</p>
      </div>
      ${
        input.attachments.length
          ? `<p style="margin:16px 0 0;font-size:13px;color:#475569">${input.attachments.length} pièce(s) jointe(s) dans cet e-mail.</p>`
          : ""
      }
    </div>
  `;

  const mailAttachments = input.attachments.map((a) => ({
    filename: a.filename,
    content: a.content,
    contentType: a.contentType,
  }));

  await transporter.sendMail({
    from: `"Assistance intranet" <${smtp.user}>`,
    to,
    replyTo: input.userEmail,
    subject: `[Assistance] ${input.scopeLabel} — ${input.ticketId}`,
    html,
    attachments: mailAttachments,
  });

  await transporter.sendMail({
    from: `"Assistance intranet" <${smtp.user}>`,
    to: input.userEmail,
    subject: `Confirmation — ticket ${input.ticketId}`,
    html: `
      <div style="font-family:Arial,sans-serif;line-height:1.55;color:#334155">
        <p>Bonjour ${input.userName},</p>
        <p>Votre demande d'assistance a bien été transmise (réf. <strong>${input.ticketId}</strong>).</p>
        <p><strong>Zone :</strong> ${input.scopeLabel}</p>
        <p>Nous reviendrons vers vous sur <strong>${input.userEmail}</strong>.</p>
        <p style="margin-top:24px;color:#64748b;font-size:13px">Merci,<br/>L'équipe technique</p>
      </div>
    `,
  });
}
