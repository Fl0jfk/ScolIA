import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { markRead, MessagingError } from "@/app/lib/messaging/service";

type Ctx = { params: Promise<{ id: string }> };

export async function POST(_req: Request, ctx: Ctx) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const { id } = await ctx.params;
    await markRead(gate.ctx.etablissementId, id, gate.ctx.userId);
    return NextResponse.json({ ok: true });
  } catch (error) {
    if (error instanceof MessagingError) {
      return NextResponse.json(
        { error: error.message, code: error.code },
        { status: error.status },
      );
    }
    console.error("[messaging/read]", error);
    return NextResponse.json({ error: "Impossible de marquer comme lu." }, { status: 500 });
  }
}
