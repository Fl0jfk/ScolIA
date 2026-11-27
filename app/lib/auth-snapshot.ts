import "server-only";

import { createHmac, timingSafeEqual } from "node:crypto";

/**
 * Header interne proxy → RSC / API. Il porte l'identité et les rôles : il est signé et daté, car un
 * navigateur peut envoyer n'importe quel en-tête. Sans signature valide, il est ignoré et la session
 * est relue normalement.
 */
export const SCOLA_AUTH_SNAPSHOT_HEADER = "x-scola-auth-snapshot";

export type ScolaAuthSnapshot = {
  authUserId: string;
  userId: string;
  email: string;
  etablissementId: string | null;
  roles: string[];
  orgAdmin: boolean;
  platformAdmin: boolean;
  twoFactorEnabled: boolean;
  hasPasskey: boolean;
  firstName?: string;
  lastName?: string;
  name?: string;
  imageUrl?: string;
  externalUserId?: string;
};

/** Durée de validité : une requête, même lente. Au-delà, la session est relue. */
const SNAPSHOT_TTL_MS = 5 * 60 * 1000;

/** Clé propre à cet usage, dérivée du secret d'authentification (partagé par le proxy et les routes). */
function signingKey(): Buffer | null {
  const secret = process.env.BETTER_AUTH_SECRET?.trim();
  return secret ? createHmac("sha256", secret).update("scola-auth-snapshot-v1").digest() : null;
}

const sign = (payload: string, key: Buffer) => createHmac("sha256", key).update(payload).digest();

/** `null` sans secret : on ne transmet alors rien plutôt qu'une valeur falsifiable. */
export function encodeAuthSnapshot(snapshot: ScolaAuthSnapshot, now = Date.now()): string | null {
  const key = signingKey();
  if (!key) return null;
  const payload = Buffer.from(JSON.stringify({ ...snapshot, exp: now + SNAPSHOT_TTL_MS }), "utf8").toString(
    "base64url",
  );
  return `${payload}.${sign(payload, key).toString("base64url")}`;
}

function verifiedPayload(raw: string): string | null {
  const key = signingKey();
  const [payload, signature, extra] = raw.split(".");
  if (!key || !payload || !signature || extra !== undefined) return null;
  const given = Buffer.from(signature, "base64url");
  const expected = sign(payload, key);
  return given.length === expected.length && timingSafeEqual(given, expected) ? payload : null;
}

export function decodeAuthSnapshot(
  raw: string | null | undefined,
  now = Date.now(),
): ScolaAuthSnapshot | null {
  if (!raw?.trim()) return null;
  try {
    const payload = verifiedPayload(raw.trim());
    if (!payload) return null;
    const parsed = JSON.parse(Buffer.from(payload, "base64url").toString("utf8")) as unknown;
    if (!parsed || typeof parsed !== "object") return null;
    const o = parsed as Record<string, unknown>;
    if (typeof o.exp !== "number" || o.exp < now) return null;
    if (typeof o.authUserId !== "string" || !o.authUserId) return null;
    return {
      authUserId: o.authUserId,
      userId: typeof o.userId === "string" ? o.userId : o.authUserId,
      email: typeof o.email === "string" ? o.email : "",
      etablissementId: typeof o.etablissementId === "string" ? o.etablissementId : null,
      roles: Array.isArray(o.roles) ? o.roles.filter((r): r is string => typeof r === "string") : [],
      orgAdmin: Boolean(o.orgAdmin),
      platformAdmin: Boolean(o.platformAdmin),
      twoFactorEnabled: Boolean(o.twoFactorEnabled),
      hasPasskey: Boolean(o.hasPasskey),
      firstName: typeof o.firstName === "string" ? o.firstName : undefined,
      lastName: typeof o.lastName === "string" ? o.lastName : undefined,
      name: typeof o.name === "string" ? o.name : undefined,
      imageUrl: typeof o.imageUrl === "string" ? o.imageUrl : undefined,
      externalUserId: typeof o.externalUserId === "string" ? o.externalUserId : undefined,
    };
  } catch {
    return null;
  }
}
