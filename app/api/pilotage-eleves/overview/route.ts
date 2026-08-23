import { NextResponse } from "next/server";
import { getDirectoryUserRoles } from "@/app/lib/directory-members";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  canAccessPilotageModule,
  canIndexPilotage,
  canWritePilotageNotes,
  resolvePilotageSecteursForRoles,
} from "@/app/lib/pilotage-eleves-access";
import { buildPilotageOverview } from "@/app/lib/pilotage-eleves";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const roles = await getDirectoryUserRoles(gate.ctx.userId);
  if (!canAccessPilotageModule(roles)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const secteurs = await resolvePilotageSecteursForRoles(roles, gate.ctx.userId);
  const overview = await buildPilotageOverview(secteurs, {
    canWriteNotes: canWritePilotageNotes(roles),
    canIndex: canIndexPilotage(roles),
  });
  return NextResponse.json(overview);
}
