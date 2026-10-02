import { NextResponse } from "next/server";
import { z } from "zod";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  deleteInvitationPage,
  getInvitationPageById,
  updateInvitationPage,
} from "@/app/lib/invitation-db";
import {
  INVITATION_ASK_SITUATION,
  INVITATION_DIPLOMA_MODES,
  INVITATION_THEMES,
} from "@/app/lib/invitation-types";

const PatchSchema = z.object({
  title: z.string().min(1).max(200).optional(),
  slug: z.string().max(80).optional(),
  intro: z.string().max(4000).optional(),
  theme: z.enum(INVITATION_THEMES).optional(),
  enabled: z.boolean().optional(),
  startsAt: z.string().nullable().optional(),
  endsAt: z.string().nullable().optional(),
  location: z.string().max(500).optional(),
  diplomaMode: z.enum(INVITATION_DIPLOMA_MODES).optional(),
  maxTotalPersons: z.number().int().min(1).max(50000).optional(),
  maxPersonsPerEleve: z.number().int().min(1).max(50).optional(),
  notifyEmail: z.string().max(200).nullable().optional(),
  requireEligible: z.boolean().optional(),
  askSituation: z.enum(INVITATION_ASK_SITUATION).optional(),
  rsvpClosesAt: z.string().nullable().optional(),
});

type Ctx = { params: Promise<{ pageId: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;
  try {
    const { pageId } = await ctx.params;
    const page = await getInvitationPageById(pageId);
    if (!page) return NextResponse.json({ error: "Page introuvable." }, { status: 404 });
    return NextResponse.json({ page });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}

export async function PATCH(req: Request, ctx: Ctx) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;
  try {
    const { pageId } = await ctx.params;
    const body = PatchSchema.parse(await req.json());
    const page = await updateInvitationPage(pageId, body);
    if (!page) return NextResponse.json({ error: "Page introuvable." }, { status: 404 });
    return NextResponse.json({ page });
  } catch (e) {
    if (e instanceof z.ZodError) {
      return NextResponse.json({ error: e.issues[0]?.message || "Données invalides." }, { status: 400 });
    }
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}

export async function DELETE(_req: Request, ctx: Ctx) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;
  try {
    const { pageId } = await ctx.params;
    const ok = await deleteInvitationPage(pageId);
    if (!ok) return NextResponse.json({ error: "Page introuvable." }, { status: 404 });
    return NextResponse.json({ success: true });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
