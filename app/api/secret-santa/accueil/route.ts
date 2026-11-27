import { NextResponse } from "next/server";
import { requireSantaContext, santaErrorResponse } from "@/app/lib/secret-santa-access";
import { listSantaHomeCards } from "@/app/lib/secret-santa-db";

export const dynamic = "force-dynamic";

/** Encadré de la page d'accueil : les seules invitations de la personne connectée. Ne déclenche aucun tirage. */
export async function GET() {
  const auth = await requireSantaContext();
  if (!auth.ok) return auth.response;
  try {
    const cards = await listSantaHomeCards(auth.ctx.etablissementId, auth.ctx.userId, new Date());
    return NextResponse.json({ cards }, { headers: { "Cache-Control": "no-store" } });
  } catch (error) {
    return santaErrorResponse(error);
  }
}
