import "server-only";

import { NextResponse } from "next/server";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import { requireAdmin, requireAuth, type AuthContext } from "@/app/lib/intranet-auth";
import { hasEleveRole, hasGlobalAdminRole, hasRole } from "@/app/lib/intranet-role-utils";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";

const REFERENT_ROLES = [...INTRANET_DIRECTION_SLUGS, "administratif", "surveillant", "cpe"];

export async function requireEleveAuth(): Promise<
  { ok: true; ctx: AuthContext } | { ok: false; response: NextResponse }
> {
  const gate = await requireAuth();
  if (!gate.ok) return gate;

  const user = await safeCurrentUser();
  const roles = intranetRolesFromMetadata(user?.publicMetadata);
  if (!hasEleveRole(roles)) {
    return {
      ok: false,
      response: NextResponse.json(
        { error: "Accès réservé aux élèves.", code: "ELEVE_ONLY" },
        { status: 403 },
      ),
    };
  }
  return gate;
}

export async function requireBienEtreReferentAuth(): Promise<
  { ok: true; ctx: AuthContext } | { ok: false; response: NextResponse }
> {
  const adminGate = await requireAdmin();
  if (adminGate.ok) return adminGate;

  const gate = await requireAuth();
  if (!gate.ok) return gate;

  const user = await safeCurrentUser();
  const roles = intranetRolesFromMetadata(user?.publicMetadata);
  if (hasGlobalAdminRole(roles)) return gate;
  if (REFERENT_ROLES.some((r) => hasRole(roles, r))) return gate;

  return {
    ok: false,
    response: NextResponse.json({ error: "Accès refusé.", code: "MODULE_FORBIDDEN" }, { status: 403 }),
  };
}
