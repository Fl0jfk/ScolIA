import "server-only";
import nodemailer from "nodemailer";
import { getTenant } from "@/app/lib/tenant-context";

type MailTransporter = ReturnType<typeof nodemailer.createTransport>;

export type TenantSmtpConfig = {
  user: string;
  pass: string;
  host: string;
  port?: number;
  secure?: boolean;
};

function parsePort(raw: string | undefined, fallback: number): number {
  if (!raw?.trim()) return fallback;
  const n = Number(raw.trim());
  return Number.isFinite(n) && n > 0 ? n : fallback;
}

function parseSecure(raw: string | undefined, port: number): boolean {
  if (raw?.trim()) return raw.trim() !== "false" && raw.trim() !== "0";
  return port === 465;
}

/** Adresse unique plateforme : mailer@scolia.fr */
export function platformMailerAddress(): string | null {
  const raw =
    process.env.MAILER_EMAIL?.trim() ||
    process.env.TRAVEL_INBOUND_EMAIL?.trim() || // legacy
    process.env.SMTP_USER?.trim() ||
    "";
  return raw || null;
}

/** Mot de passe boîte (SMTP + IMAP). */
export function platformMailerPass(): string | null {
  const raw =
    process.env.MAILER_PASS?.trim() ||
    process.env.SMTP_PASS?.trim() ||
    process.env.TRAVEL_IMAP_PASS?.trim() || // legacy
    process.env.IMAP_PASS?.trim() ||
    "";
  return raw || null;
}

/**
 * Host OVH partagé (souvent ssl0.ovh.net pour SMTP et IMAP).
 * Surcharges optionnelles : MAILER_SMTP_HOST / MAILER_IMAP_HOST.
 */
export function platformMailerHost(): string | null {
  const raw =
    process.env.MAILER_HOST?.trim() ||
    process.env.MAILER_SMTP_HOST?.trim() ||
    process.env.SMTP_HOST?.trim() ||
    process.env.TRAVEL_IMAP_HOST?.trim() || // legacy
    process.env.IMAP_HOST?.trim() ||
    "";
  return raw || null;
}

export function getPlatformSmtpConfig(): TenantSmtpConfig | null {
  const user = platformMailerAddress();
  const pass = platformMailerPass();
  const host =
    process.env.MAILER_SMTP_HOST?.trim() || platformMailerHost() || "";
  if (!user || !pass || !host) return null;

  const port = parsePort(process.env.MAILER_SMTP_PORT?.trim() || process.env.SMTP_PORT?.trim(), 465);
  const secure = parseSecure(process.env.MAILER_SMTP_SECURE?.trim() || process.env.SMTP_SECURE?.trim(), port);
  return { user, pass, host, port, secure };
}

export type PlatformImapConfig = {
  host: string;
  port: number;
  secure: boolean;
  user: string;
  pass: string;
};

export function getPlatformImapConfig(): PlatformImapConfig | null {
  const user =
    process.env.MAILER_IMAP_USER?.trim() ||
    platformMailerAddress() ||
    process.env.TRAVEL_IMAP_USER?.trim() ||
    process.env.IMAP_USER?.trim() ||
    "";
  const pass = platformMailerPass();
  const host =
    process.env.MAILER_IMAP_HOST?.trim() || platformMailerHost() || "";
  if (!user || !pass || !host) return null;

  const port = parsePort(
    process.env.MAILER_IMAP_PORT?.trim() ||
      process.env.TRAVEL_IMAP_PORT?.trim() ||
      process.env.IMAP_PORT?.trim(),
    993,
  );
  const secure = parseSecure(
    process.env.MAILER_IMAP_SECURE?.trim() || process.env.TRAVEL_IMAP_SECURE?.trim(),
    port,
  );
  return { host, port, secure, user, pass };
}

function createTransportFromConfig(smtp: TenantSmtpConfig): MailTransporter {
  return nodemailer.createTransport({
    host: smtp.host,
    port: smtp.port ?? 465,
    secure: smtp.secure ?? true,
    auth: { user: smtp.user, pass: smtp.pass },
  });
}

/**
 * SMTP effectif : plateforme MAILER_* en priorité, sinon SMTP tenant (host obligatoire).
 */
export async function getTenantSmtpConfig(): Promise<TenantSmtpConfig | null> {
  const platform = getPlatformSmtpConfig();
  if (platform) return platform;

  try {
    const tenant = await getTenant();
    const smtp = tenant.secrets?.smtp;
    if (smtp?.user?.trim() && smtp?.pass?.trim() && smtp?.host?.trim()) {
      return {
        user: smtp.user.trim(),
        pass: smtp.pass.trim(),
        host: smtp.host.trim(),
        port: 465,
        secure: true,
      };
    }
  } catch {
    /* pas de contexte tenant */
  }
  return null;
}

export function isTenantSmtpConfigured(): Promise<boolean> {
  return getTenantSmtpConfig().then(Boolean);
}

export async function createTenantTransporter(): Promise<MailTransporter | null> {
  const smtp = await getTenantSmtpConfig();
  if (!smtp) return null;
  return createTransportFromConfig(smtp);
}

export function createPlatformTransporter(): MailTransporter | null {
  const smtp = getPlatformSmtpConfig();
  if (!smtp) return null;
  return createTransportFromConfig(smtp);
}

export async function getTenantSmtpFromAddress(): Promise<string | null> {
  const smtp = await getTenantSmtpConfig();
  return smtp?.user ?? null;
}
