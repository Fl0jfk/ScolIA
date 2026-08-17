import { randomBytes } from "crypto";
import {
  DeleteObjectsCommand,
  GetObjectCommand,
  ListObjectsV2Command,
  PutObjectCommand,
} from "@aws-sdk/client-s3";
import { loadAppConfig } from "@/app/lib/app-config";
import { escapeHtml } from "@/app/lib/escape-html";
import { assertEligibleRequestAttachment, sanitizeRequestFileName } from "@/app/lib/requests";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getTenantBucketName } from "@/app/lib/tenant-config";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
  getTenantSmtpFromAddress,
} from "@/app/lib/tenant-mail";

export const RENTREE_SUBMISSION_TTL_MS = 72 * 60 * 60 * 1000;
export const RENTREE_SUBMISSION_MAX_BYTES = 12 * 1024 * 1024;

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export type RentreePendingSubmission = {
  version: 1;
  senderEmail: string;
  studentName?: string;
  fileName: string;
  contentType: string;
  size: number;
  fileKey: string;
  itemId: string;
  itemTitle: string;
  establishmentId: string;
  establishmentLabel: string;
  recipientEmails: string[];
  createdAt: string;
  expiresAt: string;
};

function pendingPrefix(token: string) {
  return `rentree/submissions/pending/${token}/`;
}

function metaKey(token: string) {
  return `${pendingPrefix(token)}meta.json`;
}

export function generateRentreeSubmissionToken(): string {
  return randomBytes(32).toString("base64url");
}

export function isValidRentreeSenderEmail(value: string): boolean {
  return EMAIL_RE.test(value.trim().toLowerCase());
}

export function normalizeRentreeSenderEmail(value: string): string {
  return value.trim().toLowerCase();
}

export async function savePendingRentreeSubmission(
  token: string,
  fields: Omit<RentreePendingSubmission, "version" | "createdAt" | "expiresAt" | "fileKey">,
  file: { buffer: Buffer; fileName: string; contentType: string },
): Promise<void> {
  const check = assertEligibleRequestAttachment(file.fileName, file.contentType, file.buffer.length);
  if (!check.ok) throw new Error(check.error);

  const s3Client = await getTenantDataS3Client();
  const bucket = await getTenantBucketName();
  const now = new Date().toISOString();
  const expiresAt = new Date(Date.now() + RENTREE_SUBMISSION_TTL_MS).toISOString();
  const safe = sanitizeRequestFileName(file.fileName);
  const fileKey = `${pendingPrefix(token)}files/${safe}`;

  await s3Client.send(
    new PutObjectCommand({
      Bucket: bucket,
      Key: fileKey,
      Body: file.buffer,
      ContentType: file.contentType || "application/octet-stream",
    }),
  );

  const meta: RentreePendingSubmission = {
    version: 1,
    ...fields,
    fileKey,
    createdAt: now,
    expiresAt,
  };

  await s3Client.send(
    new PutObjectCommand({
      Bucket: bucket,
      Key: metaKey(token),
      Body: JSON.stringify(meta),
      ContentType: "application/json",
    }),
  );
}

export async function loadPendingRentreeSubmission(token: string): Promise<RentreePendingSubmission | null> {
  const trimmed = token.trim();
  if (!trimmed || trimmed.length > 200 || /[^a-zA-Z0-9_-]/.test(trimmed)) return null;
  const s3Client = await getTenantDataS3Client();
  try {
    const res = await s3Client.send(
      new GetObjectCommand({
        Bucket: await getTenantBucketName(),
        Key: metaKey(trimmed),
      }),
    );
    const body = await res.Body?.transformToString();
    if (!body) return null;
    const parsed = JSON.parse(body) as RentreePendingSubmission;
    if (parsed.version !== 1 || !parsed.senderEmail || !parsed.fileKey || !parsed.recipientEmails?.length) {
      return null;
    }
    return parsed;
  } catch (e: unknown) {
    const err = e as { name?: string };
    if (err?.name === "NoSuchKey") return null;
    throw e;
  }
}

export async function deletePendingRentreeSubmission(token: string): Promise<void> {
  const s3Client = await getTenantDataS3Client();
  const bucket = await getTenantBucketName();
  const prefix = pendingPrefix(token.trim());
  let continuationToken: string | undefined;
  do {
    const list = await s3Client.send(
      new ListObjectsV2Command({ Bucket: bucket, Prefix: prefix, ContinuationToken: continuationToken }),
    );
    const keys = (list.Contents ?? []).map((c) => c.Key).filter(Boolean) as string[];
    if (keys.length > 0) {
      await s3Client.send(
        new DeleteObjectsCommand({
          Bucket: bucket,
          Delete: { Objects: keys.map((Key) => ({ Key })), Quiet: true },
        }),
      );
    }
    continuationToken = list.IsTruncated ? list.NextContinuationToken : undefined;
  } while (continuationToken);
}

export async function loadPendingRentreeFileBytes(fileKey: string): Promise<Buffer | null> {
  const s3Client = await getTenantDataS3Client();
  try {
    const res = await s3Client.send(
      new GetObjectCommand({
        Bucket: await getTenantBucketName(),
        Key: fileKey,
      }),
    );
    const bytes = await res.Body?.transformToByteArray();
    return bytes?.length ? Buffer.from(bytes) : null;
  } catch {
    return null;
  }
}

async function schoolLabel(): Promise<string> {
  try {
    const app = await loadAppConfig();
    return app.identity.shortName?.trim() || app.identity.name?.trim() || "Établissement";
  } catch {
    return "Établissement";
  }
}

async function mailer() {
  const smtp = await getTenantSmtpConfig();
  const transporter = await createTenantTransporter();
  const fromAddr = (await getTenantSmtpFromAddress()) || smtp?.user;
  if (!smtp || !transporter || !fromAddr) return null;
  return { transporter, fromAddr };
}

export async function notifyRentreeSubmissionVerification(
  senderEmail: string,
  itemTitle: string,
  confirmUrl: string,
): Promise<void> {
  const mail = await mailer();
  if (!mail) throw new Error("SMTP non configuré");
  const school = await schoolLabel();
  await mail.transporter.sendMail({
    from: `"${school}" <${mail.fromAddr}>`,
    to: senderEmail,
    subject: `Confirmez l’envoi de « ${itemTitle} »`,
    text: [
      "Bonjour,",
      "",
      `Vous avez déposé un document (« ${itemTitle} ») sur la page rentrée.`,
      "Pour le transmettre à l’établissement, cliquez sur ce lien (une seule fois) :",
      "",
      confirmUrl,
      "",
      "Le lien est valable 72 heures. Si vous n’êtes pas à l’origine de ce message, ignorez-le.",
    ].join("\n"),
    html: `
      <p>Bonjour,</p>
      <p>Vous avez déposé un document (<strong>${escapeHtml(itemTitle)}</strong>) sur la page rentrée.</p>
      <p>Pour le transmettre à l’établissement, cliquez sur le lien ci-dessous (une seule fois) :</p>
      <p><a href="${escapeHtml(confirmUrl)}">${escapeHtml(confirmUrl)}</a></p>
      <p>Le lien est valable 72 heures. Si vous n’êtes pas à l’origine de ce message, ignorez-le.</p>
    `,
  });
}

export async function deliverConfirmedRentreeSubmission(
  meta: RentreePendingSubmission,
  fileBytes: Buffer,
): Promise<void> {
  const mail = await mailer();
  if (!mail) throw new Error("SMTP non configuré");
  const school = await schoolLabel();
  const studentLine = meta.studentName?.trim()
    ? `Élève : ${meta.studentName.trim()}`
    : "Élève : non renseigné";

  await mail.transporter.sendMail({
    from: `"${school}" <${mail.fromAddr}>`,
    to: meta.recipientEmails,
    replyTo: meta.senderEmail,
    subject: `Rentrée — ${meta.itemTitle} (${meta.establishmentLabel})`,
    text: [
      `Un document a été déposé depuis la page rentrée « ${meta.establishmentLabel} ».`,
      `Rubrique : ${meta.itemTitle}`,
      studentLine,
      `E-mail de l’expéditeur : ${meta.senderEmail}`,
      `Fichier : ${meta.fileName}`,
      "",
      "La pièce jointe est jointe à ce message. Vous pouvez répondre directement à l’expéditeur.",
    ].join("\n"),
    html: `
      <p>Un document a été déposé depuis la page rentrée <strong>${escapeHtml(meta.establishmentLabel)}</strong>.</p>
      <p><strong>Intitulé :</strong> ${escapeHtml(meta.itemTitle)}<br/>
      <strong>${escapeHtml(studentLine)}</strong><br/>
      <strong>Expéditeur :</strong> ${escapeHtml(meta.senderEmail)}<br/>
      <strong>Fichier :</strong> ${escapeHtml(meta.fileName)}</p>
      <p>La pièce est jointe à ce message. Vous pouvez répondre directement à l’expéditeur.</p>
    `,
    attachments: [
      {
        filename: sanitizeRequestFileName(meta.fileName),
        content: fileBytes,
        contentType: meta.contentType || "application/octet-stream",
      },
    ],
  });

  try {
    await mail.transporter.sendMail({
      from: `"${school}" <${mail.fromAddr}>`,
      to: meta.senderEmail,
      subject: `Document bien transmis — ${meta.itemTitle}`,
      text: [
        "Bonjour,",
        "",
        `Votre document « ${meta.itemTitle} » a bien été transmis à l’établissement.`,
        studentLine,
        `Fichier : ${meta.fileName}`,
        "",
        "Vous n’avez rien d’autre à faire.",
      ].join("\n"),
    });
  } catch (e) {
    console.error("[rentree/submissions] ack parent:", e);
  }
}

export async function rentreeSubmissionConfirmUrl(token: string): Promise<string> {
  return tenantAbsolutePath(`/api/rentree/submissions/confirm?token=${encodeURIComponent(token)}`);
}

export async function rentreeSubmissionResultUrl(query: Record<string, string>): Promise<string> {
  const base = await tenantAbsolutePath("/rentree/depot/confirme");
  const u = new URL(base);
  for (const [k, v] of Object.entries(query)) u.searchParams.set(k, v);
  return u.toString();
}
