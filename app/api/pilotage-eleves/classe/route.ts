import { NextRequest, NextResponse } from "next/server";
import { getDirectoryUserRoles } from "@/app/lib/directory-members";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  canAccessPilotageModule,
  resolvePilotageSecteursForRoles,
} from "@/app/lib/pilotage-eleves-access";
import { listClassRoster } from "@/app/lib/pilotage-eleves";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";

export async function GET(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const roles = await getDirectoryUserRoles(gate.ctx.userId);
  if (!canAccessPilotageModule(roles)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const secteur = String(req.nextUrl.searchParams.get("secteur") ?? "").trim() as Secteur;
  const classe = String(req.nextUrl.searchParams.get("classe") ?? "").trim();
  if (!secteur || !classe) {
    return NextResponse.json({ error: "secteur et classe requis." }, { status: 400 });
  }

  const allowed = await resolvePilotageSecteursForRoles(roles, gate.ctx.userId);
  const eleves = await listClassRoster(secteur, classe, allowed);
  return NextResponse.json({ secteur, classe, eleves });
}
