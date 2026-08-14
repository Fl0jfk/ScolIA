import { resolveDepositFinalRecipients } from "@/app/lib/stage-contacts";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";
import { scheduleSummary } from "@/app/lib/stage-schedule";
import { resolveStagesAdminEmails, resolveStagesDirectionEmail } from "@/app/lib/stage-config";
import { tryAutoFileConventionToOneDrive } from "@/app/lib/stage-onedrive-filing";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { GetObjectCommand } from "@aws-sdk/client-s3";
import {
  STAGE_SIGNER_ROLE_LABELS,
  type StageConvention,
  type StageSignature,
} from "@/app/lib/stage-types";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import { loadAppConfig } from "@/app/lib/app-config";

async function signLink(token: string) {
  return tenantAbsolutePath(`/stages/signer?token=${encodeURIComponent(token)}`);
}

async function studentLink(token: string) {
  return tenantAbsolutePath(`/stages/eleve?token=${encodeURIComponent(token)}`);
}

async function mailer() {
  const smtp = await getTenantSmtpConfig();
  if (!smtp) return null;
  const transporter = await createTenantTransporter();
  if (!transporter) return null;
  return { smtp, transporter };
}

function studentLabel(c: StageConvention) {
  return `${c.student.firstName} ${c.student.lastName}`.trim();
}

export async function notifyStagePreconventionSubmitted(convention: StageConvention) {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" as const };

  const recipients = await resolveStagesAdminEmails();
  if (!recipients.length) return { sent: false, reason: "no_recipients" as const };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const text = [
    "Bonjour,",
    "",
    `Une préconvention de stage a été déposée et attend votre validation.`,
    "",
    `Élève : ${studentLabel(convention)} (${convention.student.className})`,
    `Entreprise : ${convention.company.name}`,
    `Période : ${convention.schedule.periodStart} → ${convention.schedule.periodEnd}`,
    `Horaires : ${scheduleSummary(convention.schedule)}`,
    "",
    `Connectez-vous à l'intranet → module Stages & conventions pour valider.`,
    "",
    "Cordialement,",
    school,
  ].join("\n");

  for (const to of recipients) {
    await m.transporter.sendMail({
      from: `"Stages ${school}" <${m.smtp.user}>`,
      to,
      subject: `[Stages] Préconvention à valider — ${studentLabel(convention)}`,
      text,
    });
  }
  return { sent: true, recipients };
}

export async function notifyStageConventionDeposited(convention: StageConvention) {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" as const };

  const recipients = await resolveStagesAdminEmails();
  if (!recipients.length) return { sent: false, reason: "no_recipients" as const };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const text = [
    "Bonjour,",
    "",
    `Une convention de stage (PDF) a été déposée par un élève.`,
    "",
    `Élève : ${studentLabel(convention)} (${convention.student.className || "classe à vérifier"})`,
    `Entreprise : ${convention.company.name}`,
    convention.company.siret ? `SIRET : ${convention.company.siret}` : null,
    `Période : ${convention.schedule.periodStart} → ${convention.schedule.periodEnd}`,
    "",
    `Connectez-vous à l'intranet → module Stages & conventions pour valider le dépôt.`,
    "",
    "Cordialement,",
    school,
  ]
    .filter(Boolean)
    .join("\n");

  for (const to of recipients) {
    await m.transporter.sendMail({
      from: `"Stages ${school}" <${m.smtp.user}>`,
      to,
      subject: `[Stages] Convention PDF déposée — ${studentLabel(convention)}`,
      text,
    });
  }
  return { sent: true, recipients };
}

/** Refus automatique à l'upload (signatures papier manquantes). */
export async function notifyStageDepositPaperRejected(params: {
  studentLabel: string;
  missingSignatures: string[];
  missingFields?: string[];
  notifyEmails: string[];
}) {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" as const };
  if (!params.notifyEmails.length) return { sent: false, reason: "no_recipients" as const };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const details: string[] = [];
  if (params.missingSignatures.length) {
    details.push(`Signatures manquantes : ${params.missingSignatures.join(", ")}.`);
  }
  if (params.missingFields?.length) {
    details.push(`Champs incomplets : ${params.missingFields.join(", ")}.`);
  }
  const text = [
    "Bonjour,",
    "",
    `La convention de stage de ${params.studentLabel} n'a pas pu être acceptée.`,
    "",
    ...details,
    "",
    "Veuillez compléter le document (champs + signatures élève, responsable légal et organisme d'accueil),",
    "puis déposer à nouveau le PDF sur la page de dépôt des conventions.",
    "",
    "Cordialement,",
    school,
  ].join("\n");

  for (const to of params.notifyEmails) {
    await m.transporter.sendMail({
      from: `"Stages ${school}" <${m.smtp.user}>`,
      to,
      subject: `[Stages] Convention refusée — signature manquante (${params.studentLabel})`,
      text,
    });
  }
  return { sent: true, recipients: params.notifyEmails };
}

/** Refus par l'administration d'un dépôt PDF. */
export async function notifyStageDepositAdminRejected(convention: StageConvention, note?: string) {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" as const };

  const recipients = await resolveDepositFinalRecipients(convention);
  if (!recipients.length) return { sent: false, reason: "no_recipients" as const };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const text = [
    "Bonjour,",
    "",
    `La convention de stage de ${studentLabel(convention)} a été refusée par l'administration.`,
    note ? `Motif : ${note}` : null,
    "",
    "Veuillez corriger le document et le déposer à nouveau sur la page de dépôt des conventions.",
    "",
    "Cordialement,",
    school,
  ]
    .filter(Boolean)
    .join("\n");

  for (const to of recipients) {
    await m.transporter.sendMail({
      from: `"Stages ${school}" <${m.smtp.user}>`,
      to,
      subject: `[Stages] Convention refusée — ${studentLabel(convention)}`,
      text,
    });
  }
  return { sent: true, recipients };
}

export async function notifyStageAdminRejected(convention: StageConvention, note?: string) {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" as const };

  const to =
    convention.student.email?.trim() ||
    convention.parentSignerEmail?.trim() ||
    convention.student.parentEmail?.trim();
  if (!to) return { sent: false, reason: "no_recipients" as const };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const link = convention.studentAccessToken ? await studentLink(convention.studentAccessToken) : null;

  const text = [
    "Bonjour,",
    "",
    `La préconvention de stage de ${studentLabel(convention)} doit être corrigée.`,
    note ? `Motif : ${note}` : null,
    link ? `Lien pour modifier : ${link}` : null,
    "",
    "Cordialement,",
    school,
  ]
    .filter(Boolean)
    .join("\n");

  await m.transporter.sendMail({
    from: `"Stages ${school}" <${m.smtp.user}>`,
    to,
    subject: `[Stages] Préconvention à corriger — ${studentLabel(convention)}`,
    text,
  });
  return { sent: true, recipients: [to] };
}

export async function notifyStageSignatureRequest(
  convention: StageConvention,
  signature: StageSignature,
) {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" as const };

  const to = signature.signEmail?.trim();
  if (!to || !signature.signToken) return { sent: false, reason: "no_email" as const };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const roleLabel = STAGE_SIGNER_ROLE_LABELS[signature.role];
  const link = await signLink(signature.signToken);
  const intranetStages = await tenantAbsolutePath("/stages");

  const profHint =
    signature.role === "professeur_referent"
      ? [
          "",
          "Conseil : enregistrez votre signature une fois dans l'intranet (module Stages → « Ma signature »),",
          "puis signez en un clic depuis ce lien ou depuis le bandeau de notifications du module.",
        ]
      : signature.role === "direction"
        ? [
            "",
            "Votre paraphe direction sera apposé automatiquement sur le PDF de la convention.",
          ]
        : [];

  const text = [
    "Bonjour,",
    "",
    `La convention de stage de ${studentLabel(convention)} (${convention.student.className}) a été validée par l'administration.`,
    `Votre signature est maintenant requise en tant que ${roleLabel}.`,
    "",
    `Entreprise : ${convention.company.name}`,
    `Période : ${convention.schedule.periodStart} → ${convention.schedule.periodEnd}`,
    `Horaires : ${scheduleSummary(convention.schedule)}`,
    "",
    "Pour signer (paraphe ajouté directement sur le PDF) :",
    link,
    "",
    `Vous pouvez aussi ouvrir le module Stages : ${intranetStages}`,
    ...profHint,
    "",
    "Cordialement,",
    school,
  ].join("\n");

  await m.transporter.sendMail({
    from: `"Stages ${school}" <${m.smtp.user}>`,
    to,
    subject: `[Stages] Convention validée — signature requise (${roleLabel}) — ${studentLabel(convention)}`,
    text,
  });
  return { sent: true, recipients: [to] };
}

export async function notifyAllStageSignatureRequests(convention: StageConvention) {
  const results: Array<{ role: string; sent: boolean; reason?: string }> = [];
  for (const sig of convention.signatures) {
    if (sig.status !== "en_attente") continue;
    const r = await notifyStageSignatureRequest(convention, sig);
    results.push({
      role: sig.role,
      sent: r.sent,
      reason: !r.sent && "reason" in r ? String(r.reason) : undefined,
    });
  }
  const sentCount = results.filter((r) => r.sent).length;
  return { sentCount, total: results.length, results };
}

async function loadUploadedPdfAttachment(convention: StageConvention) {
  const key = convention.uploadedPdf?.s3Key;
  if (!key) return null;
  try {
    const s3Client = await getTenantDataS3Client();
    const obj = await s3Client.send(
      new GetObjectCommand({ Bucket: await getBucketName(), Key: key }),
    );
    const bytes = await obj.Body?.transformToByteArray();
    if (!bytes?.length) return null;
    return {
      filename: convention.uploadedPdf!.fileName || "convention.pdf",
      content: Buffer.from(bytes),
      contentType: "application/pdf",
    };
  } catch {
    return null;
  }
}

function uniqueEmails(...lists: Array<string | undefined | null>): string[] {
  const set = new Set<string>();
  for (const e of lists) {
    const v = String(e || "").trim().toLowerCase();
    if (v && /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v)) set.add(v);
  }
  return [...set];
}

export async function notifyStageFullySigned(convention: StageConvention) {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" as const };

  const isDepositFlow = Boolean(convention.uploadedPdf?.s3Key);
  let recipients: string[];
  if (isDepositFlow) {
    recipients = await resolveDepositFinalRecipients(convention);
  } else {
    const directionEmail = await resolveStagesDirectionEmail(convention.student.level);
    const admins = await resolveStagesAdminEmails();
    recipients = uniqueEmails(
      convention.student.email,
      convention.student.parentEmail,
      convention.parentSignerEmail,
      convention.company.tutorEmail,
      convention.company.rhEmail,
      directionEmail,
      convention.teacherReferent.email,
      ...admins,
      ...convention.signatures.map((s) => s.signEmail),
    );
  }
  if (!recipients.length) return { sent: false, reason: "no_recipients" as const };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const attachment = await loadUploadedPdfAttachment(convention);

  const text = [
    "Bonjour,",
    "",
    `La convention de stage de ${studentLabel(convention)} (${convention.student.className}) est finalisée et signée par toutes les parties.`,
    "",
    `Organisme d'accueil : ${convention.company.name}`,
    `Période : ${convention.schedule.periodStart} → ${convention.schedule.periodEnd}`,
    attachment
      ? "Vous trouverez la convention signée en pièce jointe."
      : "Connectez-vous à l'intranet pour consulter le dossier.",
    "",
    "Cordialement,",
    school,
  ].join("\n");

  for (const to of recipients) {
    await m.transporter.sendMail({
      from: `"Stages ${school}" <${m.smtp.user}>`,
      to,
      subject: `[Stages] Convention finalisée — ${studentLabel(convention)}`,
      text,
      ...(attachment ? { attachments: [attachment] } : {}),
    });
  }

  void tryAutoFileConventionToOneDrive(convention).catch((e) =>
    console.error("[stages] auto OneDrive:", e),
  );

  return { sent: true, recipients };
}
