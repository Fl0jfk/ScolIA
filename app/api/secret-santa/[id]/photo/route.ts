import { NextResponse } from "next/server";
import { requireSantaContext, santaErrorResponse } from "@/app/lib/secret-santa-access";
import { readSantaFor } from "@/app/lib/secret-santa-db";
import { buildSantaView, receiverOf } from "@/app/lib/secret-santa";
import { avatarResponse, readAvatar } from "@/app/lib/avatar-db";

export const dynamic = "force-dynamic";

/**
 * Photo de la personne à qui l'on offre. L'adresse ne contient pas son identifiant : les journaux
 * du serveur ne disent pas qui a tiré qui.
 */
export async function GET(_req: Request, { params }: { params: Promise<{ id: string }> }) {
  const auth = await requireSantaContext();
  if (!auth.ok) return auth.response;
  try {
    const { id } = await params;
    const now = new Date();
    const { santa, participants } = await readSantaFor(auth.ctx.etablissementId, id, auth.ctx.userId, now);
    const receiver = buildSantaView(santa, participants, auth.ctx.userId, now).mine
      ? receiverOf(participants, auth.ctx.userId)
      : null;
    const avatar = receiver ? await readAvatar(receiver.userId) : null;
    if (!avatar) return NextResponse.json({ error: "Pas de photo." }, { status: 404 });
    const response = avatarResponse(avatar.image);
    response.headers.set("Cache-Control", "no-store");
    return response;
  } catch (error) {
    return santaErrorResponse(error);
  }
}
