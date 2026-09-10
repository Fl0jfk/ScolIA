import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { publishTyping, MessagingError } from "@/app/lib/messaging/service";

type Ctx = { params: Promise<{ id: string }> };

export async function POST(_req: Request, ctx: Ctx) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const { id } = await ctx.params;
    await publishTyping(gate.ctx.etablissementId, id, gate.ctx.userId);
    return NextResponse.json({ ok: true });
  } catch (error) {
    if (error instanceof MessagingError) {
      return NextResponse.json(
        { error: error.message, code: error.code },
        { status: error.status },
      );
    }
    console.error("[messaging/typing]", error);
    return NextResponse.json({ error: "Impossible de signaler la saisie." }, { status: 500 });
  }
}
