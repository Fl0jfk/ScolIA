import "server-only";

import {
  billingPeriodLabel,
  buildInvoiceNumber,
  renderBillingInvoicePdf,
} from "@/app/lib/billing/invoice-pdf";
import { MARKETING } from "@/app/lib/marketing-site";
import { PLATFORM_ASSISTANCE_EMAIL } from "@/app/lib/platform-assistance-email";
import { platformAppOrigin } from "@/app/lib/platform-portal-url";
import {
  computePricingWithA3Extras,
  formatEur,
  type BillingMode,
} from "@/app/lib/pricing";
import { BILLING_GRACE_DAYS, type TenantBillingState } from "@/app/lib/tenant-billing-types";
import type { TenantConfig } from "@/app/lib/tenant-types";
import { createPlatformTransporter, getPlatformSmtpConfig } from "@/app/lib/tenant-mail";
import type { Attachment } from "nodemailer/lib/mailer";

function readBilling(tenant: TenantConfig): TenantBillingState {
  return tenant.billing || { status: "active" };
}

async function sendMail(opts: {
  to: string;
  subject: string;
  html: string;
  text: string;
  attachments?: Attachment[];
}) {
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
    attachments: opts.attachments,
  });
  return true;
}

function adminEmail(tenant: TenantConfig): string | null {
  return readBilling(tenant).adminEmail?.trim() || null;
}

function masterBillingUrl(slug: string): string {
  return `${platformAppOrigin()}/plateforme/tenants/${slug}/billing`;
}

function resolvePaidAmountEur(
  billing: TenantBillingState,
  amountCents?: number,
): { totalEur: number; fromGateway: boolean } {
  if (typeof amountCents === "number" && Number.isFinite(amountCents) && amountCents > 0) {
    // Easytransac renvoie en général des centimes ; si valeur < 100 et mode mensuel élevé, on garde centimes.
    return { totalEur: amountCents / 100, fromGateway: true };
  }
  const students = billing.estimatedStudentCount || 100;
  const mode: BillingMode = billing.billingMode || "monthly";
  const pricing = computePricingWithA3Extras(students, mode, billing.extraA3Count || 0);
  const totalEur =
    mode === "monthly" ? pricing.totalMonthlyWithExtras : pricing.totalAnnualWithExtras;
  return { totalEur, fromGateway: false };
}

/** Facture PDF + e-mail à l'administrateur général après paiement réussi. */
export async function emailTenantPaymentInvoice(
  tenant: TenantConfig,
  detail?: { tid?: string; amountCents?: number; paidAt?: string },
): Promise<boolean> {
  const billing = readBilling(tenant);
  const to = adminEmail(tenant);
  if (!to) {
    console.warn("[tenant-billing-email] Pas d'adminEmail — facture non envoyée:", tenant.slug);
    return false;
  }

  const paidAt = detail?.paidAt ? new Date(detail.paidAt) : new Date();
  const mode: BillingMode = billing.billingMode || "monthly";
  const students = billing.estimatedStudentCount || 100;
  const extraA3 = billing.extraA3Count || 0;
  const pricing = computePricingWithA3Extras(students, mode, extraA3);
  const { totalEur } = resolvePaidAmountEur(billing, detail?.amountCents);
  const periodLabel = billingPeriodLabel(mode, paidAt);
  const invoiceNumber = buildInvoiceNumber({
    slug: tenant.slug,
    paidAt,
    tid: detail?.tid,
  });

  const subscriptionEur =
    mode === "monthly" ? pricing.monthlyTotal : pricing.annualTotal;
  const extrasEur =
    mode === "monthly" ? pricing.extraA3MonthlyTotal : pricing.extraA3AnnualTotal;

  const lineItems = [
    {
      label:
        mode === "monthly"
          ? `Abonnement Scola — ${students} élève${students > 1 ? "s" : ""} (mensuel)`
          : `Abonnement Scola — ${students} élève${students > 1 ? "s" : ""} (annuel)`,
      detail: `Période : ${periodLabel}`,
      amountEur: subscriptionEur,
    },
    ...(extraA3 > 0
      ? [
          {
            label: `Licences Microsoft A3 supplémentaires (×${extraA3})`,
            detail: `${formatEur(pricing.extraA3UnitMonthly, { decimals: 2 })} / licence / mois`,
            amountEur: extrasEur,
          },
        ]
      : []),
  ];

  // Si le montant gateway diffère du détail calculé, une seule ligne « total payé »
  const linesSum = lineItems.reduce((s, l) => s + l.amountEur, 0);
  const useGatewayTotal = Math.abs(linesSum - totalEur) > 0.05;
  const pdfLines = useGatewayTotal
    ? [
        {
          label: mode === "monthly" ? "Abonnement Scola (mensuel)" : "Abonnement Scola (annuel)",
          detail: `Période : ${periodLabel}`,
          amountEur: totalEur,
        },
      ]
    : lineItems;

  const pdfBytes = await renderBillingInvoicePdf({
    invoiceNumber,
    issuedAt: paidAt,
    customerName: tenant.label,
    customerEmail: to,
    customerAddress: tenant.postalAddress,
    billingMode: mode,
    periodLabel,
    lineItems: pdfLines,
    totalEur,
    transactionId: detail?.tid,
    tenantSlug: tenant.slug,
  });

  const filename = `${invoiceNumber}.pdf`;
  const amountLabel = formatEur(totalEur, { decimals: 2 });
  const modeLabel = mode === "monthly" ? "mensuel" : "annuel";

  const sent = await sendMail({
    to,
    subject: `[Scola] Facture ${invoiceNumber} — ${tenant.label}`,
    text: `Bonjour,\n\nVotre paiement ${modeLabel} Scola pour ${tenant.label} a bien été reçu (${amountLabel}).\nPériode : ${periodLabel}\nFacture : ${invoiceNumber}\n\nLa facture PDF est jointe à cet e-mail.\n\n${MARKETING.contactEmail}`,
    html: `<p>Bonjour,</p>
<p>Votre paiement <strong>${modeLabel}</strong> Scola pour <strong>${tenant.label}</strong> a bien été reçu.</p>
<ul>
<li>Montant : <strong>${amountLabel}</strong></li>
<li>Période : ${periodLabel}</li>
<li>Facture : <strong>${invoiceNumber}</strong></li>
</ul>
<p>La facture PDF est jointe à cet e-mail.</p>
<p>Contact : ${MARKETING.contactEmail}</p>`,
    attachments: [
      {
        filename,
        content: Buffer.from(pdfBytes),
        contentType: "application/pdf",
      },
    ],
  });

  if (sent) {
    void sendMail({
      to: PLATFORM_ASSISTANCE_EMAIL,
      subject: `[Scola] Facture envoyée — ${tenant.label} (${invoiceNumber})`,
      text: `Facture ${invoiceNumber} envoyée à ${to} pour ${tenant.label} (${tenant.slug}).\nMontant : ${amountLabel}\n${masterBillingUrl(tenant.slug)}`,
      html: `<p>Facture <strong>${invoiceNumber}</strong> envoyée à ${to}.</p>
<p>${tenant.label} — ${amountLabel}</p>
<p><a href="${masterBillingUrl(tenant.slug)}">Console facturation</a></p>`,
    });
  }

  return sent;
}

/** Facture PDF pour le premier paiement (dossier /souscrire), avant provisioning. */
export async function emailSignupPaymentInvoice(
  req: {
    id: string;
    billingMode?: BillingMode;
    extraA3Count?: number;
    adminContact: { email: string; firstName: string; lastName: string };
    establishment: {
      legalName: string;
      estimatedStudentCount: number;
      postalAddress?: { street?: string; zip?: string; city?: string };
    };
    easytransac?: { paymentPageRequestId?: string; lastPaymentAt?: string };
  },
  detail?: { tid?: string; amountCents?: number },
): Promise<boolean> {
  const to = req.adminContact.email.trim().toLowerCase();
  if (!to) return false;

  const paidAt = req.easytransac?.lastPaymentAt
    ? new Date(req.easytransac.lastPaymentAt)
    : new Date();
  const mode: BillingMode = req.billingMode || "monthly";
  const students = req.establishment.estimatedStudentCount || 100;
  const extraA3 = req.extraA3Count || 0;
  const pricing = computePricingWithA3Extras(students, mode, extraA3);
  const computed =
    mode === "monthly" ? pricing.totalMonthlyWithExtras : pricing.totalAnnualWithExtras;
  const totalEur =
    typeof detail?.amountCents === "number" && detail.amountCents > 0
      ? detail.amountCents / 100
      : computed;
  const tid = detail?.tid || req.easytransac?.paymentPageRequestId;
  const periodLabel = billingPeriodLabel(mode, paidAt);
  const slugHint = req.establishment.legalName
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")
    .slice(0, 20);
  const invoiceNumber = buildInvoiceNumber({
    slug: slugHint || req.id.slice(0, 8),
    paidAt,
    tid,
  });
  const name = `${req.adminContact.firstName} ${req.adminContact.lastName}`.trim();

  const pdfBytes = await renderBillingInvoicePdf({
    invoiceNumber,
    issuedAt: paidAt,
    customerName: req.establishment.legalName,
    customerEmail: to,
    customerAddress: req.establishment.postalAddress,
    billingMode: mode,
    periodLabel,
    lineItems: [
      {
        label:
          mode === "monthly"
            ? `Abonnement Scola — ${students} élèves (mensuel)`
            : `Abonnement Scola — ${students} élèves (annuel)`,
        detail: `Période : ${periodLabel}${name ? ` · Contact : ${name}` : ""}`,
        amountEur: totalEur,
      },
    ],
    totalEur,
    transactionId: tid,
  });

  const amountLabel = formatEur(totalEur, { decimals: 2 });
  return sendMail({
    to,
    subject: `[Scola] Facture ${invoiceNumber} — ${req.establishment.legalName}`,
    text: `Bonjour ${name},\n\nVotre paiement Scola pour ${req.establishment.legalName} est confirmé (${amountLabel}).\nFacture : ${invoiceNumber}\n\nLa facture PDF est jointe.\n\n${MARKETING.contactEmail}`,
    html: `<p>Bonjour ${name},</p>
<p>Votre paiement Scola pour <strong>${req.establishment.legalName}</strong> est confirmé.</p>
<p>Montant : <strong>${amountLabel}</strong> — Facture : <strong>${invoiceNumber}</strong></p>
<p>La facture PDF est jointe à cet e-mail.</p>`,
    attachments: [
      {
        filename: `${invoiceNumber}.pdf`,
        content: Buffer.from(pdfBytes),
        contentType: "application/pdf",
      },
    ],
  });
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
