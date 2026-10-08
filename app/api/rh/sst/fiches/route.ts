import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAppUser } from "@/app/lib/app-session";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { canAccessSstRegistre } from "@/app/lib/sst-registre/access";
import { createSstFiche, listSstFiches } from "@/app/lib/sst-registre/db";

export async function GET() {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }
  if (!canAccessSstRegistre(appUser.user.roles)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  try {
    const fiches = await listSstFiches({ etablissementId: scope.ctx.etablissementId });
    return NextResponse.json({ fiches });
  } catch (e) {
    console.error("[rh/sst/fiches GET]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur fiches SST." },
      { status: 500 },
    );
  }
}

const CreateSchema = z.object({
  observedDate: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  observedTime: z.string().max(8).optional(),
  lieu: z.string().max(200).optional(),
  observations: z.string().min(5).max(8000),
  suggestions: z.string().max(4000).optional(),
  signatureDataUrl: z.string().max(1_200_000).optional(),
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
  const parsed = CreateSchema.safeParse(json);
  if (!parsed.success) {
    return NextResponse.json({ error: "Données invalides." }, { status: 400 });
  }

  const result = await createSstFiche({
    etablissementId: scope.ctx.etablissementId,
    userId: appUser.user.id,
    firstName: appUser.user.firstName ?? "",
    lastName: appUser.user.lastName ?? "",
    roles: appUser.user.roles,
    observedDate: parsed.data.observedDate,
    observedTime: parsed.data.observedTime,
    lieu: parsed.data.lieu,
    observations: parsed.data.observations,
    suggestions: parsed.data.suggestions,
    signatureDataUrl: parsed.data.signatureDataUrl,
  });

  if (!result.ok) {
    return NextResponse.json({ error: result.error }, { status: 400 });
  }
  return NextResponse.json({ ok: true, fiche: result.fiche }, { status: 201 });
}
