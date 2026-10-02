import { NextResponse } from "next/server";
import { z } from "zod";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  listPartenariatInscriptions,
  updatePartenariatInscriptionStatus,
} from "@/app/lib/partenariats-db";
import { PARTENARIAT_INSCRIPTION_STATUSES } from "@/app/lib/partenariats-types";

type Ctx = { params: Promise<{ offreId: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireModule("partenariats");
  if (!gate.ok) return gate.response;
  try {
    const { offreId } = await ctx.params;
    const inscriptions = await listPartenariatInscriptions(offreId);
    return NextResponse.json({ inscriptions });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}

const PatchSchema = z.object({
  inscriptionId: z.string().uuid(),
  status: z.enum(PARTENARIAT_INSCRIPTION_STATUSES),
  adminNote: z.string().max(2000).optional(),
});

export async function PATCH(req: Request, ctx: Ctx) {
  const gate = await requireModule("partenariats");
  if (!gate.ok) return gate.response;
  try {
    void (await ctx.params);
    const body = PatchSchema.parse(await req.json());
    const inscription = await updatePartenariatInscriptionStatus(
      body.inscriptionId,
      body.status,
      body.adminNote,
    );
    return NextResponse.json({ inscription });
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
