import "server-only";

import { createHash, createHmac, randomInt, timingSafeEqual } from "crypto";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";
import { normalizeParentEmail } from "@/app/lib/eleves-parent-emails";

const COOKIE_NAME = "class_allocation_parent_session";
const SESSION_TTL_MS = 30 * 60 * 1000;
const CODE_TTL_MS = 15 * 60 * 1000;
const MAX_VERIFY_ATTEMPTS = 5;
const RESEND_COOLDOWN_MS = 60 * 1000;

export type ClassAllocationParentSession = {
  email: string;
  campaignId: string;
  childInes: string[];
  verifiedAt: string;
};

type PendingParentAuth = {
  email: string;
  campaignId: string;
  codeHash: string;
  expiresAt: string;
  attempts: number;
  lastSentAt: string;
};

function authSecret(): string {
  const secret =
    process.env.CLASS_ALLOCATION_PARENT_SECRET?.trim() ||
    process.env.CLERK_ENCRYPTION_KEY?.trim() ||
    "";
  if (secret) return secret;
  if (process.env.NODE_ENV === "production") {
    throw new Error("CLASS_ALLOCATION_PARENT_SECRET (ou CLERK_ENCRYPTION_KEY) requis en production.");
  }
  return "dev-class-allocation-parent-secret";
}

function pendingKey(campaignId: string, email: string): string {
  const hash = createHash("sha256").update(`${campaignId}:${normalizeParentEmail(email)}`).digest("hex");
  return `class-allocation/parent-auth/${campaignId}/${hash}.json`;
}

function hashCode(email: string, campaignId: string, code: string): string {
  return createHmac("sha256", authSecret())
    .update(`${campaignId}:${normalizeParentEmail(email)}:${code}`)
    .digest("hex");
}

export function generateParentAuthCode(): string {
  return String(randomInt(0, 1_000_000)).padStart(6, "0");
}

export async function loadPendingParentAuth(
  campaignId: string,
  email: string,
): Promise<PendingParentAuth | null> {
  const hit = await getJson<PendingParentAuth>(pendingKey(campaignId, email));
  return hit?.data ?? null;
}

export async function savePendingParentAuth(record: PendingParentAuth): Promise<void> {
  await putJson(pendingKey(record.campaignId, record.email), record);
}

export async function clearPendingParentAuth(campaignId: string, email: string): Promise<void> {
  await savePendingParentAuth({
    email: normalizeParentEmail(email),
    campaignId,
    codeHash: "",
    expiresAt: new Date(0).toISOString(),
    attempts: MAX_VERIFY_ATTEMPTS,
    lastSentAt: new Date(0).toISOString(),
  });
}

export async function issueParentAuthCode(params: {
  campaignId: string;
  email: string;
}): Promise<{ ok: true; code: string } | { ok: false; reason: "cooldown"; retryAfterSec: number }> {
  const email = normalizeParentEmail(params.email);
  const existing = await loadPendingParentAuth(params.campaignId, email);
  if (existing?.lastSentAt) {
    const elapsed = Date.now() - new Date(existing.lastSentAt).getTime();
    if (elapsed < RESEND_COOLDOWN_MS) {
      return {
        ok: false,
        reason: "cooldown",
        retryAfterSec: Math.ceil((RESEND_COOLDOWN_MS - elapsed) / 1000),
      };
    }
  }
  const code = generateParentAuthCode();
  await savePendingParentAuth({
    email,
    campaignId: params.campaignId,
    codeHash: hashCode(email, params.campaignId, code),
    expiresAt: new Date(Date.now() + CODE_TTL_MS).toISOString(),
    attempts: 0,
    lastSentAt: new Date().toISOString(),
  });
  return { ok: true, code };
}

export async function verifyParentAuthCode(params: {
  campaignId: string;
  email: string;
  code: string;
}): Promise<{ ok: true } | { ok: false; error: string }> {
  const email = normalizeParentEmail(params.email);
  const code = String(params.code || "").trim();
  if (!/^\d{6}$/.test(code)) {
    return { ok: false, error: "Code invalide (6 chiffres)." };
  }
  const pending = await loadPendingParentAuth(params.campaignId, email);
  if (!pending?.codeHash || new Date(pending.expiresAt).getTime() < Date.now()) {
    return { ok: false, error: "Code expiré ou inconnu. Demandez un nouveau code." };
  }
  if (pending.attempts >= MAX_VERIFY_ATTEMPTS) {
    return { ok: false, error: "Trop de tentatives. Demandez un nouveau code." };
  }
  const expected = pending.codeHash;
  const actual = hashCode(email, params.campaignId, code);
  const a = Buffer.from(actual);
  const b = Buffer.from(expected);
  const match = a.length === b.length && timingSafeEqual(a, b);
  if (!match) {
    await savePendingParentAuth({
      ...pending,
      attempts: pending.attempts + 1,
    });
    return { ok: false, error: "Code incorrect." };
  }
  await clearPendingParentAuth(params.campaignId, email);
  return { ok: true };
}

export function sealParentSession(session: ClassAllocationParentSession): string {
  const payload = Buffer.from(
    JSON.stringify({
      ...session,
      email: normalizeParentEmail(session.email),
      exp: Date.now() + SESSION_TTL_MS,
    }),
  ).toString("base64url");
  const sig = createHmac("sha256", authSecret()).update(payload).digest("base64url");
  return `${payload}.${sig}`;
}

export function openParentSession(sealed: string | undefined | null): ClassAllocationParentSession | null {
  if (!sealed?.includes(".")) return null;
  const dot = sealed.lastIndexOf(".");
  const payload = sealed.slice(0, dot);
  const sig = sealed.slice(dot + 1);
  const expected = createHmac("sha256", authSecret()).update(payload).digest("base64url");
  try {
    const a = Buffer.from(sig);
    const b = Buffer.from(expected);
    if (a.length !== b.length || !timingSafeEqual(a, b)) return null;
  } catch {
    return null;
  }
  try {
    const data = JSON.parse(Buffer.from(payload, "base64url").toString("utf8")) as ClassAllocationParentSession & {
      exp?: number;
    };
    if (!data.exp || data.exp < Date.now()) return null;
    if (!data.email || !data.campaignId || !Array.isArray(data.childInes)) return null;
    return {
      email: normalizeParentEmail(data.email),
      campaignId: data.campaignId,
      childInes: data.childInes.map(String),
      verifiedAt: data.verifiedAt || new Date().toISOString(),
    };
  } catch {
    return null;
  }
}

export function parentSessionCookieName(): string {
  return COOKIE_NAME;
}

export function parentSessionCookieOptions(sealed: string) {
  return {
    name: COOKIE_NAME,
    value: sealed,
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax" as const,
    path: "/",
    maxAge: SESSION_TTL_MS / 1000,
  };
}

export function clearParentSessionCookieOptions() {
  return {
    name: COOKIE_NAME,
    value: "",
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax" as const,
    path: "/",
    maxAge: 0,
  };
}

export async function sendParentAuthCodeEmail(params: {
  to: string;
  code: string;
  schoolName: string;
  campaignLabel: string;
}): Promise<boolean> {
  const smtp = await getTenantSmtpConfig();
  const transporter = await createTenantTransporter();
  if (!smtp || !transporter) return false;
  await transporter.sendMail({
    from: `"${params.schoolName}" <${smtp.user}>`,
    to: params.to,
    subject: `Code de connexion — ${params.campaignLabel}`,
    html: `
      <p>Bonjour,</p>
      <p>Voici votre code pour accéder au formulaire de vœux de répartition des classes (${params.campaignLabel}) :</p>
      <p style="font-size:28px;font-weight:bold;letter-spacing:4px">${params.code}</p>
      <p>Ce code est valable 15 minutes. Si vous n'êtes pas à l'origine de cette demande, ignorez ce message.</p>
      <p>— ${params.schoolName}</p>
    `,
  });
  return true;
}
