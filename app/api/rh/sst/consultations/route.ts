import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAppUser } from "@/app/lib/app-session";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { canManageSstRegistre } from "@/app/lib/sst-registre/access";
import { addSstConsultation, listSstConsultations } from "@/app/lib/sst-registre/db";

export async function GET() {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }
  if (!canManageSstRegistre(appUser.user.roles)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  try {
    const consultations = await listSstConsultations(scope.ctx.etablissementId);
    return NextResponse.json({ consultations });
  } catch (e) {
    console.error("[rh/sst/consultations GET]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur consultations SST." },
      { status: 500 },
    );
  }
}

const BodySchema = z.object({
  comments: z.string().max(2000).optional(),
  signatureDataUrl: z.string().max(1_200_000).optional(),
});

export async function POST(req: Request) {
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
  const parsed = BodySchema.safeParse(json);
  if (!parsed.success) {
    return NextResponse.json({ error: "Données invalides." }, { status: 400 });
  }

  const result = await addSstConsultation({
    etablissementId: scope.ctx.etablissementId,
    userId: appUser.user.id,
    firstName: appUser.user.firstName ?? "",
    lastName: appUser.user.lastName ?? "",
    roles: appUser.user.roles,
    comments: parsed.data.comments,
    signatureDataUrl: parsed.data.signatureDataUrl,
  });

  if (!result.ok) {
    return NextResponse.json({ error: result.error }, { status: 400 });
  }
  return NextResponse.json({ ok: true, consultation: result.consultation }, { status: 201 });
}
