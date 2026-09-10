import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { setReaction, MessagingError } from "@/app/lib/messaging/service";

type Ctx = { params: Promise<{ id: string }> };

export async function POST(req: Request, ctx: Ctx) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const { id } = await ctx.params;
    const body = (await req.json()) as { emoji?: string };
    const emoji = body.emoji?.trim();
    if (!emoji) {
      return NextResponse.json({ error: "Emoji requis." }, { status: 400 });
    }
    const message = await setReaction(
      gate.ctx.etablissementId,
      id,
      gate.ctx.userId,
      emoji,
    );
    return NextResponse.json({ message });
  } catch (error) {
    if (error instanceof MessagingError) {
      return NextResponse.json(
        { error: error.message, code: error.code },
        { status: error.status },
      );
    }
    console.error("[messaging/reactions]", error);
    return NextResponse.json({ error: "Réaction impossible." }, { status: 500 });
  }
}
