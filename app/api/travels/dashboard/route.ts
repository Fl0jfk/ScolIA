import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";

import { requireAuth } from "@/app/lib/intranet-auth";
import { loadAppConfig } from "@/app/lib/app-config";
import {
  buildTravelsDirectionDashboard,
  resolveDirectionEtab,
} from "@/app/lib/travels-direction-dashboard";
import { listTravelsIndex } from "@/app/lib/travels-storage";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const bundle = await loadAppConfig();
  const etab = resolveDirectionEtab(roles, bundle.establishments);

  if (!etab) {
    return NextResponse.json({ isDirection: false });
  }

  try {
    const trips = await listTravelsIndex();
    const dashboard = buildTravelsDirectionDashboard(trips, etab);
    return NextResponse.json({ isDirection: true, dashboard });
  } catch (e) {
    console.error("[travels/dashboard]", e);
    return NextResponse.json({ error: "Impossible de charger le tableau de bord." }, { status: 500 });
  }
}
