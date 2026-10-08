import { NextResponse } from "next/server";
import { requireAppUser } from "@/app/lib/app-session";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { canAccessSstRegistre } from "@/app/lib/sst-registre/access";
import {
  SST_EMARGEMENT_TEXTE,
  SST_NOTICE,
  SST_REGLEMENTATION,
  SST_URGENCES,
} from "@/app/lib/sst-registre/content";
import { getSstMyStatus } from "@/app/lib/sst-registre/db";

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
    const status = await getSstMyStatus({
      etablissementId: scope.ctx.etablissementId,
      userId: appUser.user.id,
      roles: appUser.user.roles,
    });
    return NextResponse.json({
      ...status,
      content: {
        reglementation: SST_REGLEMENTATION,
        notice: SST_NOTICE,
        emargementTexte: SST_EMARGEMENT_TEXTE,
        urgences: SST_URGENCES,
      },
    });
  } catch (e) {
    console.error("[rh/sst GET]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur registre SST." },
      { status: 500 },
    );
  }
}
