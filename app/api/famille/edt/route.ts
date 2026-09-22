import { NextResponse } from "next/server";
import { requireFamilleAccess } from "@/app/lib/famille-auth";
import { listFamilleEdtByEnfants } from "@/app/lib/famille-edt-db";

/** Emploi du temps des enfants liés — grille `edt_creneau` par classe. */
export async function GET(req: Request) {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;

  const url = new URL(req.url);
  const eleveId = url.searchParams.get("eleveId")?.trim() || url.searchParams.get("enfant")?.trim();
  const enfants = eleveId
    ? gate.ctx.enfants.filter((e) => e.id === eleveId)
    : gate.ctx.enfants;

  if (eleveId && !enfants.length) {
    return NextResponse.json(
      { error: "Accès refusé à cet élève.", code: "FAMILLE_ELEVE_FORBIDDEN" },
      { status: 403 },
    );
  }

  const emplois = await listFamilleEdtByEnfants(
    gate.ctx.etablissementId,
    enfants.map((e) => ({
      id: e.id,
      nom: e.nom,
      prenom: e.prenom,
      classe: e.classe,
    })),
  );

  return NextResponse.json({
    enfants: gate.ctx.enfants,
    emplois,
  });
}
