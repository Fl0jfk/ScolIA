import { NextResponse } from "next/server";
import { requireAppUser } from "@/app/lib/app-session";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { canAccessSstRegistre } from "@/app/lib/sst-registre/access";
import { signSstEmargement } from "@/app/lib/sst-registre/db";
import { parseEmargerBody } from "@/app/lib/sst-registre/emarger-body";

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
  const parsed = parseEmargerBody(json);
  if (!parsed.ok) {
    return NextResponse.json({ error: parsed.error }, { status: 400 });
  }

  const result = await signSstEmargement({
    etablissementId: scope.ctx.etablissementId,
    userId: appUser.user.id,
    firstName: appUser.user.firstName ?? "",
    lastName: appUser.user.lastName ?? "",
    roles: appUser.user.roles,
    signatureDataUrl: parsed.data.signatureDataUrl,
    attestationOnly: parsed.data.attestationOnly,
    remarques: parsed.data.remarques,
  });

  if (!result.ok) {
    const status = result.error.includes("déjà signé") ? 409 : 400;
    return NextResponse.json({ error: result.error }, { status });
  }

  return NextResponse.json({ ok: true, emargement: result.emargement });
}
