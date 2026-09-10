import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import {
  listMessages,
  sendMessage,
  MessagingError,
} from "@/app/lib/messaging/service";
import type { MessagingSendAttachmentInput } from "@/app/lib/messaging/types";

type Ctx = { params: Promise<{ id: string }> };

export async function GET(req: Request, ctx: Ctx) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const { id } = await ctx.params;
    const url = new URL(req.url);
    const cursor = url.searchParams.get("cursor") ?? undefined;
    const limitRaw = url.searchParams.get("limit");
    const limit = limitRaw ? Number.parseInt(limitRaw, 10) : 50;
    const result = await listMessages(gate.ctx.etablissementId, id, gate.ctx.userId, {
      cursor,
      limit: Number.isFinite(limit) ? limit : 50,
    });
    return NextResponse.json({
      messages: result.messages,
      nextCursor: result.nextCursor,
    });
  } catch (error) {
    if (error instanceof MessagingError) {
      return NextResponse.json(
        { error: error.message, code: error.code },
        { status: error.status },
      );
    }
    console.error("[messaging/messages GET]", error);
    return NextResponse.json({ error: "Impossible de charger les messages." }, { status: 500 });
  }
}

export async function POST(req: Request, ctx: Ctx) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const { id } = await ctx.params;
    const body = (await req.json()) as {
      body?: string | null;
      type?: string;
      replyToId?: string | null;
      forwardedFromId?: string | null;
      attachments?: MessagingSendAttachmentInput[];
    };
    const message = await sendMessage(gate.ctx.etablissementId, id, gate.ctx.userId, {
      body: body.body ?? null,
      type: body.type,
      replyToId: body.replyToId ?? undefined,
      forwardedFromId: body.forwardedFromId ?? undefined,
      attachments: body.attachments,
    });
    return NextResponse.json({ message });
  } catch (error) {
    if (error instanceof MessagingError) {
      return NextResponse.json(
        { error: error.message, code: error.code },
        { status: error.status },
      );
    }
    console.error("[messaging/messages POST]", error);
    return NextResponse.json({ error: "Impossible d'envoyer le message." }, { status: 500 });
  }
}
