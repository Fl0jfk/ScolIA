import { resolveDepositFinalRecipients } from "@/app/lib/stage-contacts";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";
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

  const recipients = await resolveStagesAdminEmails(
    convention.student.level,
    convention.student.className,
  );
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

/** Alerte secrétariat : une demande d'avenant (dates / horaires) est en attente. */
export async function notifyStageScheduleChangeRequested(convention: StageConvention) {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" as const };

  const recipients = await resolveStagesAdminEmails(
    convention.student.level,
    convention.student.className,
  );
  if (!recipients.length) return { sent: false, reason: "no_recipients" as const };

  const req = convention.scheduleChangeRequest;
  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const prev = req?.previousSchedule;
  const next = req?.requestedSchedule;
  const fromStaff = req?.source === "staff" || req?.requestedByRole === "secretariat";
  const text = [
    "Bonjour,",
    "",
    fromStaff
      ? `Une demande d'avenant a été ouverte par l'établissement (dates / horaires de stage).`
      : `Une demande d'avenant a été déposée via le lien de signature (dates / horaires de stage).`,
    "",
    `Élève : ${studentLabel(convention)} (${convention.student.className})`,
    `Entreprise : ${convention.company.name}`,
    `Demandé par : ${req?.requestedByLabel || "Signataire"}`,
    prev
      ? `Période actuelle : ${prev.periodStart} → ${prev.periodEnd}`
      : null,
    next
      ? `Période demandée : ${next.periodStart} → ${next.periodEnd}`
      : null,
    req?.note ? `Motif : ${req.note}` : null,
    "",
    `Si vous appliquez l'avenant, le PDF est régénéré et — si des signatures étaient en cours — chaque signataire devra re-signer.`,
    `Connectez-vous à l'intranet → module Stages & conventions pour traiter la demande.`,
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
      subject: `[Stages] Avenant dates/horaires — ${studentLabel(convention)}`,
      text,
    });
  }
  return { sent: true, recipients };
}

/**
 * Informe les parties (via leurs liens de signature) qu'un avenant est proposé
 * et qu'elles peuvent discuter / répondre sur le lien.
 */
export async function notifyStageAmendmentProposed(convention: StageConvention) {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" as const };

  const req = convention.scheduleChangeRequest;
  if (!req) return { sent: false, reason: "no_request" as const };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const prev = req.previousSchedule;
  const next = req.requestedSchedule;
  let sentCount = 0;

  for (const signature of convention.signatures) {
    const to = signature.signEmail?.trim();
    if (!to || !signature.signToken) continue;
    if (signature.status === "refuse") continue;
    const link = await signLink(signature.signToken);
    const roleLabel = STAGE_SIGNER_ROLE_LABELS[signature.role];
    const text = [
      `Bonjour ${signature.label?.trim() || roleLabel},`,
      "",
      `Concernant la convention de stage de ${studentLabel(convention)} (${convention.student.className}) chez ${convention.company.name} :`,
      "",
      `Une demande d'avenant a été ouverte par ${req.requestedByLabel}.`,
      `Période actuelle : ${prev.periodStart} → ${prev.periodEnd}`,
      `Période proposée : ${next.periodStart} → ${next.periodEnd}`,
      req.note ? `Motif : ${req.note}` : null,
      "",
      "Ouvrez votre lien sécurisé pour consulter la proposition, échanger dans le fil de discussion,",
      "ou proposer d'autres dates si besoin :",
      link,
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
        subject: `[Stages] Demande d'avenant — ${studentLabel(convention)}`,
        text,
      });
      sentCount += 1;
    } catch (err) {
      console.error("[stages] amendment mail failed:", to, err);
    }
  }

  return { sent: sentCount > 0, sentCount };
}

/** Notifie les autres participants du fil (via e-mail + lien) qu'un message a été posté. */
export async function notifyStageDiscussionMessage(
  convention: StageConvention,
  message: { authorLabel: string; body: string; authorRole: string },
) {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" as const };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const snippet = message.body.length > 280 ? `${message.body.slice(0, 277)}…` : message.body;
  let sentCount = 0;

  for (const signature of convention.signatures) {
    const to = signature.signEmail?.trim();
    if (!to || !signature.signToken) continue;
    if (signature.status === "refuse") continue;
    // Ne pas renvoyer à l'auteur du message.
    if (
      signature.label?.trim().toLowerCase() === message.authorLabel.trim().toLowerCase() &&
      signature.role === message.authorRole
    ) {
      continue;
    }
    const link = await signLink(signature.signToken);
    const text = [
      `Bonjour ${signature.label?.trim() || STAGE_SIGNER_ROLE_LABELS[signature.role]},`,
      "",
      `Nouveau message sur la convention de ${studentLabel(convention)} :`,
      `${message.authorLabel} : ${snippet}`,
      "",
      "Répondre via le lien sécurisé :",
      link,
      "",
      "Cordialement,",
      school,
    ].join("\n");

    try {
      await m.transporter.sendMail({
        from: `"Stages ${school}" <${m.smtp.user}>`,
        to,
        subject: `[Stages] Message — ${studentLabel(convention)}`,
        text,
      });
      sentCount += 1;
    } catch (err) {
      console.error("[stages] discussion mail failed:", to, err);
    }
  }

  return { sent: sentCount > 0, sentCount };
}

export async function notifyStageConventionDeposited(convention: StageConvention) {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" as const };

  const recipients = await resolveStagesAdminEmails(
    convention.student.level,
    convention.student.className,
  );
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
  const period =
    convention.schedule.periodStart && convention.schedule.periodEnd
      ? `${convention.schedule.periodStart} → ${convention.schedule.periodEnd}`
      : null;
  const signerName = signature.label?.trim() || roleLabel;

  const text = [
    `Bonjour ${signerName},`,
    "",
    `La convention de stage de ${studentLabel(convention)} (classe ${convention.student.className}) a bien été validée par l'administration.`,
    `Votre signature est requise en tant que ${roleLabel}.`,
    period ? `Période du stage : ${period}.` : null,
    "",
    "Pour signer la convention, ouvrez le lien sécurisé ci-dessous :",
    link,
    "",
    "Sur ce même lien, vous pouvez aussi demander un avenant (changement de dates ou d'horaires)",
    "et échanger avec la direction, le responsable légal, le tuteur et le professeur référent.",
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

/** OTP envoyé uniquement quand le signataire choisit « Code e-mail » puis Valider. */
export async function notifyStageSignConfirmCode(params: {
  to: string;
  studentName: string;
  roleLabel: string;
  code: string;
}): Promise<{ sent: boolean; reason?: string; error?: string }> {
  const m = await mailer();
  if (!m) return { sent: false, reason: "smtp" };
  const to = params.to.trim();
  if (!to) return { sent: false, reason: "no_email" };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const text = [
    "Bonjour,",
    "",
    `Voici votre code de confirmation pour signer la convention de stage de ${params.studentName}`,
    `en tant que ${params.roleLabel} :`,
    "",
    `  ${params.code}`,
    "",
    "Ce code est valable 30 minutes. Saisissez-le sur la page de signature pour valider.",
    "",
    "Cordialement,",
    school,
  ].join("\n");

  try {
    await m.transporter.sendMail({
      from: `"Stages ${school}" <${m.smtp.user}>`,
      to,
      subject: `[Stages] Code de signature — ${params.studentName}`,
      text,
    });
    return { sent: true };
  } catch (err) {
    console.error("[stages] send sign confirm code failed:", to, err);
    return {
      sent: false,
      reason: "smtp_error",
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

/** Code OTP pour confirmer l'e-mail du responsable légal avant soumission. */
export async function notifyParentEmailVerification(params: {
  to: string;
  studentName: string;
  code: string;
}) {
  return notifyIdentityAccessOtp({
    recipients: [params.to],
    studentName: params.studentName,
    code: params.code,
    purpose: "parent_verify",
  });
}

/**
 * Envoie le même code OTP à plusieurs destinataires (élève + responsables).
 * Utilisé pour sécuriser l'accès à la préconvention.
 */
export async function notifyIdentityAccessOtp(params: {
  recipients: string[];
  studentName: string;
  code: string;
  purpose?: "identity_access" | "parent_verify";
}) {
  const m = await mailer();
  if (!m) return { sentCount: 0, reason: "smtp" as const };

  const recipients = [
    ...new Set(
      params.recipients
        .map((r) => r.trim().toLowerCase())
        .filter((r) => r.includes("@")),
    ),
  ];
  if (recipients.length === 0) return { sentCount: 0, reason: "no_email" as const };

  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  const purpose = params.purpose ?? "identity_access";
  const intro =
    purpose === "parent_verify"
      ? `Pour confirmer votre adresse e-mail et envoyer la préconvention de stage de ${params.studentName},`
      : `Pour accéder à l'espace stages de ${params.studentName},`;
  const subject =
    purpose === "parent_verify"
      ? `[Stages] Code de confirmation e-mail — ${params.studentName}`
      : `[Stages] Code d'accès — ${params.studentName}`;

  const text = [
    "Bonjour,",
    "",
    intro,
    `saisissez ce code à 6 chiffres sur la page du formulaire :`,
    "",
    `  ${params.code}`,
    "",
    "Ce code est valable 30 minutes. Le même code a pu être envoyé à plusieurs adresses de la famille.",
    "",
    "Si vous n'êtes pas à l'origine de cette demande, ignorez ce message.",
    "",
    "Cordialement,",
    school,
  ].join("\n");

  let sentCount = 0;
  let lastError: string | undefined;
  for (const to of recipients) {
    try {
      await m.transporter.sendMail({
        from: `"Stages ${school}" <${m.smtp.user}>`,
        to,
        subject,
        text,
      });
      sentCount += 1;
    } catch (err) {
      console.error("[stages] identity otp mail failed:", to, err);
      lastError = err instanceof Error ? err.message : String(err);
    }
  }

  if (sentCount === 0) {
    return {
      sentCount: 0,
      reason: "smtp_error" as const,
      error: lastError,
    };
  }
  return { sentCount };
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
    "Merci de signer à nouveau via le lien ci-dessous (code e-mail, signature au doigt, ou document papier) :",
    link,
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
    const directionEmail = await resolveStagesDirectionEmail(
      convention.student.level,
      convention.student.className,
    );
    const admins = await resolveStagesAdminEmails(
      convention.student.level,
      convention.student.className,
    );
    recipients = uniqueEmails(
      convention.student.email,
      convention.student.parentEmail,
      convention.student.parent1Email,
      convention.student.parent2Email,
      convention.parentSignerEmail,
      convention.parent2SignerEmail,
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
