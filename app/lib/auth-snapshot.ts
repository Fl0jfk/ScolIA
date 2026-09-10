import "server-only";

/** Header interne proxy → RSC / API (jamais exposé au client). */
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

export function encodeAuthSnapshot(snapshot: ScolaAuthSnapshot): string {
  return Buffer.from(JSON.stringify(snapshot), "utf8").toString("base64url");
}

export function decodeAuthSnapshot(raw: string | null | undefined): ScolaAuthSnapshot | null {
  if (!raw?.trim()) return null;
  try {
    const parsed = JSON.parse(Buffer.from(raw, "base64url").toString("utf8")) as unknown;
    if (!parsed || typeof parsed !== "object") return null;
    const o = parsed as Record<string, unknown>;
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
