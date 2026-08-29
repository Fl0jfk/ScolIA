import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAppUser } from "@/app/lib/app-session";
import { canConseilFichesDialogue } from "@/app/lib/fiches-dialogue-access";
import { submitFdConseilDecision } from "@/app/lib/fiches-dialogue-workflow";
import { requireTenantId } from "@/app/lib/tenant-scope";

type Ctx = { params: Promise<{ id: string }> };

const ConseilSchema = z.object({
  etapeId: z.string().uuid(),
  payload: z.object({
    avis: z.enum(["favorable", "reserve", "defavorable", "autre"]),
    destinationProposee: z.string().optional(),
    optionsProposees: z.array(z.string()).optional(),
    motif: z.string().optional(),
    commentaire: z.string().optional(),
  }),
  signatures: z
    .array(
      z.object({
        role: z.enum(["professeur_principal", "direction"]),
        name: z.string().min(1),
        pngBase64: z.string().optional(),
        method: z.string().optional(),
      }),
    )
    .min(1),
  auteurLabel: z.string().optional(),
});

export async function POST(req: Request, ctx: Ctx) {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  if (!canConseilFichesDialogue(appUser.user.roles, { orgAdmin: appUser.user.orgAdmin })) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const { id } = await ctx.params;
  const body = ConseilSchema.safeParse(await req.json().catch(() => null));
  if (!body.success) {
    return NextResponse.json({ error: "Payload invalide." }, { status: 400 });
  }

  const result = await submitFdConseilDecision({
    etablissementId: scope.ctx.etablissementId,
    ficheId: id,
    etapeId: body.data.etapeId,
    payload: body.data.payload,
    auteurUserId: scope.ctx.authUserId,
    auteurLabel: body.data.auteurLabel ?? appUser.user.name,
    signatures: body.data.signatures,
  });

  if (!result.ok) {
    return NextResponse.json({ error: result.error }, { status: 400 });
  }
  return NextResponse.json({ ok: true });
}
