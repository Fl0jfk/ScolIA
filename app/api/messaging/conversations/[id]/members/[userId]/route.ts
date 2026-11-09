import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { messagingErrorResponse } from "@/app/lib/messaging/http";
import { removeGroupMember } from "@/app/lib/messaging/service";

type Ctx = { params: Promise<{ id: string; userId: string }> };

/** Soi-même = quitter le groupe ; un autre membre = retrait par le créateur. */
export async function DELETE(_req: Request, ctx: Ctx) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const { id, userId: rawUserId } = await ctx.params;
    let userId: string;
    try {
      userId = decodeURIComponent(rawUserId);
    } catch {
      return NextResponse.json({ error: "Identifiant invalide." }, { status: 400 });
    }
    const conversation = await removeGroupMember(
      gate.ctx.etablissementId,
      id,
      gate.ctx.userId,
      userId,
    );
    return NextResponse.json({ ok: true, conversation });
  } catch (error) {
    return messagingErrorResponse(error, "members DELETE", "Opération impossible.");
  }
}
