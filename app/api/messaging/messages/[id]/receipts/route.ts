import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { messagingErrorResponse } from "@/app/lib/messaging/http";
import { getMessageReceipts } from "@/app/lib/messaging/service";

type Ctx = { params: Promise<{ id: string }> };

/** « Vu par » — réservé à l’auteur du message. */
export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const { id } = await ctx.params;
    const receipts = await getMessageReceipts(
      gate.ctx.etablissementId,
      id,
      gate.ctx.userId,
    );
    return NextResponse.json(receipts);
  } catch (error) {
    return messagingErrorResponse(error, "receipts GET", "Impossible de charger les accusés.");
  }
}
