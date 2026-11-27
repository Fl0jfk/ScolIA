import { NextResponse } from "next/server";
import { requireSantaContext, santaErrorResponse } from "@/app/lib/secret-santa-access";
import { respondToSanta } from "@/app/lib/secret-santa-db";
import { SantaError } from "@/app/lib/secret-santa";

export const dynamic = "force-dynamic";

/** Chacun répond pour soi uniquement : la personne vient de la session, jamais de la requête. */
export async function POST(req: Request, { params }: { params: Promise<{ id: string }> }) {
  const auth = await requireSantaContext();
  if (!auth.ok) return auth.response;
  try {
    const { id } = await params;
    const body = (await req.json().catch(() => null)) as { response?: unknown } | null;
    if (body?.response !== "accepted" && body?.response !== "declined") {
      throw new SantaError("Réponse non reconnue.");
    }
    await respondToSanta(auth.ctx.etablissementId, id, auth.ctx.userId, body.response);
    return NextResponse.json({ success: true });
  } catch (error) {
    return santaErrorResponse(error);
  }
}
