import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { messagingErrorResponse, readJsonBody } from "@/app/lib/messaging/http";
import { addGroupMembers } from "@/app/lib/messaging/service";

type Ctx = { params: Promise<{ id: string }> };

export async function POST(req: Request, ctx: Ctx) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const { id } = await ctx.params;
    const raw = await readJsonBody(req);
    const userIds = Array.isArray(raw.userIds)
      ? raw.userIds.filter((u): u is string => typeof u === "string")
      : [];
    const conversation = await addGroupMembers(
      gate.ctx.etablissementId,
      id,
      gate.ctx.userId,
      userIds,
    );
    return NextResponse.json({ conversation });
  } catch (error) {
    return messagingErrorResponse(error, "members POST", "Ajout impossible.");
  }
}
