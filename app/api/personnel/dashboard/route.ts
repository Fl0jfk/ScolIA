import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";

import { requireAuth } from "@/app/lib/intranet-auth";
import { buildPersonnelDashboard } from "@/app/lib/personnel-dashboard";
import { getAllPersonnelRecords } from "@/app/lib/personnel-storage";
import { canViewPersonnelDashboard } from "@/app/lib/personnel-types";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);

  if (!canViewPersonnelDashboard(roles)) {
    return NextResponse.json({ error: "Accès réservé à la RH / comptabilité / direction." }, { status: 403 });
  }

  try {
    const records = await getAllPersonnelRecords();
    const dashboard = await buildPersonnelDashboard(records);
    return NextResponse.json(dashboard);
  } catch (e) {
    console.error("[personnel/dashboard]", e);
    return NextResponse.json({ error: "Impossible de charger le tableau de bord." }, { status: 500 });
  }
}
