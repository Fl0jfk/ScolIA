import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  computeDashboardStats,
  findDuplicateSuspects,
  getInvitationPageById,
  listInvitationRsvps,
} from "@/app/lib/invitation-db";

type Ctx = { params: Promise<{ pageId: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;
  try {
    const { pageId } = await ctx.params;
    const page = await getInvitationPageById(pageId);
    if (!page) return NextResponse.json({ error: "Page introuvable." }, { status: 404 });
    const rsvps = await listInvitationRsvps(pageId);
    const stats = computeDashboardStats(page, rsvps);
    const duplicates = findDuplicateSuspects(rsvps);
    return NextResponse.json({ page, rsvps, stats, duplicates });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
