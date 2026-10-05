import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { deleteInvitationRsvp } from "@/app/lib/invitation-db";

type Ctx = { params: Promise<{ pageId: string; rsvpId: string }> };

export async function DELETE(_req: Request, ctx: Ctx) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;
  try {
    const { pageId, rsvpId } = await ctx.params;
    const ok = await deleteInvitationRsvp(pageId, rsvpId);
    if (!ok) return NextResponse.json({ error: "Réponse introuvable." }, { status: 404 });
    return NextResponse.json({ success: true });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
