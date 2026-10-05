import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  getInvitationPageById,
  listInvitationRsvps,
} from "@/app/lib/invitation-db";
import {
  buildInvitationRsvpExportCsv,
  invitationExportFilename,
} from "@/app/lib/invitation-export";

type Ctx = { params: Promise<{ pageId: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;
  try {
    const { pageId } = await ctx.params;
    const page = await getInvitationPageById(pageId);
    if (!page) return NextResponse.json({ error: "Page introuvable." }, { status: 404 });
    const rsvps = await listInvitationRsvps(pageId);
    const csv = buildInvitationRsvpExportCsv(rsvps);
    const filename = invitationExportFilename(page.slug);
    return new NextResponse(csv, {
      headers: {
        "Content-Type": "text/csv; charset=utf-8",
        "Content-Disposition": `attachment; filename="${filename}"`,
        "Cache-Control": "no-store",
      },
    });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
