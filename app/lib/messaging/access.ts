import "server-only";

import { NextResponse } from "next/server";
import type { AppUser } from "@/app/lib/app-session";
import { requireAppUser } from "@/app/lib/app-session";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { isEleveOnlyRoleSet } from "@/app/lib/intranet-role-utils";

export type MessagingAuthContext = {
  user: AppUser;
  /** Identifiant Better-Auth (`user.id`). */
  userId: string;
  etablissementId: string;
};

/** Personnel uniquement — exclut parent-only / élève-only. */
export function canUseMessaging(roles: string[]): boolean {
  if (isEleveOnlyRoleSet(roles)) return false;
  const visible = roles.filter((r) => r !== "master");
  if (visible.length === 0) return roles.includes("master");
  return visible.some((r) => r !== "parent" && r !== "eleve");
}

/**
 * Auth messagerie : session + personnel (pas de gate module strict),
 * pour que l’overlay flottant fonctionne même sans tuile activée.
 */
export async function requireMessagingContext(): Promise<
  { ok: true; ctx: MessagingAuthContext } | { ok: false; response: NextResponse }
> {
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return {
      ok: false,
      response: NextResponse.json(
        { error: "Non autorisé.", code: "AUTH_REQUIRED" },
        { status: 401 },
      ),
    };
  }

  const user = appUser.user;

  if (!canUseMessaging(user.roles) && !user.orgAdmin && !user.platformAdmin) {
    return {
      ok: false,
      response: NextResponse.json(
        {
          error: "Messagerie réservée au personnel de l’établissement.",
          code: "MESSAGING_FORBIDDEN",
        },
        { status: 403 },
      ),
    };
  }

  const etablissementId =
    user.etablissementId?.trim() || (await resolveCurrentEtablissementId()) || "";
  if (!etablissementId) {
    return {
      ok: false,
      response: NextResponse.json(
        {
          error: "Établissement introuvable pour cette session.",
          code: "ETABLISSEMENT_REQUIRED",
        },
        { status: 503 },
      ),
    };
  }

  return {
    ok: true,
    ctx: {
      user,
      userId: user.id,
      etablissementId,
    },
  };
}
