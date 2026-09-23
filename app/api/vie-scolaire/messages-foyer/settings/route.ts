import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  getFamilleMessagingSettings,
  upsertFamilleMessagingSettings,
} from "@/app/lib/famille-messaging-db";
import {
  canInitiateFromMatrix,
  FAMILLE_MESSAGING_ROLE_OPTIONS,
} from "@/app/lib/famille-messaging-matrix";
import { hasGlobalAdminRole } from "@/app/lib/intranet-role-utils";

function canEditMatrix(roles: string[], orgAdmin?: boolean): boolean {
  if (orgAdmin || hasGlobalAdminRole(roles)) return true;
  return roles.some((r) => {
    const n = r.toLowerCase();
    return n.includes("direction") || n.includes("directeur") || n === "admin";
  });
}

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }
  const settings = await getFamilleMessagingSettings(etabId);
  return NextResponse.json({
    settings,
    roleOptions: FAMILLE_MESSAGING_ROLE_OPTIONS,
    canEdit: canEditMatrix(roles, user?.orgAdmin),
    canInitiate: canInitiateFromMatrix(roles, settings, { orgAdmin: user?.orgAdmin }),
  });
}

export async function PATCH(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canEditMatrix(roles, user?.orgAdmin)) {
    return NextResponse.json({ error: "Matrice réservée à la direction." }, { status: 403 });
  }
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }
  const body = (await req.json().catch(() => ({}))) as {
    rolesCanInitiate?: string[];
    profOwnClassesOnly?: boolean;
    allowBroadcast?: boolean;
    allowParentAttachments?: boolean;
  };
  try {
    const settings = await upsertFamilleMessagingSettings(
      etabId,
      {
        rolesCanInitiate: Array.isArray(body.rolesCanInitiate)
          ? body.rolesCanInitiate.map(String)
          : [],
        profOwnClassesOnly: body.profOwnClassesOnly !== false,
        allowBroadcast: body.allowBroadcast !== false,
        allowParentAttachments: body.allowParentAttachments !== false,
      },
      userId,
    );
    return NextResponse.json({ success: true, settings });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Enregistrement impossible." },
      { status: 400 },
    );
  }
}
