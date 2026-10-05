import { NextResponse } from "next/server";
import { z } from "zod";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  dismissInvitationDuplicates,
  getInvitationPageById,
  linkInvitationDuplicates,
} from "@/app/lib/invitation-db";

const BodySchema = z.object({
  action: z.enum(["link", "dismiss"]),
  rsvpIds: z.array(z.string().uuid()).min(1).max(50),
});

type Ctx = { params: Promise<{ pageId: string }> };

export async function POST(req: Request, ctx: Ctx) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;
  try {
    const { pageId } = await ctx.params;
    const page = await getInvitationPageById(pageId);
    if (!page) return NextResponse.json({ error: "Page introuvable." }, { status: 404 });
    const body = BodySchema.parse(await req.json());
    if (body.action === "link") {
      const result = await linkInvitationDuplicates(pageId, body.rsvpIds);
      if (!result.ok) {
        return NextResponse.json({ error: result.error }, { status: 400 });
      }
      return NextResponse.json({ success: true, groupId: result.groupId });
    }
    const result = await dismissInvitationDuplicates(pageId, body.rsvpIds);
    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: 400 });
    }
    return NextResponse.json({ success: true });
  } catch (e) {
    if (e instanceof z.ZodError) {
      return NextResponse.json({ error: e.issues[0]?.message || "Données invalides." }, { status: 400 });
    }
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
