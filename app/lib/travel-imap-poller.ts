import { createHash } from "crypto";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { ImapFlow } from "imapflow";
import { simpleParser, type Attachment, type ParsedMail } from "mailparser";
import { getDataS3ClientForTenantSlug, getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getTenantBucketName } from "@/app/lib/tenant-config";
import { getPlatformImapConfig } from "@/app/lib/tenant-mail";
import { resolveTenantBySlug } from "@/app/lib/tenant-registry";
import { ingestTravelEmail, type TravelEmailIngestResult } from "@/app/lib/travel-email-ingest";
import { extractTenantSlugFromEmailHeaders } from "@/app/lib/travel-email-routing";

export type TravelEmailPollSummary = {
  scanned: number;
  processed: number;
  unmatched: number;
  pending: number;
  errors: Array<{ messageId?: string; file?: string; err: string }>;
};

export type TravelImapConfig = NonNullable<ReturnType<typeof getPlatformImapConfig>>;

/** Config IMAP = MAILER_* (même boîte que l'envoi SMTP). */
export function getTravelImapConfig(): TravelImapConfig | null {
  return getPlatformImapConfig();
}

export function travelImapConfigured(): boolean {
  return getTravelImapConfig() != null;
}

function countAsUnmatched(result: TravelEmailIngestResult): boolean {
  return result.completed && !result.failed && !result.matched;
}

function headerList(value: ParsedMail["to"] | ParsedMail["cc"] | string | undefined): string[] {
  if (!value) return [];
  if (typeof value === "string") return [value];
  if (Array.isArray(value)) {
    return value.flatMap((v) => {
      if (!v) return [];
      if (typeof v === "string") return [v];
      if (typeof v === "object" && "text" in v && typeof (v as { text?: string }).text === "string") {
        return [(v as { text: string }).text];
      }
      if (typeof v === "object" && "value" in v) {
        const arr = (v as { value?: Array<{ address?: string }> }).value;
        return (arr || []).map((a) => a.address || "").filter(Boolean);
      }
      return [];
    });
  }
  if (typeof value === "object" && value && "text" in value) {
    return [String((value as { text: string }).text || "")].filter(Boolean);
  }
  return [];
}

function fromAddress(parsed: ParsedMail): string {
  const v = parsed.from;
  if (!v) return "";
  if (typeof v === "object" && "value" in v) {
    const first = (v as { value?: Array<{ address?: string }> }).value?.[0]?.address;
    if (first) return first.trim();
  }
  const text = typeof v === "object" && "text" in v ? String((v as { text?: string }).text || "") : "";
  const m = text.match(/<([^>]+)>/);
  return (m?.[1] || text).trim();
}

function stableMessageId(parsed: ParsedMail, uid: number): string {
  const mid = (parsed.messageId || "").trim().replace(/^<|>$/g, "");
  if (mid) {
    return createHash("sha256").update(mid).digest("hex").slice(0, 32);
  }
  return `imap-uid-${uid}`;
}

function isPdfAttachment(att: Attachment): boolean {
  const name = att.filename || "";
  const mime = (att.contentType || "").toLowerCase();
  if (/\.pdf$/i.test(name)) return true;
  return mime === "application/pdf" || mime === "application/x-pdf";
}

async function resolveUploadTarget(slugHint: string | null) {
  if (slugHint) {
    const tenant = await resolveTenantBySlug(slugHint);
    if (tenant) {
      return {
        client: await getDataS3ClientForTenantSlug(tenant.slug),
        bucket: tenant.dataBucket,
        tenantSlugHint: tenant.slug,
      };
    }
  }
  return {
    client: await getTenantDataS3Client(),
    bucket: await getTenantBucketName(),
    tenantSlugHint: slugHint,
  };
}

/**
 * Poll IMAP (UNSEEN), upload PDF → Object Storage, ingest synchrone (Mistral).
 * Marque SEEN seulement si safeToMarkRead.
 */
export async function pollTravelImapInbox(options?: {
  maxMessages?: number;
}): Promise<TravelEmailPollSummary> {
  const maxMessages = options?.maxMessages ?? 25;
  const cfg = getTravelImapConfig();
  if (!cfg) {
    throw new Error(
      "IMAP non configuré. Définir MAILER_EMAIL, MAILER_PASS, MAILER_HOST (boîte OVH mailer@…).",
    );
  }

  const summary: TravelEmailPollSummary = {
    scanned: 0,
    processed: 0,
    unmatched: 0,
    pending: 0,
    errors: [],
  };

  const client = new ImapFlow({
    host: cfg.host,
    port: cfg.port,
    secure: cfg.secure,
    auth: { user: cfg.user, pass: cfg.pass },
    logger: false,
  });

  await client.connect();
  try {
    const lock = await client.getMailboxLock("INBOX");
    try {
      const searchResult = await client.search({ seen: false }, { uid: true });
      const uids = (Array.isArray(searchResult) ? searchResult : [])
        .map((u) => Number(u))
        .filter((u) => Number.isFinite(u) && u > 0)
        .slice(0, maxMessages);
      summary.scanned = uids.length;

      for (const uid of uids) {
        const messageIdLabel = `uid-${uid}`;
        try {
          const downloaded = await client.download(uid, undefined, { uid: true });
          if (!downloaded?.content) {
            summary.errors.push({ messageId: messageIdLabel, err: "download_vide" });
            continue;
          }
          const parsed = await simpleParser(downloaded.content);
          const emailMessageId = stableMessageId(parsed, uid);
          const fromEmail = fromAddress(parsed);
          const subject = parsed.subject || "";
          const emailBodyPlain = (parsed.text || "").trim();
          const snippet = emailBodyPlain.slice(0, 500) || subject;

          const deliveredTo = parsed.headers?.get("delivered-to");
          const xOriginalTo = parsed.headers?.get("x-original-to");
          const slugHint = extractTenantSlugFromEmailHeaders([
            ...headerList(parsed.to),
            ...headerList(parsed.cc),
            typeof deliveredTo === "string" ? deliveredTo : null,
            typeof xOriginalTo === "string" ? xOriginalTo : null,
          ]);

          const uploadTarget = await resolveUploadTarget(slugHint);
          const pdfAtts = (parsed.attachments || []).filter(isPdfAttachment);

          let allOk = true;
          let anyUnmatched = false;

          if (pdfAtts.length === 0) {
            const ingestResult = await ingestTravelEmail({
              fromEmail,
              subject,
              snippet,
              emailBodyPlain: emailBodyPlain || snippet,
              gmailMessageId: emailMessageId,
              tenantSlugHint: uploadTarget.tenantSlugHint,
            });
            if (ingestResult.pending) {
              summary.pending += 1;
              allOk = false;
            } else if (!ingestResult.safeToMarkRead) {
              allOk = false;
              summary.errors.push({
                messageId: emailMessageId,
                err: ingestResult.reason || ingestResult.detail || "ingest_texte_echec",
              });
            } else if (countAsUnmatched(ingestResult)) {
              anyUnmatched = true;
            }
          } else {
            for (const att of pdfAtts) {
              const buf = Buffer.isBuffer(att.content)
                ? att.content
                : Buffer.from(att.content as Uint8Array);
              if (buf.length === 0) {
                allOk = false;
                summary.errors.push({
                  messageId: emailMessageId,
                  file: att.filename || "file.pdf",
                  err: "piece_vide",
                });
                break;
              }

              const safeName = (att.filename || "document.pdf")
                .replace(/[/\\]/g, "_")
                .replace(/\s+/g, "_")
                .replace(/\.\./g, "_");
              const attId = createHash("sha256")
                .update(`${emailMessageId}:${safeName}:${buf.length}`)
                .digest("hex")
                .slice(0, 16);
              const s3Key = `devis-incoming/${emailMessageId}/${attId}_${safeName}`;

              await uploadTarget.client.send(
                new PutObjectCommand({
                  Bucket: uploadTarget.bucket,
                  Key: s3Key,
                  Body: buf,
                  ContentType: "application/pdf",
                }),
              );

              const ingestResult = await ingestTravelEmail({
                s3Key,
                fromEmail,
                subject,
                snippet,
                emailBodyPlain: emailBodyPlain || snippet,
                gmailMessageId: emailMessageId,
                originalFilename: att.filename || safeName,
                tenantSlugHint: uploadTarget.tenantSlugHint,
              });

              if (ingestResult.pending) {
                summary.pending += 1;
                allOk = false;
                break;
              }
              if (!ingestResult.safeToMarkRead) {
                allOk = false;
                summary.errors.push({
                  messageId: emailMessageId,
                  file: att.filename || safeName,
                  err: ingestResult.reason || ingestResult.detail || "ingest_echec",
                });
                break;
              }
              if (countAsUnmatched(ingestResult)) anyUnmatched = true;
            }
          }

          if (allOk) {
            await client.messageFlagsAdd(uid, ["\\Seen"], { uid: true });
            summary.processed += 1;
            if (anyUnmatched) summary.unmatched += 1;
          }
        } catch (e) {
          summary.errors.push({
            messageId: messageIdLabel,
            err: e instanceof Error ? e.message : String(e),
          });
        }
      }
    } finally {
      lock.release();
    }
  } finally {
    try {
      await client.logout();
    } catch {
      /* ignore */
    }
  }

  return summary;
}
