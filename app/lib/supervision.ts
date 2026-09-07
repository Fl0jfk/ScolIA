import "server-only";

import { createHmac, timingSafeEqual } from "crypto";
import { and, eq } from "drizzle-orm";
import { cookies } from "next/headers";
import type { NextRequest } from "next/server";
import type { AppUser } from "@/app/lib/app-session";
import { listUserRolesFromDb, resolveBusinessUserId } from "@/app/lib/auth-roles-db";
import { isAnyDirectionRole } from "@/app/lib/establishment-catalog";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { user, userMembership } from "@/db/schema";

export const SUPERVISION_COOKIE_NAME = "scola_supervision";
const TTL_MS = 2 * 60 * 60 * 1000;

export type SupervisionCookiePayload = {
  actorUserId: string;
  targetUserId: string;
  etablissementId: string;
  startedAt: number;
};

export type SupervisionTargetProfile = {
  userId: string;
  businessUserId: string;
  email: string;
  firstName?: string;
  lastName?: string;
  name?: string;
  displayName: string;
  etablissementId: string;
  roles: string[];
  orgAdmin: boolean;
  platformAdmin: boolean;
  twoFactorEnabled: boolean;
  imageUrl?: string;
};

function cookieSecret(): string {
  return (
    process.env.BETTER_AUTH_SECRET?.trim() ||
    process.env.AUTH_SECRET?.trim() ||
    "dev-supervision-cookie-secret"
  );
}

/** Admin établissement, master plateforme, ou rôle direction. */
export function canStartSupervision(user: {
  roles: string[];
  orgAdmin?: boolean;
  platformAdmin?: boolean;
}): boolean {
  if (user.platformAdmin || user.orgAdmin) return true;
  if (user.roles.includes("admin")) return true;
  return isAnyDirectionRole(user.roles);
}

export function sealSupervisionCookie(payload: SupervisionCookiePayload): string {
  const body = Buffer.from(
    JSON.stringify({
      ...payload,
      exp: Date.now() + TTL_MS,
    }),
  ).toString("base64url");
  const sig = createHmac("sha256", cookieSecret()).update(body).digest("base64url");
  return `${body}.${sig}`;
}

export function openSupervisionCookie(
  sealed: string | undefined | null,
): SupervisionCookiePayload | null {
  if (!sealed?.includes(".")) return null;
  const dot = sealed.lastIndexOf(".");
  const body = sealed.slice(0, dot);
  const sig = sealed.slice(dot + 1);
  const expected = createHmac("sha256", cookieSecret()).update(body).digest("base64url");
  try {
    const a = Buffer.from(sig);
    const b = Buffer.from(expected);
    if (a.length !== b.length || !timingSafeEqual(a, b)) return null;
  } catch {
    return null;
  }
  try {
    const data = JSON.parse(Buffer.from(body, "base64url").toString("utf8")) as SupervisionCookiePayload & {
      exp?: number;
    };
    if (!data.exp || data.exp < Date.now()) return null;
    if (!data.actorUserId || !data.targetUserId || !data.etablissementId) return null;
    if (data.actorUserId === data.targetUserId) return null;
    return {
      actorUserId: data.actorUserId,
      targetUserId: data.targetUserId,
      etablissementId: data.etablissementId,
      startedAt: typeof data.startedAt === "number" ? data.startedAt : Date.now(),
    };
  } catch {
    return null;
  }
}

export function supervisionCookieSetOptions(sealed: string) {
  return {
    name: SUPERVISION_COOKIE_NAME,
    value: sealed,
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax" as const,
    path: "/",
    maxAge: Math.floor(TTL_MS / 1000),
  };
}

export function supervisionCookieClearOptions() {
  return {
    name: SUPERVISION_COOKIE_NAME,
    value: "",
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax" as const,
    path: "/",
    maxAge: 0,
  };
}

export function readSupervisionCookieFromRequest(
  request: NextRequest,
): SupervisionCookiePayload | null {
  return openSupervisionCookie(request.cookies.get(SUPERVISION_COOKIE_NAME)?.value);
}

export async function readSupervisionCookieFromJar(): Promise<SupervisionCookiePayload | null> {
  const jar = await cookies();
  return openSupervisionCookie(jar.get(SUPERVISION_COOKIE_NAME)?.value);
}

function displayNameFor(row: {
  name?: string | null;
  firstName?: string | null;
  lastName?: string | null;
  email: string;
}): string {
  const fromParts = `${row.firstName ?? ""} ${row.lastName ?? ""}`.trim();
  return row.name?.trim() || fromParts || row.email;
}

/** Charge le profil « vue » d’un utilisateur staff du tenant. */
export async function loadSupervisionTargetProfile(
  targetUserId: string,
  etablissementId: string,
): Promise<SupervisionTargetProfile | null> {
  if (!isDatabaseConfigured()) return null;
  const db = getDb();
  const [row] = await db.select().from(user).where(eq(user.id, targetUserId)).limit(1);
  if (!row) return null;

  const [membership] = await db
    .select({
      context: userMembership.context,
      active: userMembership.active,
    })
    .from(userMembership)
    .where(
      and(
        eq(userMembership.userId, targetUserId),
        eq(userMembership.etablissementId, etablissementId),
        eq(userMembership.active, true),
      ),
    )
    .limit(1);

  const homeOk = row.etablissementId === etablissementId;
  if (!membership && !homeOk) return null;
  if (membership && membership.context !== "staff") return null;
  if (!membership && homeOk) {
    // Legacy home sans ligne membership : autoriser uniquement si pas parent/élève exclusif.
  }

  const roles = await listUserRolesFromDb(row.id, etablissementId);
  const platformAdmin = Boolean(row.platformAdmin);
  const orgAdmin = Boolean(row.orgAdmin) || platformAdmin || roles.includes("admin");
  const businessUserId =
    row.externalUserId?.trim() || (await resolveBusinessUserId(row.id, etablissementId));

  return {
    userId: row.id,
    businessUserId,
    email: row.email,
    firstName: row.firstName ?? undefined,
    lastName: row.lastName ?? undefined,
    name: row.name,
    displayName: displayNameFor(row),
    etablissementId,
    roles,
    orgAdmin,
    platformAdmin,
    twoFactorEnabled: Boolean(row.twoFactorEnabled),
    imageUrl: row.image ?? undefined,
  };
}

export function supervisionTargetToAppUser(profile: SupervisionTargetProfile): AppUser {
  return {
    id: profile.userId,
    businessUserId: profile.businessUserId,
    email: profile.email,
    firstName: profile.firstName,
    lastName: profile.lastName,
    name: profile.name,
    imageUrl: profile.imageUrl,
    etablissementId: profile.etablissementId,
    roles: profile.roles,
    orgAdmin: profile.orgAdmin,
    platformAdmin: profile.platformAdmin,
    twoFactorEnabled: profile.twoFactorEnabled,
    externalUserId: profile.businessUserId !== profile.userId ? profile.businessUserId : undefined,
    authSource: "better-auth",
  };
}

/**
 * Résout la cible de supervision si le cookie est valide pour l’acteur courant.
 * Ne vérifie pas les droits « canStart » (déjà validés au start) ; vérifie acteur = cookie.actor.
 */
export async function resolveActiveSupervision(opts: {
  actorUserId: string;
  cookie?: SupervisionCookiePayload | null;
}): Promise<{
  cookie: SupervisionCookiePayload;
  target: SupervisionTargetProfile;
} | null> {
  const cookie = opts.cookie ?? (await readSupervisionCookieFromJar());
  if (!cookie) return null;
  if (cookie.actorUserId !== opts.actorUserId) return null;
  const target = await loadSupervisionTargetProfile(cookie.targetUserId, cookie.etablissementId);
  if (!target) return null;
  return { cookie, target };
}

/** Chemins supervision toujours autorisés (hors matrice modules cible). */
export function isSupervisionApiPath(pathname: string): boolean {
  return (
    pathname === "/api/supervision" ||
    pathname.startsWith("/api/supervision/")
  );
}

/** Mutations autorisées en mode supervision (sortie uniquement). */
export function isSupervisionWriteAllowedPath(pathname: string): boolean {
  return pathname === "/api/supervision/stop";
}
