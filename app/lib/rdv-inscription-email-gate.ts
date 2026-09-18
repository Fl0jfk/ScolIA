import "server-only";

import { createHmac, randomBytes, timingSafeEqual } from "crypto";
import { and, desc, eq, isNull, lt } from "drizzle-orm";
import { cookies } from "next/headers";
import { getDb } from "@/db/index";
import { rdvInscriptionEmailGate } from "@/db/schema";
import { isValidParentEmail, normalizeParentEmail } from "@/app/lib/eleves-parent-emails";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";
import { escapeHtml } from "@/app/lib/escape-html";
import { getRdvInscriptionDirectionBySlug } from "@/app/lib/rdv-inscription-db";

export const RDV_EMAIL_GATE_COOKIE = "rdv_inscription_email";
export const RDV_EMAIL_GATE_TTL_MS = 2 * 60 * 60 * 1000;
const TOKEN_TTL_MS = 2 * 60 * 60 * 1000;
const RESEND_COOLDOWN_MS = 60 * 1000;

export type RdvEmailGateSession = {
  email: string;
  etablissementId: string;
  directionSlug: string;
  exp: number;
};

function signingSecret(): string {
  const secret =
    process.env.RDV_INSCRIPTION_EMAIL_SECRET?.trim() ||
    process.env.BETTER_AUTH_SECRET?.trim() ||
    process.env.AUTH_SECRET?.trim() ||
    "";
  if (secret) return secret;
  if (process.env.NODE_ENV === "production") {
    throw new Error("BETTER_AUTH_SECRET (ou RDV_INSCRIPTION_EMAIL_SECRET) requis.");
  }
  return "dev-rdv-inscription-email-secret";
}

function b64url(input: string | Buffer): string {
  return Buffer.from(input)
    .toString("base64")
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/g, "");
}

function b64urlJson(value: unknown): string {
  return b64url(JSON.stringify(value));
}

function signPayload(payloadB64: string): string {
  return createHmac("sha256", signingSecret()).update(payloadB64).digest("base64url");
}

export function encodeRdvEmailGateCookie(session: RdvEmailGateSession): string {
  const payload = b64urlJson(session);
  return `v1.${payload}.${signPayload(payload)}`;
}

export function decodeRdvEmailGateCookie(raw: string | undefined | null): RdvEmailGateSession | null {
  const value = String(raw || "").trim();
  if (!value.startsWith("v1.")) return null;
  const parts = value.split(".");
  if (parts.length !== 3) return null;
  const [, payload, sig] = parts;
  if (!payload || !sig) return null;
  const expected = signPayload(payload);
  try {
    const a = Buffer.from(sig);
    const b = Buffer.from(expected);
    if (a.length !== b.length || !timingSafeEqual(a, b)) return null;
  } catch {
    return null;
  }
  try {
    const json = JSON.parse(Buffer.from(payload, "base64url").toString("utf8")) as RdvEmailGateSession;
    if (!json?.email || !json.etablissementId || !json.directionSlug || !json.exp) return null;
    if (json.exp < Date.now()) return null;
    if (!isValidParentEmail(json.email)) return null;
    return {
      email: normalizeParentEmail(json.email),
      etablissementId: json.etablissementId,
      directionSlug: String(json.directionSlug).trim().toLowerCase(),
      exp: Number(json.exp),
    };
  } catch {
    return null;
  }
}

export async function readRdvEmailGateSession(opts?: {
  directionSlug?: string;
}): Promise<RdvEmailGateSession | null> {
  const jar = await cookies();
  const session = decodeRdvEmailGateCookie(jar.get(RDV_EMAIL_GATE_COOKIE)?.value);
  if (!session) return null;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId || session.etablissementId !== etabId) return null;
  if (opts?.directionSlug && session.directionSlug !== opts.directionSlug.trim().toLowerCase()) {
    return null;
  }
  return session;
}

export async function setRdvEmailGateCookie(session: RdvEmailGateSession): Promise<void> {
  const jar = await cookies();
  jar.set(RDV_EMAIL_GATE_COOKIE, encodeRdvEmailGateCookie(session), {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax",
    path: "/",
    maxAge: Math.floor(RDV_EMAIL_GATE_TTL_MS / 1000),
  });
}

export async function clearRdvEmailGateCookie(): Promise<void> {
  const jar = await cookies();
  jar.set(RDV_EMAIL_GATE_COOKIE, "", {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax",
    path: "/",
    maxAge: 0,
  });
}

export async function startRdvEmailGate(opts: {
  directionSlug: string;
  parentEmail: string;
}): Promise<
  | { ok: true; pending: true; mailWarning?: string }
  | { ok: false; status: number; error: string }
> {
  const slug = opts.directionSlug.trim().toLowerCase();
  const email = normalizeParentEmail(opts.parentEmail);
  if (!slug) return { ok: false, status: 400, error: "Direction requise." };
  if (!isValidParentEmail(email)) {
    return { ok: false, status: 400, error: "E-mail invalide." };
  }

  const direction = await getRdvInscriptionDirectionBySlug(slug, { activeOnly: true });
  if (!direction) return { ok: false, status: 404, error: "Direction introuvable." };

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return { ok: false, status: 503, error: "Établissement introuvable." };

  const db = getDb();
  const recent = await db
    .select({ createdAt: rdvInscriptionEmailGate.createdAt })
    .from(rdvInscriptionEmailGate)
    .where(
      and(
        eq(rdvInscriptionEmailGate.etablissementId, etabId),
        eq(rdvInscriptionEmailGate.parentEmail, email),
        eq(rdvInscriptionEmailGate.directionSlug, slug),
      ),
    )
    .orderBy(desc(rdvInscriptionEmailGate.createdAt))
    .limit(1);
  if (recent[0]?.createdAt) {
    const elapsed = Date.now() - recent[0].createdAt.getTime();
    if (elapsed < RESEND_COOLDOWN_MS) {
      return {
        ok: false,
        status: 429,
        error: `Patientez ${Math.ceil((RESEND_COOLDOWN_MS - elapsed) / 1000)} s avant un nouvel envoi.`,
      };
    }
  }

  const token = randomBytes(32).toString("hex");
  const expiresAt = new Date(Date.now() + TOKEN_TTL_MS);
  await db.insert(rdvInscriptionEmailGate).values({
    etablissementId: etabId,
    directionSlug: slug,
    parentEmail: email,
    token,
    expiresAt,
  });

  const verifyUrl = await tenantAbsolutePath(
    `/api/rdv-inscription/email-verify?token=${encodeURIComponent(token)}`,
  );

  const smtp = await getTenantSmtpConfig();
  const transporter = await createTenantTransporter();
  if (!smtp || !transporter) {
    return {
      ok: true,
      pending: true,
      mailWarning: "SMTP non configuré — lien de vérification non envoyé.",
    };
  }

  try {
    await transporter.sendMail({
      from: smtp.user,
      to: email,
      subject: `Confirmez votre e-mail — ${direction.title} — ${direction.label}`,
      html: `
        <p>Bonjour,</p>
        <p>Pour accéder au formulaire de rendez-vous d’inscription (${escapeHtml(direction.label)}),
        confirmez votre adresse e-mail :</p>
        <p style="margin:24px 0;">
          <a href="${escapeHtml(verifyUrl)}"
             style="display:inline-block;background:#0369a1;color:#fff;padding:12px 20px;border-radius:8px;text-decoration:none;font-weight:700;">
            Confirmer mon e-mail
          </a>
        </p>
        <p style="color:#64748b;font-size:13px;">Ce lien expire dans 2 heures.
        Si vous n’êtes pas à l’origine de cette demande, ignorez cet e-mail.</p>
      `,
    });
  } catch (e) {
    return {
      ok: false,
      status: 502,
      error: e instanceof Error ? e.message : "Envoi du mail impossible.",
    };
  }

  return { ok: true, pending: true };
}

export async function verifyRdvEmailGateToken(
  tokenRaw: string,
): Promise<
  | { ok: true; session: RdvEmailGateSession; redirectPath: string }
  | { ok: false; error: string; directionSlug?: string }
> {
  const token = String(tokenRaw || "").trim();
  if (!token || token.length < 20) {
    return { ok: false, error: "Lien invalide." };
  }

  const db = getDb();
  const [row] = await db
    .select()
    .from(rdvInscriptionEmailGate)
    .where(eq(rdvInscriptionEmailGate.token, token))
    .limit(1);
  if (!row) {
    return { ok: false, error: "Lien invalide ou déjà utilisé." };
  }
  if (row.expiresAt.getTime() <= Date.now()) {
    return {
      ok: false,
      error: "Lien expiré. Demandez un nouvel e-mail.",
      directionSlug: row.directionSlug,
    };
  }

  if (!row.verifiedAt) {
    await db
      .update(rdvInscriptionEmailGate)
      .set({ verifiedAt: new Date() })
      .where(eq(rdvInscriptionEmailGate.id, row.id));
  }

  const session: RdvEmailGateSession = {
    email: normalizeParentEmail(row.parentEmail),
    etablissementId: row.etablissementId,
    directionSlug: row.directionSlug,
    exp: Date.now() + RDV_EMAIL_GATE_TTL_MS,
  };
  await setRdvEmailGateCookie(session);

  return {
    ok: true,
    session,
    redirectPath: `/rdv-inscription/${encodeURIComponent(row.directionSlug)}?email_ok=1`,
  };
}

/** Nettoyage opportuniste des tokens expirés non vérifiés. */
export async function pruneExpiredRdvEmailGates(etablissementId: string): Promise<void> {
  try {
    const db = getDb();
    await db
      .delete(rdvInscriptionEmailGate)
      .where(
        and(
          eq(rdvInscriptionEmailGate.etablissementId, etablissementId),
          isNull(rdvInscriptionEmailGate.verifiedAt),
          lt(rdvInscriptionEmailGate.expiresAt, new Date()),
        ),
      );
  } catch {
    /* non bloquant */
  }
}
