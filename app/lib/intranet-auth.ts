import { NextResponse } from "next/server";
import {
  AUTH_CONFIG_HINT,
  isAuthConfigError,
} from "@/app/lib/auth-request-error";
import { requireAppUser } from "@/app/lib/app-session";
import {
  isOrgAdminFromAppUser,
  isPlatformMasterFromAppUser,
} from "@/app/lib/auth-roles-db";
import {
  isOrgAdminFromPublicMetadata,
  isPlatformMasterFromPublicMetadata,
} from "@/app/lib/intranet-auth-metadata";
import { resolveSession, safeCurrentUser } from "@/app/lib/intranet-session";

export type AuthContext = {
  userId: string;
};

function authServerConfigResponse(): NextResponse {
  return NextResponse.json(
    {
      error: "Configuration auth serveur incomplète pour ce tenant.",
      code: "AUTH_SERVER_CONFIG",
      hint: AUTH_CONFIG_HINT,
    },
    { status: 503 },
  );
}

export async function requireAuth(): Promise<
  { ok: true; ctx: AuthContext } | { ok: false; response: NextResponse }
> {
  try {
    const session = await resolveSession();
    if (!session) {
      return {
        ok: false,
        response: NextResponse.json({ error: "Non autorisé.", code: "AUTH_REQUIRED" }, { status: 401 }),
      };
    }
    return { ok: true, ctx: { userId: session.userId } };
  } catch (error) {
    if (isAuthConfigError(error)) {
      return { ok: false, response: authServerConfigResponse() };
    }
    console.error("[requireAuth]", error);
    return {
      ok: false,
      response: NextResponse.json(
        { error: "Authentification temporairement indisponible.", code: "AUTH_UNAVAILABLE" },
        { status: 503 },
      ),
    };
  }
}

async function isPlatformMaster(): Promise<boolean> {
  const appUser = await requireAppUser();
  if (appUser.ok) return isPlatformMasterFromAppUser(appUser.user);
  const user = await safeCurrentUser();
  return isPlatformMasterFromPublicMetadata(user?.publicMetadata);
}

export async function isIntranetAdmin(): Promise<boolean> {
  const appUser = await requireAppUser();
  if (appUser.ok) return isOrgAdminFromAppUser(appUser.user);
  const user = await safeCurrentUser();
  return isOrgAdminFromPublicMetadata(user?.publicMetadata);
}

export async function requireAdmin(): Promise<
  { ok: true; ctx: AuthContext } | { ok: false; response: NextResponse }
> {
  const gate = await requireAuth();
  if (!gate.ok) return gate;

  if (await isIntranetAdmin()) {
    return gate;
  }

  return {
    ok: false,
    response: NextResponse.json(
      { error: "Réservé aux utilisateurs avec le rôle admin.", code: "ADMIN_REQUIRED" },
      { status: 403 },
    ),
  };
}

export async function requirePlatformMaster(): Promise<
  { ok: true; ctx: AuthContext } | { ok: false; response: NextResponse }
> {
  const gate = await requireAuth();
  if (!gate.ok) return gate;

  if (await isPlatformMaster()) {
    return gate;
  }

  return {
    ok: false,
    response: NextResponse.json(
      { error: "Réservé au profil Master plateforme.", code: "MASTER_REQUIRED" },
      { status: 403 },
    ),
  };
}
