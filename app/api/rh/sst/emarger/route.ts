import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAppUser } from "@/app/lib/app-session";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { canAccessSstRegistre } from "@/app/lib/sst-registre/access";
import { signSstEmargement } from "@/app/lib/sst-registre/db";

const BodySchema = z.object({
  signatureDataUrl: z.string().min(40).max(1_200_000),
  remarques: z.string().max(500).optional(),
});

export async function POST(req: Request) {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }
  if (!canAccessSstRegistre(appUser.user.roles)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  let json: unknown;
  try {
    json = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }
  const parsed = BodySchema.safeParse(json);
  if (!parsed.success) {
    return NextResponse.json({ error: "Données invalides." }, { status: 400 });
  }

  const result = await signSstEmargement({
    etablissementId: scope.ctx.etablissementId,
    userId: appUser.user.id,
    firstName: appUser.user.firstName ?? "",
    lastName: appUser.user.lastName ?? "",
    roles: appUser.user.roles,
    signatureDataUrl: parsed.data.signatureDataUrl,
    remarques: parsed.data.remarques,
  });

  if (!result.ok) {
    const status = result.error.includes("déjà signé") ? 409 : 400;
    return NextResponse.json({ error: result.error }, { status });
  }

  return NextResponse.json({ ok: true, emargement: result.emargement });
}
