import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canManagePersonnel } from "@/app/lib/personnel-types";
import { probeRhDriveHealth } from "@/app/lib/rh/graph-rh-drive";


/** État du lien OneDrive RH (attachée de gestion). */
export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!user || !canManagePersonnel(rolesFromUserLike(user))) {
    return NextResponse.json({ error: "Réservé à la RH / comptabilité." }, { status: 403 });
  }

  const status = await probeRhDriveHealth();
  return NextResponse.json(status);
}
