import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { messagingErrorResponse, readJsonBody } from "@/app/lib/messaging/http";
import { MessagingError, setMessagePinned } from "@/app/lib/messaging/service";

type Ctx = { params: Promise<{ id: string }> };

export async function POST(req: Request, ctx: Ctx) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const { id } = await ctx.params;
    const raw = await readJsonBody(req);
    if (typeof raw.pinned !== "boolean") {
      throw new MessagingError("Champ « pinned » requis.", 400, "INVALID_INPUT");
    }
    const message = await setMessagePinned(
      gate.ctx.etablissementId,
      id,
      gate.ctx.userId,
      raw.pinned,
    );
    return NextResponse.json({ message });
  } catch (error) {
    return messagingErrorResponse(error, "pin POST", "Épinglage impossible.");
  }
}
