import { NextResponse } from "next/server";
import {
  AUTH_CONFIG_HINT,
  isAuthConfigError,
} from "@/app/lib/auth-request-error";
import { requireAppUser, type AppUser } from "@/app/lib/app-session";
import {
  isOrgAdminFromAppUser,
  isPlatformMasterFromAppUser,
} from "@/app/lib/auth-roles-db";
import {
  isOrgAdminFromPublicMetadata,
  isPlatformMasterFromPublicMetadata,
} from "@/app/lib/intranet-auth-metadata";
import {
  getIntranetModuleById,
  rolesAllowModule,
} from "@/app/lib/intranet-modules";
import { resolveSession, safeCurrentUser } from "@/app/lib/intranet-session";

export type AuthContext = {
  userId: string;
};

export type ModuleAuthContext = AuthContext & {
  user: AppUser;
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

function moduleForbiddenResponse(moduleId: string): NextResponse {
  return NextResponse.json(
    { error: `Accès refusé au module « ${moduleId} ».`, code: "MODULE_FORBIDDEN" },
    { status: 403 },
  );
}

async function resolveModuleUser():
  Promise<
    | { ok: true; user: AppUser; userId: string }
    | { ok: false; response: NextResponse }
  > {
  const gate = await requireAuth();
  if (!gate.ok) return gate;
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return {
      ok: false,
      response: NextResponse.json({ error: "Non autorisé.", code: "AUTH_REQUIRED" }, { status: 401 }),
    };
  }
  return { ok: true, user: appUser.user, userId: gate.ctx.userId };
}

/** Garde RBAC explicite sur un module intranet (fail-closed). */
export async function requireModule(
  moduleId: string,
): Promise<
  { ok: true; ctx: ModuleAuthContext } | { ok: false; response: NextResponse }
> {
  const resolved = await resolveModuleUser();
  if (!resolved.ok) return resolved;

  const module = getIntranetModuleById(moduleId);
  if (!module) {
    return { ok: false, response: moduleForbiddenResponse(moduleId) };
  }

  const isOrgAdmin = isOrgAdminFromAppUser(resolved.user);
  const { loadModuleAccess } = await import("@/app/lib/module-access-store");
  const access = await loadModuleAccess();
  if (
    !rolesAllowModule(resolved.user.roles, module, isOrgAdmin, access, {
      userId: resolved.user.id,
      businessUserId: resolved.user.businessUserId,
    })
  ) {
    return { ok: false, response: moduleForbiddenResponse(moduleId) };
  }

  return {
    ok: true,
    ctx: { userId: resolved.userId, user: resolved.user },
  };
}

/** Accès si au moins un des modules listés est autorisé. */
export async function requireAnyModule(
  moduleIds: string[],
): Promise<
  { ok: true; ctx: ModuleAuthContext; moduleId: string } | { ok: false; response: NextResponse }
> {
  const resolved = await resolveModuleUser();
  if (!resolved.ok) return resolved;

  const isOrgAdmin = isOrgAdminFromAppUser(resolved.user);
  const { loadModuleAccess } = await import("@/app/lib/module-access-store");
  const access = await loadModuleAccess();
  for (const moduleId of moduleIds) {
    const module = getIntranetModuleById(moduleId);
    if (
      module &&
      rolesAllowModule(resolved.user.roles, module, isOrgAdmin, access, {
        userId: resolved.user.id,
        businessUserId: resolved.user.businessUserId,
      })
    ) {
      return {
        ok: true,
        moduleId,
        ctx: { userId: resolved.userId, user: resolved.user },
      };
    }
  }

  return {
    ok: false,
    response: NextResponse.json(
      { error: "Accès refusé à cette ressource.", code: "MODULE_FORBIDDEN" },
      { status: 403 },
    ),
  };
}
