import { NextResponse } from "next/server";
import { requireFamilleAccess } from "@/app/lib/famille-auth";
import { listSanctionsForFamille } from "@/app/lib/vs-sanctions-db";

/** Sanctions actives des enfants du foyer (lecture seule). */
export async function GET(req: Request) {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;

  const url = new URL(req.url);
  const eleveId =
    url.searchParams.get("eleveId")?.trim() || url.searchParams.get("enfant")?.trim();
  const enfants = eleveId
    ? gate.ctx.enfants.filter((e) => e.id === eleveId)
    : gate.ctx.enfants;

  if (eleveId && !enfants.length) {
    return NextResponse.json(
      { error: "Accès refusé à cet élève.", code: "FAMILLE_ELEVE_FORBIDDEN" },
      { status: 403 },
    );
  }

  const sanctions = await listSanctionsForFamille(
    gate.ctx.etablissementId,
    enfants.map((e) => e.id),
  );

  return NextResponse.json({
    enfants: gate.ctx.enfants,
    filterEleveId: eleveId || null,
    sanctions,
  });
}
