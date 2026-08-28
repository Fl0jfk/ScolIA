import { NextResponse } from "next/server";
import { requireFamilleAccess } from "@/app/lib/famille-auth";

/** Contexte portail famille : enfants, foyers, année courante. */
export async function GET() {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;

  return NextResponse.json({
    enfants: gate.ctx.enfants,
    foyers: gate.ctx.foyers,
    anneeCouranteLabel: gate.ctx.anneeCouranteLabel,
    email: gate.ctx.email,
  });
}
