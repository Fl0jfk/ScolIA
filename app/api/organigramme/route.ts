import { NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import {
  canEditOrganigramme,
  loadOrganigramConfig,
} from "@/app/lib/organigramme-config";
import { buildOrganigramView } from "@/app/lib/organigramme-resolve";
import { getPersonnelIndex } from "@/app/lib/personnel-storage";
import { getRequestsRoutingConfig } from "@/app/lib/requests-routing-config";

function rolesFromUser(user: Awaited<ReturnType<typeof safeCurrentUser>>) {
  const rolesRaw = user?.publicMetadata?.role;
  return Array.isArray(rolesRaw) ? rolesRaw.map(String) : rolesRaw ? [String(rolesRaw)] : [];
}

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const user = await safeCurrentUser();
    const roles = rolesFromUser(user);
    const [config, personnelIndex, app, routing] = await Promise.all([
      loadOrganigramConfig(),
      getPersonnelIndex().catch(() => []),
      loadAppConfig().catch(() => null),
      getRequestsRoutingConfig().catch(() => null),
    ]);

    const view = buildOrganigramView(config, {
      personnelIndex,
      establishments: app?.establishments || [],
      routing,
    });

    return NextResponse.json({
      view,
      canEdit: canEditOrganigramme(roles),
    });
  } catch (e) {
    console.error("[organigramme] GET", e);
    return NextResponse.json({ error: "Impossible de charger l'organigramme." }, { status: 500 });
  }
}
