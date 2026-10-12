import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAppUser } from "@/app/lib/app-session";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { canAccessSstRegistre, canManageSstRegistre } from "@/app/lib/sst-registre/access";
import { advanceSstFiche, getSstFiche } from "@/app/lib/sst-registre/db";

type Ctx = { params: Promise<{ id: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }
  if (!canAccessSstRegistre(appUser.user.roles)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const { id } = await ctx.params;
  const fiche = await getSstFiche({
    etablissementId: scope.ctx.etablissementId,
    ficheId: id,
    roles: appUser.user.roles,
  });
  if (!fiche) {
    return NextResponse.json({ error: "Fiche introuvable." }, { status: 404 });
  }
  return NextResponse.json({ fiche });
}

const PatchSchema = z.object({
  action: z.literal("advance"),
  notes: z.string().max(4000).optional(),
  signatureDataUrl: z.string().max(1_200_000).optional(),
});

export async function PATCH(req: Request, ctx: Ctx) {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }
  if (!canManageSstRegistre(appUser.user.roles)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  let json: unknown;
  try {
    json = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }
  const parsed = PatchSchema.safeParse(json);
  if (!parsed.success) {
    return NextResponse.json({ error: "Données invalides." }, { status: 400 });
  }

  const { id } = await ctx.params;
  const result = await advanceSstFiche({
    etablissementId: scope.ctx.etablissementId,
    ficheId: id,
    userId: appUser.user.id,
    firstName: appUser.user.firstName ?? "",
    lastName: appUser.user.lastName ?? "",
    roles: appUser.user.roles,
    notes: parsed.data.notes ?? "",
    signatureDataUrl: parsed.data.signatureDataUrl,
  });

  if (!result.ok) {
    return NextResponse.json({ error: result.error }, { status: 400 });
  }
  return NextResponse.json({ ok: true, fiche: result.fiche });
}
