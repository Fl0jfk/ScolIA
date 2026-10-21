import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAppUser } from "@/app/lib/app-session";
import { canManageFichesDialogue } from "@/app/lib/fiches-dialogue-access";
import { resolveFdConflit } from "@/app/lib/fiches-dialogue-workflow";
import { requireTenantId } from "@/app/lib/tenant-scope";

type Ctx = { params: Promise<{ id: string }> };

const ConflitSchema = z.object({
  resolution: z.enum(["maintien_famille", "avis_conseil"]),
  note: z.string().max(2000).optional(),
});

export async function POST(req: Request, ctx: Ctx) {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  if (!canManageFichesDialogue(appUser.user.roles, { orgAdmin: appUser.user.orgAdmin })) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const { id } = await ctx.params;
  const body = ConflitSchema.safeParse(await req.json().catch(() => null));
  if (!body.success) {
    return NextResponse.json({ error: "Payload invalide." }, { status: 400 });
  }

  const result = await resolveFdConflit({
    etablissementId: scope.ctx.etablissementId,
    ficheId: id,
    resolution: body.data.resolution,
    note: body.data.note,
    userId: scope.ctx.authUserId,
    auteurLabel: appUser.user.name,
  });
  if (!result.ok) {
    return NextResponse.json({ error: result.error }, { status: 400 });
  }
  return NextResponse.json({ ok: true });
}
