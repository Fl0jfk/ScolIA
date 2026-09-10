import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import {
  editMessage,
  softDeleteMessage,
  MessagingError,
} from "@/app/lib/messaging/service";

type Ctx = { params: Promise<{ id: string }> };

export async function PATCH(req: Request, ctx: Ctx) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const { id } = await ctx.params;
    const body = (await req.json()) as { body?: string };
    if (typeof body.body !== "string") {
      return NextResponse.json({ error: "Contenu requis." }, { status: 400 });
    }
    const message = await editMessage(
      gate.ctx.etablissementId,
      id,
      gate.ctx.userId,
      body.body,
    );
    return NextResponse.json({ message });
  } catch (error) {
    if (error instanceof MessagingError) {
      return NextResponse.json(
        { error: error.message, code: error.code },
        { status: error.status },
      );
    }
    console.error("[messaging/messages PATCH]", error);
    return NextResponse.json({ error: "Modification impossible." }, { status: 500 });
  }
}

export async function DELETE(_req: Request, ctx: Ctx) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const { id } = await ctx.params;
    const message = await softDeleteMessage(
      gate.ctx.etablissementId,
      id,
      gate.ctx.userId,
    );
    return NextResponse.json({ message });
  } catch (error) {
    if (error instanceof MessagingError) {
      return NextResponse.json(
        { error: error.message, code: error.code },
        { status: error.status },
      );
    }
    console.error("[messaging/messages DELETE]", error);
    return NextResponse.json({ error: "Suppression impossible." }, { status: 500 });
  }
}
