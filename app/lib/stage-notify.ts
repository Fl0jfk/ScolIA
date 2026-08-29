import { resolveDepositFinalRecipients } from "@/app/lib/stage-contacts";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";
import { scheduleSummary } from "@/app/lib/stage-schedule";
import { resolveStagesAdminEmails, resolveStagesDirectionEmail } from "@/app/lib/stage-config";
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

  const recipients = uniqueContactEmails(
    convention.student.email,
    convention.parentSignerEmail,
    convention.parent2SignerEmail,
    convention.student.parent1Email,
    convention.student.parent2Email,
    convention.student.parentEmail,
  );
  if (!recipients.length) return { sent: false, reason: "no_recipients" as const };

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

  for (const to of recipients) {
    await m.transporter.sendMail({
      from: `"Stages ${school}" <${m.smtp.user}>`,
      to,
      subject: `[Stages] Préconvention à corriger — ${studentLabel(convention)}`,
      text,
    });
  }
  return { sent: true, recipients };
}

function uniqueContactEmails(...lists: Array<string | undefined | null>): string[] {
  const set = new Set<string>();
  for (const e of lists) {
    const v = String(e || "").trim().toLowerCase();
    if (v && /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v)) set.add(v);
  }
  return [...set];
}

async function notifyStageSignatureRequest(
  convention: StageConvention,
  signature: StageSignature,
): Promise<{ sent: boolean; reason?: string; error?: string }> {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" };

  const to = signature.signEmail?.trim();
  if (!to || !signature.signToken) return { sent: false, reason: "no_email" };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const roleLabel = STAGE_SIGNER_ROLE_LABELS[signature.role];
  const link = await signLink(signature.signToken);
  const intranetStages = await tenantAbsolutePath("/stages");

  const profHint =
    signature.role === "professeur_referent"
      ? [
          "",
          "Conseil : enregistrez votre signature une fois dans Mon compte → Sécurité → Ma signature,",
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
    signature.signSecureCode
      ? `Code de signature sécurisé : ${signature.signSecureCode}`
      : null,
    "Pour signer en ligne :",
    link,
    signature.signSecureCode
      ? `Vous pouvez aussi ouvrir ${await tenantAbsolutePath("/stages/signer")} et saisir votre e-mail + le code ci-dessus.`
      : null,
    "",
    `Vous pouvez aussi ouvrir le module Stages : ${intranetStages}`,
    ...profHint,
    "",
    "Cordialement,",
    school,
  ]
    .filter(Boolean)
    .join("\n");

  try {
    await m.transporter.sendMail({
      from: `"Stages ${school}" <${m.smtp.user}>`,
      to,
      subject: `[Stages] Convention validée — signature requise (${roleLabel}) — ${studentLabel(convention)}`,
      text,
    });
    return { sent: true };
  } catch (err) {
    console.error("[stages] send signature mail failed:", to, err);
    return {
      sent: false,
      reason: "smtp_error",
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

export { notifyStageSignatureRequest };

/** Code OTP pour confirmer l'e-mail du responsable légal avant soumission. */
export async function notifyParentEmailVerification(params: {
  to: string;
  studentName: string;
  code: string;
}) {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" as const };
  const to = params.to.trim();
  if (!to) return { sent: false, reason: "no_email" as const };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const text = [
    "Bonjour,",
    "",
    `Pour confirmer votre adresse e-mail et envoyer la préconvention de stage de ${params.studentName},`,
    `saisissez ce code à 6 chiffres sur la page du formulaire :`,
    "",
    `  ${params.code}`,
    "",
    "Ce code est valable 30 minutes.",
    "",
    "Si vous n'êtes pas à l'origine de cette demande, ignorez ce message.",
    "",
    "Cordialement,",
    school,
  ].join("\n");

  try {
    await m.transporter.sendMail({
      from: `"Stages ${school}" <${m.smtp.user}>`,
      to,
      subject: `[Stages] Code de confirmation e-mail — ${params.studentName}`,
      text,
    });
    return { sent: true as const };
  } catch (err) {
    console.error("[stages] parent verify mail failed:", to, err);
    return {
      sent: false as const,
      reason: "smtp_error" as const,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

/** Signature refusée par l'administratif — nouvelle demande envoyée au signataire. */
export async function notifyStageSignatureRejected(
  convention: StageConvention,
  signature: StageSignature,
  note?: string,
) {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" as const };
  const to = signature.signEmail?.trim();
  if (!to || !signature.signToken) return { sent: false, reason: "no_email" as const };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const roleLabel = STAGE_SIGNER_ROLE_LABELS[signature.role];
  const link = await signLink(signature.signToken);

  const text = [
    "Bonjour,",
    "",
    `Votre signature pour la convention de stage de ${studentLabel(convention)} n'a pas pu être acceptée.`,
    note ? `Motif : ${note}` : null,
    "",
    "Merci de signer à nouveau la convention en utilisant l'un des modes proposés :",
    "- code sécurisé reçu par e-mail,",
    "- signature au doigt sur l'écran,",
    "- ou dépôt du document signé en papier (scan / photo PDF).",
    "",
    signature.signSecureCode ? `Nouveau code : ${signature.signSecureCode}` : null,
    `Lien : ${link}`,
    "",
    "Cordialement,",
    school,
  ]
    .filter(Boolean)
    .join("\n");

  try {
    await m.transporter.sendMail({
      from: `"Stages ${school}" <${m.smtp.user}>`,
      to,
      subject: `[Stages] Signature non acceptée — ${roleLabel} — ${studentLabel(convention)}`,
      text,
    });
    return { sent: true, recipients: [to] };
  } catch (err) {
    console.error("[stages] signature rejected mail failed:", to, err);
    return { sent: false, reason: "smtp_error" as const };
  }
}

/** Alerte parent : l'e-mail du tuteur d'entreprise a échoué. */
export async function notifyParentTutorEmailFailed(
  convention: StageConvention,
  tutorEmail: string,
  smtpError?: string,
) {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" as const };

  const recipients = uniqueContactEmails(
    convention.parentSignerEmail,
    convention.student.parent1Email,
    convention.student.parentEmail,
    convention.parent2SignerEmail,
    convention.student.parent2Email,
  );
  if (!recipients.length) return { sent: false, reason: "no_recipients" as const };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const studentLinkUrl = convention.studentAccessToken
    ? await studentLink(convention.studentAccessToken)
    : await tenantAbsolutePath("/stages/preconvention");

  const text = [
    "Bonjour,",
    "",
    `Nous avons tenté d'envoyer la convention de stage de ${studentLabel(convention)} au tuteur en entreprise.`,
    `L'adresse e-mail indiquée a renvoyé une erreur :`,
    "",
    `  ${tutorEmail}`,
    smtpError ? `Détail technique : ${smtpError}` : null,
    "",
    "Merci de vérifier et de corriger l'adresse du tuteur si besoin.",
    "Ouvrez le dossier élève pour modifier l'e-mail du tuteur :",
    studentLinkUrl,
    "",
    "Cordialement,",
    school,
  ]
    .filter(Boolean)
    .join("\n");

  const sentTo: string[] = [];
  for (const to of recipients) {
    try {
      await m.transporter.sendMail({
        from: `"Stages ${school}" <${m.smtp.user}>`,
        to,
        subject: `[Stages] E-mail tuteur invalide — ${studentLabel(convention)}`,
        text,
      });
      sentTo.push(to);
    } catch (err) {
      console.error("[stages] notify parent tutor fail mail:", to, err);
    }
  }
  return sentTo.length
    ? { sent: true as const, recipients: sentTo }
    : { sent: false as const, reason: "smtp_error" as const };
}

export async function notifyAllStageSignatureRequests(convention: StageConvention) {
  const results: Array<{ role: string; sent: boolean; reason?: string; error?: string }> = [];
  for (const sig of convention.signatures) {
    if (sig.status !== "en_attente") continue;
    const r = await notifyStageSignatureRequest(convention, sig);
    results.push({
      role: sig.role,
      sent: r.sent,
      reason: r.reason,
      error: r.error,
    });

    if (
      !r.sent &&
      (sig.role === "tuteur_entreprise" || sig.role === "rh_entreprise") &&
      sig.signEmail
    ) {
      void notifyParentTutorEmailFailed(convention, sig.signEmail, r.error).catch((e) =>
        console.error("[stages] notify parent tutor email failed:", e),
      );
    }
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

  return { sent: true, recipients };
}
