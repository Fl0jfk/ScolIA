import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAppUser } from "@/app/lib/app-session";
import { canManageFichesDialogue } from "@/app/lib/fiches-dialogue-access";
import {
  advanceCampagneToConseil,
  generateFdFichesForCampagne,
  sendFdCampagneEtapeToFamilles,
} from "@/app/lib/fiches-dialogue-workflow";
import { requireTenantId } from "@/app/lib/tenant-scope";

type Ctx = { params: Promise<{ id: string }> };

const ActionSchema = z.object({
  action: z.enum(["generate", "send", "remind", "freeze_and_conseil"]),
  fromEtapeId: z.string().uuid().optional(),
  onlyMissing: z.boolean().optional(),
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
  const body = ActionSchema.safeParse(await req.json().catch(() => null));
  if (!body.success) {
    return NextResponse.json({ error: "Payload invalide." }, { status: 400 });
  }

  try {
    if (body.data.action === "generate") {
      const result = await generateFdFichesForCampagne(scope.ctx.etablissementId, id);
      return NextResponse.json(result);
    }
    if (body.data.action === "send" || body.data.action === "remind") {
      const result = await sendFdCampagneEtapeToFamilles({
        etablissementId: scope.ctx.etablissementId,
        campagneId: id,
        onlyMissing: body.data.onlyMissing ?? body.data.action === "remind",
        reminder: body.data.action === "remind",
      });
      return NextResponse.json(result);
    }
    if (body.data.action === "freeze_and_conseil") {
      if (!body.data.fromEtapeId) {
        return NextResponse.json({ error: "fromEtapeId requis." }, { status: 400 });
      }
      const result = await advanceCampagneToConseil({
        etablissementId: scope.ctx.etablissementId,
        campagneId: id,
        fromEtapeId: body.data.fromEtapeId,
      });
      return NextResponse.json(result);
    }
    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    const msg = e instanceof Error ? e.message : "Erreur";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
