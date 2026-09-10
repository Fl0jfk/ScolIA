import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { searchMessages, MessagingError } from "@/app/lib/messaging/service";

export async function GET(req: Request) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const url = new URL(req.url);
    const q = url.searchParams.get("q")?.trim() ?? "";
    if (q.length < 2) {
      return NextResponse.json({ results: [] });
    }
    const messages = await searchMessages(gate.ctx.etablissementId, gate.ctx.userId, q);
    const results = messages.map((m) => ({
      messageId: m.id,
      conversationId: m.conversationId,
      body: m.body,
      createdAt: m.createdAt,
    }));
    return NextResponse.json({ results });
  } catch (error) {
    if (error instanceof MessagingError) {
      return NextResponse.json(
        { error: error.message, code: error.code },
        { status: error.status },
      );
    }
    console.error("[messaging/search]", error);
    return NextResponse.json({ error: "Recherche impossible." }, { status: 500 });
  }
}
