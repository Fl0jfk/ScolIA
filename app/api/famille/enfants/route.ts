import { NextResponse } from "next/server";
import { requireFamilleAccess } from "@/app/lib/famille-auth";

/** Liste des enfants rattachés au compte parent (app mobile / portail famille). */
export async function GET() {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;

  return NextResponse.json({
    enfants: gate.ctx.enfants,
  });
}
