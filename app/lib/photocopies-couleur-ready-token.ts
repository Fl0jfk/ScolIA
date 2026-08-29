import "server-only";

import { createHmac, timingSafeEqual } from "crypto";

const TTL_MS = 30 * 24 * 60 * 60 * 1000;

type ReadyTokenPayload = {
  id: string;
  action: "mark_ready";
  exp: number;
};

function tokenSecret(): string {
  return (
    process.env.PHOTOCOPIES_READY_TOKEN_SECRET?.trim() ||
    process.env.BETTER_AUTH_SECRET?.trim() ||
    process.env.AUTH_SECRET?.trim() ||
    "dev-photocopies-ready-token-secret"
  );
}

export function sealPhotocopieReadyToken(id: string): string {
  const payload = Buffer.from(
    JSON.stringify({
      id,
      action: "mark_ready",
      exp: Date.now() + TTL_MS,
    } satisfies ReadyTokenPayload),
  ).toString("base64url");
  const sig = createHmac("sha256", tokenSecret()).update(payload).digest("base64url");
  return `${payload}.${sig}`;
}

export function openPhotocopieReadyToken(sealed: string | undefined | null): ReadyTokenPayload | null {
  if (!sealed?.includes(".")) return null;
  const dot = sealed.lastIndexOf(".");
  const payload = sealed.slice(0, dot);
  const sig = sealed.slice(dot + 1);
  const expected = createHmac("sha256", tokenSecret()).update(payload).digest("base64url");
  try {
    const a = Buffer.from(sig);
    const b = Buffer.from(expected);
    if (a.length !== b.length || !timingSafeEqual(a, b)) return null;
  } catch {
    return null;
  }
  try {
    const data = JSON.parse(Buffer.from(payload, "base64url").toString("utf8")) as ReadyTokenPayload;
    if (!data.id || data.action !== "mark_ready") return null;
    if (!data.exp || data.exp < Date.now()) return null;
    return data;
  } catch {
    return null;
  }
}
