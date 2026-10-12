import { NextResponse } from "next/server";
import { requireAppUser } from "@/app/lib/app-session";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { canManageSstRegistre } from "@/app/lib/sst-registre/access";
import { getSstSuivi } from "@/app/lib/sst-registre/db";

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
    const suivi = await getSstSuivi({ etablissementId: scope.ctx.etablissementId });
    return NextResponse.json(suivi);
  } catch (e) {
    console.error("[rh/sst/suivi GET]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur suivi SST." },
      { status: 500 },
    );
  }
}
