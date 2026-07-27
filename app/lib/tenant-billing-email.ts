import "server-only";

import { MARKETING } from "@/app/lib/marketing-site";
import { PLATFORM_ASSISTANCE_EMAIL } from "@/app/lib/platform-assistance-email";
import { platformAppOrigin } from "@/app/lib/platform-portal-url";
import { BILLING_GRACE_DAYS, type TenantBillingState } from "@/app/lib/tenant-billing-types";
import type { TenantConfig } from "@/app/lib/tenant-types";
import { createPlatformTransporter, getPlatformSmtpConfig } from "@/app/lib/tenant-mail";

function readBilling(tenant: TenantConfig): TenantBillingState {
  return tenant.billing || { status: "active" };
}

async function sendMail(opts: { to: string; subject: string; html: string; text: string }) {
  const smtp = getPlatformSmtpConfig();
  const transporter = createPlatformTransporter();
  if (!smtp || !transporter) {
    console.warn("[tenant-billing-email] SMTP non configuré — e-mail non envoyé:", opts.subject);
    return false;
  }
  await transporter.sendMail({
    from: `"${MARKETING.productName}" <${smtp.user}>`,
    to: opts.to,
    subject: opts.subject,
    html: opts.html,
    text: opts.text,
  });
  return true;
}

function adminEmail(tenant: TenantConfig): string | null {
  return readBilling(tenant).adminEmail?.trim() || null;
}

function masterBillingUrl(slug: string): string {
  return `${platformAppOrigin()}/plateforme/tenants/${slug}/billing`;
}

export async function emailPaymentFailedAdmin(
  tenant: TenantConfig,
  reason?: string,
): Promise<void> {
  const billing = readBilling(tenant);
  const to = adminEmail(tenant);
  if (!to) return;
  const grace = billing.graceEndsAt
    ? new Date(billing.graceEndsAt).toLocaleDateString("fr-FR")
    : `sous ${BILLING_GRACE_DAYS} jours`;

  await sendMail({
    to,
    subject: `[Scola] Échec de paiement — action requise`,
    text: `Bonjour,\n\nUn prélèvement Scola pour ${tenant.label} n'a pas abouti.\n${reason ? `Motif : ${reason}\n` : ""}\nVotre accès reste actif pendant la période de grâce (jusqu'au ${grace}). Merci de régulariser votre moyen de paiement.\n\nContact : ${MARKETING.contactEmail}`,
    html: `<p>Bonjour,</p>
<p>Un <strong>prélèvement Scola</strong> pour <strong>${tenant.label}</strong> n'a pas abouti.</p>
${reason ? `<p><strong>Motif :</strong> ${reason}</p>` : ""}
<p>Votre accès reste actif pendant la période de grâce (jusqu'au <strong>${grace}</strong>). Merci de régulariser votre situation.</p>
<p>Contact : ${MARKETING.contactEmail}</p>`,
  });
}

export async function emailPaymentFailedMaster(
  tenant: TenantConfig,
  reason?: string,
): Promise<void> {
  const billing = readBilling(tenant);
  await sendMail({
    to: PLATFORM_ASSISTANCE_EMAIL,
    subject: `[Scola] Échec paiement — ${tenant.label}`,
    text: `Échec de paiement pour ${tenant.label} (${tenant.slug}).\nStatut : ${billing.status}\nÉchecs : ${billing.failureCount || 1}\n${reason ? `Motif : ${reason}\n` : ""}\nConsole : ${masterBillingUrl(tenant.slug)}`,
    html: `<p>Échec de paiement pour <strong>${tenant.label}</strong> (${tenant.slug}).</p>
<p>Statut : <strong>${billing.status}</strong> — échecs : ${billing.failureCount || 1}</p>
${reason ? `<p>Motif : ${reason}</p>` : ""}
<p><a href="${masterBillingUrl(tenant.slug)}">Gérer la facturation</a></p>`,
  });
}

export async function emailPaymentReminderAdmin(
  tenant: TenantConfig,
  stage: number,
): Promise<void> {
  const to = adminEmail(tenant);
  if (!to) return;
  const billing = readBilling(tenant);
  const grace = billing.graceEndsAt
    ? new Date(billing.graceEndsAt).toLocaleDateString("fr-FR")
    : "bientôt";

  await sendMail({
    to,
    subject: `[Scola] Rappel — régularisez votre abonnement`,
    text: `Bonjour,\n\nRappel ${stage}/3 : votre abonnement Scola pour ${tenant.label} est en retard de paiement.\nSans régularisation avant le ${grace}, l'accès pourra être suspendu (vos données seront conservées).\n\nContact : ${MARKETING.contactEmail}`,
    html: `<p>Bonjour,</p>
<p><strong>Rappel ${stage}/3</strong> : l'abonnement Scola pour <strong>${tenant.label}</strong> est en retard.</p>
<p>Sans régularisation avant le <strong>${grace}</strong>, l'accès pourra être suspendu. <em>Vos données seront conservées.</em></p>
<p>Contact : ${MARKETING.contactEmail}</p>`,
  });
}

export async function emailTenantSuspended(
  tenant: TenantConfig,
  reason?: string,
): Promise<void> {
  const to = adminEmail(tenant);
  if (to) {
    await sendMail({
      to,
      subject: `[Scola] Accès suspendu — abonnement`,
      text: `Bonjour,\n\nL'accès à Scola pour ${tenant.label} a été suspendu.\n${reason ? `Motif : ${reason}\n` : ""}\nVos données sont conservées. Contactez-nous pour régulariser : ${MARKETING.contactEmail}`,
      html: `<p>Bonjour,</p>
<p>L'accès à <strong>Scola</strong> pour <strong>${tenant.label}</strong> a été <strong>suspendu</strong>.</p>
${reason ? `<p>Motif : ${reason}</p>` : ""}
<p><em>Vos données sont conservées.</em> Contactez-nous pour régulariser.</p>`,
    });
  }
  await sendMail({
    to: PLATFORM_ASSISTANCE_EMAIL,
    subject: `[Scola] Tenant suspendu — ${tenant.label}`,
    text: `Tenant suspendu : ${tenant.label} (${tenant.slug}).\n${reason || ""}\n${masterBillingUrl(tenant.slug)}`,
    html: `<p>Tenant suspendu : <strong>${tenant.label}</strong></p>
<p><a href="${masterBillingUrl(tenant.slug)}">Console facturation</a></p>`,
  });
}

export async function emailMicrosoftLicensesSuspendRequested(
  tenant: TenantConfig,
  reason?: string,
): Promise<void> {
  const reseller = process.env.MICROSOFT_RESELLER_EMAIL?.trim();
  const recipients = [PLATFORM_ASSISTANCE_EMAIL, ...(reseller ? [reseller] : [])].join(", ");
  await sendMail({
    to: recipients,
    subject: `[Scola] Suspendre licences Microsoft — ${tenant.label}`,
    text: `Demande de suspension des licences Microsoft pour ${tenant.label} (${tenant.slug}).\n${reason || "Impayé prolongé"}\n\nTraiter côté revendeur CSP.`,
    html: `<p>Demande de <strong>suspension des licences Microsoft</strong> pour <strong>${tenant.label}</strong> (${tenant.slug}).</p>
<p>${reason || "Impayé prolongé"}</p>
<p>À traiter côté revendeur CSP.</p>`,
  });
}

export async function emailSignupPaymentFailed(req: {
  adminContact: { email: string; firstName: string; lastName: string };
  establishment: { legalName: string };
  accessToken: string;
}): Promise<void> {
  const payUrl = `${platformAppOrigin()}/souscrire/paiement?token=${encodeURIComponent(req.accessToken)}`;
  const name = `${req.adminContact.firstName} ${req.adminContact.lastName}`.trim();
  await sendMail({
    to: req.adminContact.email,
    subject: `[Scola] Paiement non finalisé`,
    text: `Bonjour ${name},\n\nVotre paiement pour ${req.establishment.legalName} n'a pas été finalisé.\nVous pouvez réessayer : ${payUrl}\n\n${MARKETING.contactEmail}`,
    html: `<p>Bonjour ${name},</p>
<p>Votre paiement pour <strong>${req.establishment.legalName}</strong> n'a pas été finalisé.</p>
<p><a href="${payUrl}">Réessayer le paiement</a></p>`,
  });
}
