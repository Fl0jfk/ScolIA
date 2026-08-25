import { NextResponse } from "next/server";
import { requireFamilleAccess } from "@/app/lib/famille-auth";
import { listFamilleBulletins } from "@/app/lib/famille-bulletins-db";

export async function GET(req: Request) {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;

  const url = new URL(req.url);
  const eleveId = url.searchParams.get("eleveId")?.trim();
  const eleveIds = eleveId
    ? gate.ctx.enfants.filter((e) => e.id === eleveId).map((e) => e.id)
    : gate.ctx.enfants.map((e) => e.id);

  if (eleveId && !eleveIds.length) {
    return NextResponse.json(
      { error: "Accès refusé à cet élève.", code: "FAMILLE_ELEVE_FORBIDDEN" },
      { status: 403 },
    );
  }

  const bulletins = await listFamilleBulletins(gate.ctx.etablissementId, eleveIds);

  return NextResponse.json({
    enfants: gate.ctx.enfants,
    bulletins,
  });
}
