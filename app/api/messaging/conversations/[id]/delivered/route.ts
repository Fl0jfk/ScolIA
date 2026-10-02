import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { markDelivered, MessagingError } from "@/app/lib/messaging/service";

type Ctx = { params: Promise<{ id: string }> };

export async function POST(req: Request, ctx: Ctx) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const { id } = await ctx.params;
    let messageId: string | null = null;
    try {
      const body = (await req.json()) as { messageId?: unknown };
      if (typeof body.messageId === "string" && body.messageId.trim()) {
        messageId = body.messageId.trim();
      }
    } catch {
      /* corps optionnel */
    }
    await markDelivered(gate.ctx.etablissementId, id, gate.ctx.userId, messageId);
    return NextResponse.json({ ok: true });
  } catch (error) {
    if (error instanceof MessagingError) {
      return NextResponse.json(
        { error: error.message, code: error.code },
        { status: error.status },
      );
    }
    console.error("[messaging/delivered]", error);
    return NextResponse.json(
      { error: "Impossible de marquer comme distribué." },
      { status: 500 },
    );
  }
}
