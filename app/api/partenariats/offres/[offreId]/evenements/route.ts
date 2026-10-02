import { NextResponse } from "next/server";
import { z } from "zod";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  createPartenariatEvenement,
  listPartenariatEvenements,
} from "@/app/lib/partenariats-db";

const CreateSchema = z.object({
  title: z.string().min(1).max(200),
  startsAt: z.string().min(1),
  endsAt: z.string().nullable().optional(),
  location: z.string().max(300).optional(),
  notes: z.string().max(2000).optional(),
});

type Ctx = { params: Promise<{ offreId: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireModule("partenariats");
  if (!gate.ok) return gate.response;
  try {
    const { offreId } = await ctx.params;
    const evenements = await listPartenariatEvenements(offreId);
    return NextResponse.json({ evenements });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}

export async function POST(req: Request, ctx: Ctx) {
  const gate = await requireModule("partenariats");
  if (!gate.ok) return gate.response;
  try {
    const { offreId } = await ctx.params;
    const body = CreateSchema.parse(await req.json());
    const evenement = await createPartenariatEvenement({ offreId, ...body });
    return NextResponse.json({ evenement }, { status: 201 });
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
