import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canViewAllConventions } from "@/app/lib/stage-access";
import { buildFreshConventionPdfDownload } from "@/app/lib/stage-pdf-store";
import { getStageConvention } from "@/app/lib/stage-storage";

export async function GET(_req: Request, ctx: { params: Promise<{ id: string }> }) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canViewAllConventions(roles) && !roles.includes("parent")) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }

    const { id } = await ctx.params;
    const convention = await getStageConvention(id);
    if (!convention) return NextResponse.json({ error: "Convention introuvable." }, { status: 404 });

    const { bytes, fileName } = await buildFreshConventionPdfDownload(convention);

    return new NextResponse(Buffer.from(bytes), {
      status: 200,
      headers: {
        "Content-Type": "application/pdf",
        "Content-Disposition": `attachment; filename="${fileName}"`,
        "Cache-Control": "no-store",
      },
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
