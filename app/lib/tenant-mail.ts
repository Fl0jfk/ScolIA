import "server-only";
import nodemailer from "nodemailer";
import { getTenant } from "@/app/lib/tenant-context";

type MailTransporter = ReturnType<typeof nodemailer.createTransport>;

/**
 * Config SMTP plateforme / tenant.
 * - `user` = adresse From (MAILER_EMAIL) — utilisée partout en `from: <${smtp.user}>`.
 * - `authUser` = username SMTP si différent du From (TEM : Project ID).
 */
type TenantSmtpConfig = {
  user: string;
  pass: string;
  host: string;
  port?: number;
  secure?: boolean;
  /** Username auth SMTP quand ≠ From (Scaleway TEM). */
  authUser?: string;
};

function parsePort(raw: string | undefined, fallback: number): number {
  if (!raw?.trim()) return fallback;
  const n = Number(raw.trim());
  return Number.isFinite(n) && n > 0 ? n : fallback;
}

function parseSecure(raw: string | undefined, port: number): boolean {
  if (raw?.trim()) return raw.trim() !== "false" && raw.trim() !== "0";
  // TLS implicite : SMTP 465/2465, IMAP 993. STARTTLS (587…) → false.
  return port === 465 || port === 2465 || port === 993;
}

/** Adresse unique plateforme : mailer@scolia.fr (From + boîte IMAP). */
export function platformMailerAddress(): string | null {
  const raw =
    process.env.MAILER_EMAIL?.trim() ||
    process.env.TRAVEL_INBOUND_EMAIL?.trim() || // legacy
    process.env.SMTP_USER?.trim() ||
    "";
  return raw || null;
}

/**
 * Mot de passe boîte OVH (IMAP / legacy SMTP unifié).
 * Ne jamais confondre avec MAILER_SMTP_PASS (secret IAM TEM).
 */
function platformMailerPass(): string | null {
  const raw =
    process.env.MAILER_PASS?.trim() ||
    process.env.TRAVEL_IMAP_PASS?.trim() || // legacy
    process.env.IMAP_PASS?.trim() ||
    "";
  return raw || null;
}

/** Host IMAP / legacy partagé (OVH), sans confondre avec MAILER_SMTP_HOST (TEM). */
function platformMailerHost(): string | null {
  const raw =
    process.env.MAILER_HOST?.trim() ||
    process.env.TRAVEL_IMAP_HOST?.trim() || // legacy
    process.env.IMAP_HOST?.trim() ||
    process.env.SMTP_HOST?.trim() ||
    "";
  return raw || null;
}

function isTemSmtpHost(host: string): boolean {
  return /tem\.scaleway\.com$/i.test(host) || /smtp\.tem\./i.test(host);
}

/**
 * SMTP plateforme.
 * Priorité TEM (Serverless Containers) :
 *   MAILER_SMTP_HOST + MAILER_SMTP_USER + MAILER_SMTP_PASS + MAILER_EMAIL (From)
 * Fallback legacy OVH unifié :
 *   MAILER_HOST + MAILER_PASS + MAILER_EMAIL
 */
export function getPlatformSmtpConfig(): TenantSmtpConfig | null {
  const from = platformMailerAddress();
  if (!from) return null;

  const smtpHost =
    process.env.MAILER_SMTP_HOST?.trim() ||
    process.env.SMTP_HOST?.trim() ||
    "";
  const smtpUser = process.env.MAILER_SMTP_USER?.trim() || "";
  const smtpPass =
    process.env.MAILER_SMTP_PASS?.trim() ||
    process.env.SMTP_PASS?.trim() ||
    "";

  // TEM (ou tout SMTP avec auth ≠ From)
  if (smtpHost && smtpUser && smtpPass) {
    const port = parsePort(
      process.env.MAILER_SMTP_PORT?.trim() || process.env.SMTP_PORT?.trim(),
      isTemSmtpHost(smtpHost) ? 587 : 465,
    );
    const secure = parseSecure(
      process.env.MAILER_SMTP_SECURE?.trim() || process.env.SMTP_SECURE?.trim(),
      port,
    );
    return {
      user: from,
      authUser: smtpUser,
      pass: smtpPass,
      host: smtpHost,
      port,
      secure,
    };
  }

  // Legacy : même host/creds pour SMTP (Instances / hors Scaleway Containers)
  const host = smtpHost || platformMailerHost() || "";
  const pass = platformMailerPass() || process.env.SMTP_PASS?.trim() || "";
  if (!host || !pass) return null;

  const port = parsePort(
    process.env.MAILER_SMTP_PORT?.trim() || process.env.SMTP_PORT?.trim(),
    465,
  );
  const secure = parseSecure(
    process.env.MAILER_SMTP_SECURE?.trim() || process.env.SMTP_SECURE?.trim(),
    port,
  );
  return { user: from, pass, host, port, secure };
}

type PlatformImapConfig = {
  host: string;
  port: number;
  secure: boolean;
  user: string;
  pass: string;
};

/** IMAP = boîte OVH uniquement (jamais les creds TEM). */
export function getPlatformImapConfig(): PlatformImapConfig | null {
  const user =
    process.env.MAILER_IMAP_USER?.trim() ||
    platformMailerAddress() ||
    process.env.TRAVEL_IMAP_USER?.trim() ||
    process.env.IMAP_USER?.trim() ||
    "";
  const pass = platformMailerPass();
  const host =
    process.env.MAILER_IMAP_HOST?.trim() ||
    process.env.MAILER_HOST?.trim() ||
    process.env.TRAVEL_IMAP_HOST?.trim() ||
    process.env.IMAP_HOST?.trim() ||
    "";
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

/** Timeouts courts : un SMTP qui ne répond pas ne doit pas bloquer l’API (Load failed). */
const SMTP_CONNECT_MS = 8_000;
const SMTP_SOCKET_MS = 12_000;
/** Envoi réservations salles : au-delà, on renvoie quand même le succès métier. */
const ROOM_MAIL_TIMEOUT_MS = 10_000;

function createTransportFromConfig(smtp: TenantSmtpConfig): MailTransporter {
  return nodemailer.createTransport({
    host: smtp.host,
    port: smtp.port ?? 465,
    secure: smtp.secure ?? true,
    auth: {
      user: smtp.authUser ?? smtp.user,
      pass: smtp.pass,
    },
    connectionTimeout: SMTP_CONNECT_MS,
    greetingTimeout: SMTP_CONNECT_MS,
    socketTimeout: SMTP_SOCKET_MS,
  });
}

/**
 * sendMail avec délai max. En cas de timeout, reject avec un message explicite.
 */
export async function sendMailWithTimeout(
  transporter: MailTransporter,
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  mail: any,
  timeoutMs = ROOM_MAIL_TIMEOUT_MS,
): Promise<void> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    await Promise.race([
      transporter.sendMail(mail).then(() => undefined),
      new Promise<never>((_, reject) => {
        timer = setTimeout(
          () =>
            reject(
              new Error(
                `SMTP timeout après ${timeoutMs}ms (host injoignable ou bloqué depuis le runtime)`,
              ),
            ),
          timeoutMs,
        );
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
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

function isTenantSmtpConfigured(): Promise<boolean> {
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

/** Adresse From (jamais le Project ID TEM). */
export async function getTenantSmtpFromAddress(): Promise<string | null> {
  const smtp = await getTenantSmtpConfig();
  return smtp?.user ?? platformMailerAddress();
}
