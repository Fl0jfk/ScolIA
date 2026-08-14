import "server-only";
import { getTenant } from "@/app/lib/tenant-context";
import { platformMailerAddress } from "@/app/lib/tenant-mail";

/**
 * Boîte plateforme (MAILER_EMAIL) — polling + Reply-To mailer+{slug}@…
 */
function travelInboundMailbox(): string | null {
  return platformMailerAddress();
}

/** Alerte si un mail sorties n'est pas rattaché. */
export function travelEmailAlertRecipients(): string[] {
  const raw =
    process.env.MAILER_ALERT_TO?.trim() ||
    process.env.TRAVEL_EMAIL_ALERT_TO?.trim() || // legacy
    "";
  if (!raw) return [];
  return raw
    .split(/[,;]/)
    .map((s) => s.trim())
    .filter(Boolean);
}

function parseEmailAddress(raw: string): { local: string; domain: string } | null {
  const m = raw.match(/([a-zA-Z0-9._%+-]+)@([a-zA-Z0-9.-]+\.[a-zA-Z]{2,})/);
  if (!m) return null;
  return { local: m[1], domain: m[2].toLowerCase() };
}

/**
 * Reply-To pour que les réponses transporteurs conservent le slug tenant
 * (plus-addressing : transport+mon-slug@scolia.fr → même boîte transport@…).
 */
export async function buildTransportReplyTo(slugOverride?: string): Promise<string | null> {
  const mailbox = travelInboundMailbox();
  if (!mailbox) return null;
  const parsed = parseEmailAddress(mailbox);
  if (!parsed) return null;

  let slug = (slugOverride || "").trim().toLowerCase().replace(/[^a-z0-9-]/g, "");
  if (!slug) {
    try {
      const tenant = await getTenant();
      slug = tenant.slug.trim().toLowerCase().replace(/[^a-z0-9-]/g, "");
    } catch {
      return mailbox;
    }
  }
  if (!slug) return mailbox;
  return `${parsed.local}+${slug}@${parsed.domain}`;
}

/**
 * Extrait un slug tenant depuis To / Delivered-To / X-Original-To / Cc, etc.
 * Formats : transport+slug@domaine ou "Name <transport+slug@domaine>"
 */
export function extractTenantSlugFromEmailHeaders(headerValues: Array<string | null | undefined>): string | null {
  const mailbox = travelInboundMailbox();
  const mailboxLocal = mailbox ? parseEmailAddress(mailbox)?.local.toLowerCase() : null;

  for (const raw of headerValues) {
    if (!raw) continue;
    const parts = raw.split(/[,;]/);
    for (const part of parts) {
      const parsed = parseEmailAddress(part);
      if (!parsed) continue;
      const local = parsed.local;
      const plusIdx = local.indexOf("+");
      if (plusIdx <= 0) continue;
      const base = local.slice(0, plusIdx).toLowerCase();
      const tag = local
        .slice(plusIdx + 1)
        .toLowerCase()
        .replace(/[^a-z0-9-]/g, "");
      if (!tag) continue;
      if (mailboxLocal && base !== mailboxLocal) continue;
      return tag;
    }
  }
  return null;
}
