import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canAccessRgpdModule } from "@/app/lib/rgpd-access";
import { buildRgpdIncidentPdf } from "@/app/lib/rgpd-incident-pdf";
import { loadRgpdIncident } from "@/app/lib/rgpd-storage";

export async function GET(
  _req: Request,
  ctx: { params: Promise<{ id: string }> },
) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canAccessRgpdModule(roles)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const { id } = await ctx.params;
  const incident = await loadRgpdIncident(id);
  if (!incident) {
    return NextResponse.json({ error: "Incident introuvable." }, { status: 404 });
  }

  const pdf = await buildRgpdIncidentPdf(incident);
  return new NextResponse(Buffer.from(pdf), {
    headers: {
      "Content-Type": "application/pdf",
      "Content-Disposition": `attachment; filename="incident-${id}.pdf"`,
    },
  });
}
