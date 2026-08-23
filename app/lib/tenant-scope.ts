import "server-only";

import { NextResponse } from "next/server";
import { requireAppUser } from "@/app/lib/app-session";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { assertUserBelongsToTenant } from "@/app/lib/etablissement-db";
import { getTenant } from "@/app/lib/tenant-context";

export type TenantScopeContext = {
  etablissementId: string;
  userId: string;
  authUserId: string;
};

function tenantForbiddenResponse(message: string): NextResponse {
  return NextResponse.json(
    { error: message, code: "TENANT_FORBIDDEN" },
    { status: 403 },
  );
}

function tenantUnavailableResponse(): NextResponse {
  return NextResponse.json(
    { error: "Établissement introuvable pour cette session.", code: "TENANT_UNAVAILABLE" },
    { status: 503 },
  );
}

/**
 * Résout l'UUID établissement depuis la session + hostname tenant.
 * Fail-closed si l'utilisateur n'appartient pas au tenant courant.
 */
export async function requireTenantId(): Promise<
  { ok: true; ctx: TenantScopeContext } | { ok: false; response: NextResponse }
> {
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return {
      ok: false,
      response: NextResponse.json({ error: "Non autorisé.", code: "AUTH_REQUIRED" }, { status: 401 }),
    };
  }

  const tenant = await getTenant();
  const tenantGate = await assertUserBelongsToTenant({
    userEtablissementId: appUser.user.etablissementId,
    platformAdmin: appUser.user.platformAdmin,
    tenant,
  });
  if (!tenantGate.ok) {
    return { ok: false, response: tenantForbiddenResponse(tenantGate.message) };
  }

  const fromSession = appUser.user.etablissementId?.trim();
  const fromTenant = await resolveCurrentEtablissementId();
  const etablissementId = fromSession || fromTenant;
  if (!etablissementId) {
    return { ok: false, response: tenantUnavailableResponse() };
  }

  if (fromSession && fromTenant && fromSession !== fromTenant) {
    return {
      ok: false,
      response: tenantForbiddenResponse(
        "Incohérence tenant session — accès refusé.",
      ),
    };
  }

  return {
    ok: true,
    ctx: {
      etablissementId,
      userId: appUser.user.businessUserId,
      authUserId: appUser.user.id,
    },
  };
}

/** Vérifie qu'une ligne métier appartient au tenant courant (PATCH/DELETE). */
export function assertRowBelongsToTenant(
  row: { etablissementId: string },
  etablissementId: string,
): void {
  if (row.etablissementId !== etablissementId) {
    throw new Error("TENANT_ROW_MISMATCH");
  }
}
