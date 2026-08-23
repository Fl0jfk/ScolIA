import "server-only";

import { createHmac, timingSafeEqual } from "crypto";
import type { BienEtreSession } from "@/app/lib/bien-etre-types";

const COOKIE_NAME = "bien_etre_session";
const MAX_MESSAGES = 24;
const TTL_MS = 2 * 60 * 60 * 1000;

function sessionSecret(): string {
  return (
    process.env.BIEN_ETRE_SESSION_SECRET?.trim() ||
    process.env.AUTH_PARENT_SECRET_FALLBACK?.trim() ||
    "dev-bien-etre-session-secret"
  );
}

export function sealBienEtreSession(session: BienEtreSession): string {
  const payload = Buffer.from(
    JSON.stringify({
      ...session,
      messages: session.messages.slice(-MAX_MESSAGES),
      exp: Date.now() + TTL_MS,
    }),
  ).toString("base64url");
  const sig = createHmac("sha256", sessionSecret()).update(payload).digest("base64url");
  return `${payload}.${sig}`;
}

export function openBienEtreSession(sealed: string | undefined | null): BienEtreSession | null {
  if (!sealed?.includes(".")) return null;
  const dot = sealed.lastIndexOf(".");
  const payload = sealed.slice(0, dot);
  const sig = sealed.slice(dot + 1);
  const expected = createHmac("sha256", sessionSecret()).update(payload).digest("base64url");
  try {
    const a = Buffer.from(sig);
    const b = Buffer.from(expected);
    if (a.length !== b.length || !timingSafeEqual(a, b)) return null;
  } catch {
    return null;
  }
  try {
    const data = JSON.parse(Buffer.from(payload, "base64url").toString("utf8")) as BienEtreSession & {
      exp?: number;
    };
    if (!data.exp || data.exp < Date.now()) return null;
    if (!Array.isArray(data.messages)) return null;
    return {
      messages: data.messages.slice(-MAX_MESSAGES),
      analysis: data.analysis,
      updatedAt: data.updatedAt || new Date().toISOString(),
    };
  } catch {
    return null;
  }
}

export function emptyBienEtreSession(): BienEtreSession {
  return { messages: [], updatedAt: new Date().toISOString() };
}

export function bienEtreSessionCookieOptions(sealed: string) {
  return {
    name: COOKIE_NAME,
    value: sealed,
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax" as const,
    path: "/",
    maxAge: TTL_MS / 1000,
  };
}

export function bienEtreSessionCookieName(): string {
  return COOKIE_NAME;
}

export function clearBienEtreSessionCookieOptions() {
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
