import { NextResponse } from "next/server";
import { z } from "zod";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  deletePartenariatEvenement,
  updatePartenariatEvenement,
} from "@/app/lib/partenariats-db";

const PatchSchema = z.object({
  title: z.string().min(1).max(200).optional(),
  startsAt: z.string().optional(),
  endsAt: z.string().nullable().optional(),
  location: z.string().max(300).optional(),
  notes: z.string().max(2000).optional(),
});

type Ctx = { params: Promise<{ offreId: string; eventId: string }> };

export async function PATCH(req: Request, ctx: Ctx) {
  const gate = await requireModule("partenariats");
  if (!gate.ok) return gate.response;
  try {
    const { eventId } = await ctx.params;
    const body = PatchSchema.parse(await req.json());
    const evenement = await updatePartenariatEvenement(eventId, body);
    return NextResponse.json({ evenement });
  } catch (e) {
    if (e instanceof z.ZodError) {
      return NextResponse.json(
        { error: e.issues[0]?.message || "Données invalides." },
        { status: 400 },
      );
    }
    return NextResponse.json({ error: String(e) }, { status: 400 });
  }
}

export async function DELETE(_req: Request, ctx: Ctx) {
  const gate = await requireModule("partenariats");
  if (!gate.ok) return gate.response;
  try {
    const { eventId } = await ctx.params;
    await deletePartenariatEvenement(eventId);
    return NextResponse.json({ success: true });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
