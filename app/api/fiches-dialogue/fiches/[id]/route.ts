import { NextResponse } from "next/server";
import { requireAppUser } from "@/app/lib/app-session";
import { canConseilFichesDialogue } from "@/app/lib/fiches-dialogue-access";
import { getFdFicheDossier } from "@/app/lib/fiches-dialogue-workflow";
import { requireTenantId } from "@/app/lib/tenant-scope";

type Ctx = { params: Promise<{ id: string }> };

/**
 * Dossier famille complet pour le staff qui prépare l’avis du conseil.
 * Accessible au PP/CPE et à la direction, jamais aux parents : les jetons et
 * contacts ne doivent pas sortir de l’intranet.
 */
export async function GET(_req: Request, ctx: Ctx) {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  if (!canConseilFichesDialogue(appUser.user.roles, { orgAdmin: appUser.user.orgAdmin })) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const { id } = await ctx.params;
  const dossier = await getFdFicheDossier(scope.ctx.etablissementId, id);
  if (!dossier) return NextResponse.json({ error: "Introuvable." }, { status: 404 });
  return NextResponse.json(dossier);
}
