import { NextResponse } from "next/server";
import { requireEleveAccess } from "@/app/lib/eleve-auth";

/** Profil élève pour l’app (contexte /api/eleve uniquement). */
export async function GET() {
  const gate = await requireEleveAccess();
  if (!gate.ok) return gate.response;

  return NextResponse.json({
    channel: "eleve",
    eleve: gate.ctx.eleve,
    etablissementId: gate.ctx.etablissementId,
  });
}
