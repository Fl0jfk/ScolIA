import { NextResponse } from "next/server";
import { requireFamilleAccess } from "@/app/lib/famille-auth";
import { listCahierTexteForFamille } from "@/app/lib/cahier-texte-db";

/** Cahier de textes visible pour les enfants du foyer (par classe). */
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

  const from = url.searchParams.get("from");
  const to = url.searchParams.get("to");
  const entries = await listCahierTexteForFamille(gate.ctx.etablissementId, enfants, {
    from,
    to,
  });

  return NextResponse.json({
    enfants: gate.ctx.enfants,
    filterEleveId: eleveId || null,
    entries,
  });
}
